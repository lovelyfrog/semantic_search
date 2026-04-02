# 语义索引集成说明

本仓库现在提供一个 **CLI-first** 的对外入口，可供 Cursor、OpenCode、OpenClaw、ClaudeCode 等产品通过命令调用。

同时也提供一个 **原生 Rust MCP server**，可供支持 MCP 的产品直接接入。

## 产品内 slash command 映射

推荐把产品内命令统一映射为：

- `/index` -> `semantic-search index`
- `/search <query>` -> `semantic-search search --query "<query>"`

建议所有共享配置优先通过环境变量注入，这样产品里的 slash command 只需要传很少的参数，甚至只需要 query。

推荐最小环境变量集合：

```bash
export SEMANTIC_SEARCH_PROJECT=/workspace/repo
export SEMANTIC_SEARCH_OUTPUT=json
```

说明：
- `SEMANTIC_SEARCH_INDEX_DB` / `SEMANTIC_SEARCH_VECTOR_DB` / `SEMANTIC_SEARCH_LOG_PATH` 是可选项；当不设置时，使用平台默认数据目录 `${DATA_DIR}/semantic_search/`：
  - `index_db_path`: `${DATA_DIR}/semantic_search/index.db`
  - `vector_db_path`: `${DATA_DIR}/semantic_search/vectordb`
  - `log_path`: `${DATA_DIR}/semantic_search/running.log`
  - macOS: `~/Library/Application Support`
  - Windows: `%APPDATA%`
  - Linux: 暂不考虑
- `SEMANTIC_SEARCH_ONNX_RUNTIME` / `SEMANTIC_SEARCH_MODEL` / `SEMANTIC_SEARCH_TOKENIZER` 是可选项；当不设置时，会使用仓库内内置 `resources/` 默认资源（按当前平台与 `SEMANTIC_SEARCH_MODEL_TYPE` 选择）。
- 如需改用非仓库内资源目录，请设置 `SEMANTIC_SEARCH_RESOURCES_DIR`。

配置完成后，产品侧最小映射可以直接是：

```bash
/index  => semantic-search index --layer all
/search => semantic-search search --layer symbol --limit 10 --threshold 0.5 --query "<user_query>"
```

## MCP 接入

如果产品支持 MCP，推荐直接接入 `semantic-search-mcp`。

当前提供两个 MCP tools：

- `index`
- `search`

推荐传输方式：

- `stdio`

启动示例：

```bash
semantic-search-mcp \
  --project /workspace/repo \
  --index-db-path /workspace/repo/.semantic/index.db \
  --vector-db-path /workspace/repo/.semantic/lancedb \
  --onnx-runtime-path /models/onnxruntime.dylib \
  --model-path /models/model.onnx \
  --tokenizer-path /models/tokenizer.json
```

如果共享配置已经通过环境变量注入，则可以直接启动：

```bash
semantic-search-mcp
```

tool 语义：

- `index`
  - 参数：`layer`
  - 默认：`all`
- `search`
  - 参数：`query`、`layer`、`limit`、`threshold`、`paths`
  - 默认：`layer = symbol`、`limit = 10`、`threshold = 0.5`

## 核心约定

- 首次进入仓库时，先执行 `index`
- 索引完成后，再执行 `search`
- 也支持在索引进行中执行 `search`，但只能返回当前已经写入索引的数据

建议在外部产品中把以下命令映射为 slash command：

- `/index` -> `semantic-search index --layer all`
- `/search` -> `semantic-search search --layer symbol --limit 10 --threshold 0.5 --query "<user_query>"`

## CLI 命令

二进制入口：

```bash
cargo run -- index ...
cargo run -- search ...
```

如果已经安装成独立可执行文件，也可以直接调用：

```bash
semantic-search index ...
semantic-search search ...
```

## 必要参数

两个命令共用以下核心参数：

- `--project`：仓库根目录
- `--index-db-path`：SQLite 元数据路径
- `--vector-db-path`：LanceDB 路径
- `--onnx-runtime-path`：ONNX Runtime 动态库路径（可选；不传则使用内置 `resources/`）
- `--model-path`：embedding 模型路径（可选；不传则使用内置 `resources/`）
- `--tokenizer-path`：tokenizer 路径（可选；不传则使用内置 `resources/`）

这些参数也可以通过环境变量提供：

- `SEMANTIC_SEARCH_PROJECT`
- `SEMANTIC_SEARCH_INDEX_DB`
- `SEMANTIC_SEARCH_VECTOR_DB`
- `SEMANTIC_SEARCH_ONNX_RUNTIME`（可选；用于覆盖内置 `resources/`）
- `SEMANTIC_SEARCH_MODEL`（可选；用于覆盖内置 `resources/`）
- `SEMANTIC_SEARCH_TOKENIZER`（可选；用于覆盖内置 `resources/`）
- `SEMANTIC_SEARCH_MODEL_TYPE`
- `SEMANTIC_SEARCH_RESOURCES_DIR`（可选；用于覆盖内置 `resources/` 根目录）
- `SEMANTIC_SEARCH_EMBEDDING_DIM`
- `SEMANTIC_SEARCH_BATCH_SIZE`
- `SEMANTIC_SEARCH_THREADS`
- `SEMANTIC_SEARCH_INTRA_THREADS`
- `SEMANTIC_SEARCH_OUTPUT`

