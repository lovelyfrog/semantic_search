use ndarray::{Array2, ArrayViewD, Axis, Ix2, Ix3};
use ort::session::builder::GraphOptimizationLevel;
use ort::{session::Session, value::Tensor};
use tokenizers::{EncodeInput, Encoding, Tokenizer};

use crate::embedding::utils::{Embedder, EmbeddingModelType, EmbeddingOptions, OnnxRuntimeConfig};

pub struct OnnxEmbedder {
    dim: usize,
    model_type: EmbeddingModelType,
    session: Session,
    tokenizer: Tokenizer,
}

impl OnnxEmbedder {
    pub fn load_runtime(runtime_path: &str) -> anyhow::Result<()> {
        log::info!("Loading ONNX runtime from {}", runtime_path);
        ort::init_from(runtime_path).commit()?;
        Ok(())
    }

    pub fn new(
        config: OnnxRuntimeConfig,
        embedding_options: EmbeddingOptions,
    ) -> anyhow::Result<Self> {
        // Level2/Level3 can trigger graph fusions (e.g. SimplifiedLayerNormFusion) that fail on some FP16 exports.
        let path = embedding_options.model_path.as_path();
        let should_fallback_to_level1 = matches!(
            &config.optimization_level,
            GraphOptimizationLevel::Level2 | GraphOptimizationLevel::Level3
        );

        let session = match Session::builder()?
            .with_intra_threads(config.intra_threads)?
            .with_optimization_level(config.optimization_level)?
            .commit_from_file(path)
        {
            Ok(s) => s,
            Err(e) if should_fallback_to_level1 => {
                log::warn!(
                    "ONNX graph optimization failed; retrying with Level1: {}",
                    e
                );
                Session::builder()?
                    .with_intra_threads(config.intra_threads)?
                    .with_optimization_level(GraphOptimizationLevel::Level1)?
                    .commit_from_file(path)
                    .map_err(|e2| {
                        anyhow::anyhow!(
                            "failed to load ONNX model (Level1 also failed: {e2}; original: {e})"
                        )
                    })?
            }
            Err(e) => return Err(anyhow::anyhow!("failed to load ONNX model: {e}")),
        };
        let tokenizer = Tokenizer::from_file(embedding_options.tokenizer_path)
            .map_err(|e| anyhow::anyhow!("failed to load tokenizer: {}", e))?;

        log::info!("OnnxEmbedder initialized successfully");
        Ok(Self {
            dim: embedding_options.dim,
            model_type: embedding_options.model_type,
            session,
            tokenizer,
        })
    }

    fn batch_encode(&self, texts: &[String]) -> anyhow::Result<Vec<Encoding>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let encode_inputs: Vec<EncodeInput> = texts
            .iter()
            .map(|s| EncodeInput::Single(s.clone().into()))
            .collect();

        let encodings = self
            .tokenizer
            .encode_batch(encode_inputs, true)
            .map_err(|e| anyhow::anyhow!("failed to encode batch: {}", e))?;
        Ok(encodings)
    }
}

