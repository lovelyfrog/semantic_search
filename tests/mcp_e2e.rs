use std::{collections::HashMap, fs, path::PathBuf};

use rmcp::{
    model::{CallToolRequestParams, JsonObject},
    service::QuitReason,
    transport::TokioChildProcess,
    serve_client,
};
use vnext_semantic_search::{
    embedding::utils::EmbeddingModelType,
    resources::paths as resource_paths,
};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "{}-{}-{}",
        prefix,
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    dir
}

fn json_object(pairs: &[(&str, serde_json::Value)]) -> JsonObject {
    let mut obj = JsonObject::new();
    for (k, v) in pairs {
        obj.insert((*k).to_string(), v.clone());
    }
    obj
}

fn extract_first_text_json(result: &rmcp::model::CallToolResult) -> Option<&str> {
    result
        .content
        .first()
        .and_then(|c| match &c.raw {
            rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
}

#[tokio::test]
async fn mcp_stdio_e2e_index_and_search() -> anyhow::Result<()> {
    // If model/runtime assets are not present, skip this E2E test gracefully.
    if resource_paths::default_onnxruntime_path().is_err()
        || resource_paths::default_embedding_model_path(EmbeddingModelType::Veso).is_err()
        || resource_paths::default_tokenizer_path(EmbeddingModelType::Veso).is_err()
    {
        eprintln!("skipping mcp e2e: missing embedded/legacy onnxruntime/model/tokenizer assets");
        return Ok(());
    }

    let project_dir = unique_temp_dir("semantic-search-mcp-project");
    fs::create_dir_all(&project_dir)?;
    fs::write(
        project_dir.join("main.rs"),
        "fn main() { println!(\"hello\"); }\n",
    )?;

    let data_dir = unique_temp_dir("semantic-search-mcp-data");
    fs::create_dir_all(&data_dir)?;

    let bin = env!("CARGO_BIN_EXE_semantic-search-mcp");

    let mut cmd = tokio::process::Command::new(bin);
    // Use explicit per-test storage paths for determinism/isolation.
    let envs: HashMap<&str, String> = HashMap::from([
        ("SEMANTIC_SEARCH_PROJECT", project_dir.to_string_lossy().to_string()),
    ]);
    cmd.envs(envs);

    let transport = TokioChildProcess::new(cmd)?;

    // `()` is a no-op client handler, sufficient for tool calls.
    let mut client = serve_client((), transport).await?;

    let tools = client.list_all_tools().await?;
    let tool_names = tools.iter().map(|t| t.name.as_ref()).collect::<Vec<_>>();
    assert!(tool_names.contains(&"index"), "missing tool: index");
    assert!(tool_names.contains(&"search"), "missing tool: search");

    let index_result = client
        .call_tool(
            CallToolRequestParams::new("index").with_arguments(json_object(&[(
                "layer",
                serde_json::json!("all"),
            )])),
        )
        .await?;

    let index_json = extract_first_text_json(&index_result)
        .ok_or_else(|| anyhow::anyhow!("index result missing text content"))?;
    let index_value: serde_json::Value = serde_json::from_str(index_json)?;
    assert!(index_value.get("layers").is_some(), "index json: {index_json}");

    let search_result = client
        .call_tool(
            CallToolRequestParams::new("search").with_arguments(json_object(&[
                ("query", serde_json::json!("hello")),
                ("layer", serde_json::json!("symbol")),
                ("limit", serde_json::json!(3)),
                ("threshold", serde_json::json!(0.0)),
            ])),
        )
        .await?;

    let search_json = extract_first_text_json(&search_result)
        .ok_or_else(|| anyhow::anyhow!("search result missing text content"))?;
    let search_value: serde_json::Value = serde_json::from_str(search_json)?;
    assert!(search_value.get("query").is_some(), "search json: {search_json}");

    // Graceful shutdown.
    let _reason: QuitReason = client.close().await?;

    Ok(())
}