## `/index`

示例：

```bash
semantic-search index \
  --project /workspace/repo \
  --index-db-path /workspace/repo/.semantic/index.db \
  --vector-db-path /workspace/repo/.semantic/lancedb \
  --onnx-runtime-path /models/onnxruntime.dylib \
  --model-path /models/model.onnx \
  --tokenizer-path /models/tokenizer.json \
  --layer all
```

可选参数：

- `--layer file|symbol|all`
- `--output text|json`
- `--log-path`
- `--log-level`

行为说明：

- `file`：建立文件级索引
- `symbol`：建立 symbol 级索引
- `all`：依次建立 `file` 和 `symbol`

## `/search`

示例：

```bash
semantic-search search \
  --project /workspace/repo \
  --index-db-path /workspace/repo/.semantic/index.db \
  --vector-db-path /workspace/repo/.semantic/lancedb \
  --onnx-runtime-path /models/onnxruntime.dylib \
  --model-path /models/model.onnx \
  --tokenizer-path /models/tokenizer.json \
  --query "where is audio player state handled" \
  --layer symbol \
  --limit 10 \
  --threshold 0.5
```

可选参数：

- `--layer file|symbol|content`
- `--limit`
- `--threshold`
- `--path <relative-path>`，可重复传入
- `--output text|json`

## 在回答中如何使用 `search`

建议外部产品中的 agent / assistant 遵循以下约定：

1. 当用户第一次进入仓库、索引目录不存在、或仓库刚发生大规模变化时，优先提示并执行 `/index`。
2. 当用户提问是“某功能在哪里”“某逻辑如何实现”“某符号被谁调用”“某文件里有什么”这类定位与解释问题时，优先执行 `/search`。
3. 当索引尚未完成时，也可以继续执行 `/search`，但应在回答里说明“结果基于当前已索引内容，可能不完整”。
4. 当 `/search` 返回命中后，再基于命中文件与片段继续做更细的阅读和回答，而不是盲目全仓扫描。

推荐给 agent 的简化提示词：

```text
如果用户在问代码位置、实现方式、调用关系、相关文件，优先调用 /search。
如果当前仓库尚未索引，先提示用户执行 /index，或在允许时自动执行 /index。
如果索引正在进行中，允许继续调用 /search，并明确告知结果可能是部分结果。
```

## 给外部产品的提示文案建议

可直接告诉用户：

- 首次使用请先执行 `/index`
- 如果仓库代码发生明显变化，请重新执行 `/index`
- 日常查找代码时直接执行 `/search <query>`
- 如果索引还在进行中，`/search` 会返回当前已索引的部分结果

## 接入建议

### Cursor / ClaudeCode / OpenCode / OpenClaw

推荐先使用 shell tool 或 command runner 集成：

1. 为工作区配置一组固定路径或环境变量
2. 把 `/index` 映射为 `semantic-search index --layer all`
3. 把 `/search` 映射为 `semantic-search search --layer symbol --limit 10 --threshold 0.5 --query "<user_query>"`
4. 优先使用 `--output json`，便于产品侧解析结果并渲染

后续如果需要统一协议层，可以在当前 CLI 之上继续封装 MCP 或自定义 tool server，而不需要改动索引与检索核心逻辑。

### Cursor 示例

如果产品支持自定义命令别名，可以配置为：

```text
/index
semantic-search index --layer all
```

```text
/search $ARGUMENTS
semantic-search search --layer symbol --limit 10 --threshold 0.5 --query "$ARGUMENTS"
```

### OpenCode 示例

如果产品支持 shell command / workflow command，可复用相同模板：

```text
/index
semantic-search index --layer all --output json
```

```text
/search $ARGUMENTS
semantic-search search --layer symbol --limit 10 --threshold 0.5 --output json --query "$ARGUMENTS"
```

### ClaudeCode / OpenClaw 通用模板

若暂时不支持原生命令别名，也可以在系统提示或工具定义里约定：

- 当用户输入 `/index` 时，执行 `semantic-search index --layer all --output json`
- 当用户输入 `/search <query>` 时，执行 `semantic-search search --layer symbol --limit 10 --threshold 0.5 --output json --query "<query>"`

## 场景示例

### 首次进入仓库

用户：

```text
/index
```

产品执行：

```bash
semantic-search index --layer all --output json
```

### 用户询问某功能在哪里

用户：

```text
音频播放状态在哪里处理？
```

建议 agent 先执行：

```bash
semantic-search search --layer symbol --limit 10 --threshold 0.5 --output json --query "音频播放状态在哪里处理"
```

然后基于返回的命中文件、symbol、range 和内容片段继续回答。

### 用户显式使用 `/search`

用户：

```text
/search audio player state
```

产品执行：

```bash
semantic-search search --layer symbol --limit 10 --threshold 0.5 --output json --query "audio player state"
```