impl Embedder for OnnxEmbedder {
    fn encode(&self, text: &str) -> anyhow::Result<Encoding> {
        self.batch_encode(&[text.to_string()])?
            .first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("failed to encode text"))
    }

    fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let result = self.batch_embed(&[text.to_string()])?;
        Ok(result.first().cloned().unwrap_or_default())
    }

    fn batch_embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        let encodings = self.batch_encode(texts)?;
        let batch_size = encodings.len();
        let max_len = encodings.iter().map(|e| e.len()).max().unwrap_or(0);

        if max_len == 0 {
            return Ok(vec![vec![0.0; self.dim]; batch_size]);
        }

        let need_token_type_ids = self
            .session
            .inputs
            .iter()
            .any(|i| i.name == "token_type_ids");

        let mut input_ids = Array2::<i64>::zeros((batch_size, max_len));
        let mut attention_mask = Array2::<i64>::zeros((batch_size, max_len));
        let mut token_type_ids = if need_token_type_ids {
            Some(Array2::<i64>::zeros((batch_size, max_len)))
        } else {
            None
        };

        for (i, encoding) in encodings.iter().enumerate() {
            let ids = encoding.get_ids();
            let mask = encoding.get_attention_mask();
            let types = encoding.get_type_ids();
            let seq_len = ids.len().min(max_len);

            for j in 0..seq_len {
                input_ids[(i, j)] = ids[j] as i64;
                attention_mask[(i, j)] = mask[j] as i64;
                if let Some(ref mut tti) = token_type_ids {
                    let v = types.get(j).copied().unwrap_or(0);
                    tti[[i, j]] = v as i64;
                }
            }
        }

        let (b, l) = input_ids.dim();
        let input_ids_vec: Vec<i64> = input_ids.into_iter().collect();
        let input_ids_tensor = Tensor::<i64>::from_array(([b as i64, l as i64], input_ids_vec))?;

        let (b2, l2) = attention_mask.dim();
        let attention_mask_vec: Vec<i64> = attention_mask.into_iter().collect();
        let attention_mask_tensor =
            Tensor::<i64>::from_array(([b2 as i64, l2 as i64], attention_mask_vec))?;

        let outputs = if let Some(tti) = token_type_ids {
            let (b3, l3) = tti.dim();
            let tti_vec: Vec<i64> = tti.into_iter().collect();
            let tti_tensor = Tensor::<i64>::from_array(([b3 as i64, l3 as i64], tti_vec))?;

            let inputs = ort::inputs![
                "input_ids" => input_ids_tensor,
                "attention_mask" => attention_mask_tensor,
                "token_type_ids" => tti_tensor,
            ]?;
            self.session.run(inputs)?
        } else {
            let inputs = ort::inputs![
                "input_ids" => input_ids_tensor,
                "attention_mask" => attention_mask_tensor,
            ]?;
            self.session.run(inputs)?
        };

        let output_tensor = outputs[0].try_extract_tensor::<f32>()?;
        let embeddings_view: ArrayViewD<'_, f32> = output_tensor.view().into_dyn();
        let embddings_2d: Array2<f32> = match embeddings_view.ndim() {
            2 => embeddings_view
                .into_dimensionality::<Ix2>()
                .map_err(|e| anyhow::anyhow!("failed to convert embeddings to 2D: {}", e))?
                .to_owned(),
            3 => {
                let view3 = embeddings_view
                    .into_dimensionality::<Ix3>()
                    .map_err(|e| anyhow::anyhow!("failed to convert embeddings to 3D: {}", e))?;

                let s = view3.len_of(Axis(1)) as f32;
                view3.sum_axis(Axis(1)) / s
            }
            _other => {
                return Err(anyhow::anyhow!(
                    "unexpected number of dimensions: {}",
                    embeddings_view.ndim()
                ));
            }
        };

        let (o, l) = embddings_2d.dim();
        if o != batch_size {
            return Err(anyhow::anyhow!(
                "unexpected number of embeddings: {} != {}",
                o,
                batch_size
            ));
        }

        if l != self.dim {
            return Err(anyhow::anyhow!(
                "unexpected number of dimensions: {} != {}",
                l,
                self.dim
            ));
        }

        let mut embeddings = Vec::with_capacity(batch_size);
        for row in embddings_2d.outer_iter() {
            embeddings.push(row.to_vec());
        }
        Ok(embeddings)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::utils::{load_onnx_runtime, setup_embedder};

    use super::*;

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        debug_assert_eq!(a.len(), b.len());
        let mut dot = 0.0f32;
        let mut na = 0.0f32;
        let mut nb = 0.0f32;
        for i in 0..a.len() {
            dot += a[i] * b[i];
            na += a[i] * a[i];
            nb += b[i] * b[i];
        }
        dot / ((na.sqrt() * nb.sqrt()).max(1e-12))
    }

    #[test]
    fn test_batch_embed() {
        load_onnx_runtime().unwrap();
        let embedder = setup_embedder(4, 32, 1).unwrap();
        let inputs = vec![
            "hello world".to_string(),
            "rust onnx embedder test".to_string(),
        ];

        let embeddings = embedder.batch_embed(&inputs).unwrap();
        assert_eq!(embeddings.len(), inputs.len());
        assert!(!embeddings[0].is_empty());
    }

    /// 用“相对关系”验证 embedding 的语义性质：
    /// 相似句子的 cosine similarity 应高于不相似句子。
    ///
    /// 说明：不同模型/量化/池化策略会影响绝对阈值，因此只比较相对大小并留出 margin。
    #[test]
    fn test_embedding_similarity_ordering() {
        load_onnx_runtime().unwrap();
        let embedder = setup_embedder(4, 32, 1).unwrap();

        let s1 = "How do I bake a cake?";
        let s2 = "How to bake a cake";
        let s3 = "你好，我是张三";

        let embeddings = embedder
            .batch_embed(&[s1.to_string(), s2.to_string(), s3.to_string()])
            .expect("embedding failed");

        assert_eq!(embeddings.len(), 3);
        assert_eq!(embeddings[0].len(), embeddings[1].len());
        assert_eq!(embeddings[0].len(), embeddings[2].len());
        assert!(!embeddings[0].is_empty());

        let sim_12 = cosine_similarity(&embeddings[0], &embeddings[1]);
        let sim_13 = cosine_similarity(&embeddings[0], &embeddings[2]);

        println!("sim_12: {sim_12}, sim_13: {sim_13}");

        assert!(sim_12.is_finite(), "sim_12 is not finite: {sim_12}");
        assert!(sim_13.is_finite(), "sim_13 is not finite: {sim_13}");

        // 核心断言：相似句 > 不相似句（留一定 margin，避免模型波动导致偶发失败）
        assert!(
            sim_12 > sim_13 + 0.05,
            "expected sim(s1,s2) > sim(s1,s3) + margin, got sim_12={sim_12}, sim_13={sim_13}"
        );
    }
}
