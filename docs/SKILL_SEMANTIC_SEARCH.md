## 目标

把本项目的语义索引/搜索能力，通过 MCP（stdio）包装成一个**可复用的“skill”**，让其他 agent（OpenCode / Cursor / Claude Code 等）在合适时机自动调用。

skill 需要支持的用户指令（slash commands）：

- `/start_index`：对“当前工程”启动索引；可选参数为工程路径（见“多工程”）
- `/index_progress`：查看索引进度/状态；可选工程路径（见“多工程”）
- `/stop_index`：停止/取消当前索引；可选工程路径（见“多工程”）

并且需要指导 agent：

- 什么时候应该用语义搜索（MCP `search` 工具）来辅助回答问题

---

## MCP 工具契约（server 暴露的 tools）

本仓库的 MCP server（`semantic-search-mcp`）通过 stdio 暴露以下 tools（JSON 入参/JSON 文本出参）：

- **`start_index`**：后台启动索引（非阻塞）
- **`index_progress`**：查询索引状态与进度
- **`stop_index`**：取消索引
- **`index`**：同步索引（阻塞直到完成；兼容旧用法）
- **`search`**：语义搜索

### 约定：工程（project）从 MCP server 配置而来

`semantic-search-mcp` 在启动时通过环境变量/CLI 参数确定 `SEMANTIC_SEARCH_PROJECT`（工程根目录），随后 tools 默认都针对该工程执行。

> 说明：当前实现中，`start_index/index_progress/stop_index` 的状态是**server 进程内**维护的，因此“一个 server 实例对应一个工程”最简单、最稳妥。

---

## OpenCode 配置 MCP（stdio）

OpenCode 的 MCP 配置中 `command` 是数组（第一个元素为可执行文件路径）。

下面是推荐配置（把路径替换成你的 dist 产物目录）：

```json
{
  "mcpServers": {
    "semantic-search": {
      "command": ["/ABS/PATH/dist/semantic-search-mcp"],
      "env": {
        "SEMANTIC_SEARCH_PROJECT": "/ABS/PATH/to/your/project",
        "SEMANTIC_SEARCH_RESOURCES_DIR": "/ABS/PATH/dist/resources",
        "SEMANTIC_SEARCH_MODEL_TYPE": "veso",
        "SEMANTIC_SEARCH_OUTPUT": "json"
      }
    }
  }
}
```

可选（显式指定数据文件，避免落到默认 DATA_DIR）：

- `SEMANTIC_SEARCH_INDEX_DB`
- `SEMANTIC_SEARCH_VECTOR_DB`
- `SEMANTIC_SEARCH_LOG_PATH`

---

## Skill 行为设计（给 agent 的“操作手册”）

这一节给“上层 agent”一个可直接照抄的策略：如何把用户 slash command 翻译成 MCP tool 调用。

### /start_index

**触发条件**：
- 用户显式输入 `/start_index`

**执行**：
- 调用 MCP tool：`start_index`
- 参数：
  - `layer`：缺省用 `all`（file+symbol）

**返回**：
- 将 `StartIndexResponse.status` 与 `layers` 反馈给用户
- 提示用户可用 `/index_progress` 轮询

### /index_progress

**触发条件**：
- 用户显式输入 `/index_progress`

**执行**：
- 调用 MCP tool：`index_progress`

**返回**：
- 展示：
  - `status`（running/completed/cancelled/error/not_started）
  - `progress.handled_* / progress.total_*`
  - 若 `last_error` 不为空，提示用户可以重试 `/start_index`

### /stop_index

**触发条件**：
- 用户显式输入 `/stop_index`

**执行**：
- 调用 MCP tool：`stop_index`

**返回**：
- 展示 `status`（一般为 cancelled）
- 告知用户可重新 `/start_index`

---

## 什么时候该使用语义搜索（给 agent 的决策规则）

当用户问题满足以下任一条件时，agent 应优先调用 MCP `search`：

- **定位类**：问“在哪里定义/实现/调用”  
  例：某函数/trait/struct 在哪；某配置在哪生效；某报错从哪抛出
- **关系类**：问“谁调用了谁 / 谁依赖了谁 / 哪些地方会触发”  
  例：某事件处理链路；某字段写入点
- **意图类（非精确字符串）**：用户描述功能但不知道关键字  
  例：“权限校验在哪里做的”“索引进度怎么更新的”
- **跨文件/跨模块**：需要从多个层（symbol/file/content/all）合并理解

不适合用语义搜索的情况：

- 用户只问“某字符串是否存在/在哪里出现”：更适合关键词搜索（如 ripgrep）
- 纯概念/纯解释且不依赖代码细节的问题

### search 的参数建议

- 默认层：`symbol`（定位定义/调用最稳）
- 若问“读我项目里的文档/README/注释”：用 `content`
- 若想“尽量全”：用 `all`（会合并 file+symbol，返回结果按 score 排序截断）

---

## 多工程（`/start_index + 路径`）如何支持

强烈建议的架构是：

- **一个工程启动一个 MCP server 实例**（每个实例有自己的 `SEMANTIC_SEARCH_PROJECT`）

理由：
- 索引状态（running/progress/cancel）是进程内状态，天然按实例隔离
- storage（index.db / vectordb）可共享也可隔离；共享时注意并发写入风险

如果一定要在“一个 server 实例”里支持任意 `project` 参数，需要引入：
- project -> manager 的缓存（HashMap）
- project -> 索引 session 的隔离（每个 project 一套 progress/cancel）
- storage 并发写入/锁的处理

这属于下一阶段增强项。

---

## 资源文件（onnxruntime/model/tokenizer）放置与打包

推荐：
- resources 跟随 `semantic-search-mcp` 一起打包（例如 `dist/resources`）
- OpenCode 只负责启动 server，并通过 `SEMANTIC_SEARCH_RESOURCES_DIR` 告知资源目录

打包建议：
- `cargo build --release` 后，把：
  - `semantic-search-mcp`（可执行文件）
  - `resources/`（包含 onnxruntime / embedding / tokenizer）
  复制到 `dist/` 并压缩

---

## 运行与稳定性注意事项（必须读）

- **索引是耗时任务**：`start_index` 设计为非阻塞，agent 应引导用户用 `/index_progress` 轮询
- **搜索可在索引中进行**：但结果可能不完整（partial）
- **取消是 best-effort**：`stop_index` 会触发 cancel token，具体停止点取决于 worker 检查频率
- **并行/多进程**：
  - 若多个 server 实例共享同一 `index.db/vectordb`，需要评估并发写入风险（建议每工程独立数据目录）

