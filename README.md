# Chat2Responses

Chat2Responses 是一个自托管的 **Responses API -> Chat Completions API** 转换工具，让 Codex 这类使用 Responses API 的客户端，也能调用只提供 Chat Completions 兼容接口的上游模型。

反过来说，Chat2Responses 可以将 OpenAI-compatible(/chat/completions接口)，转换为 Responses API 兼容的接口。

它适合这些场景：

- 如果你通过 vllm 或者 sglang 本地部署了大模型，但是这两个引擎对 Responses API 不兼容，或兼容不完整，Chat2Responses 可以帮助解决。
- 给 Codex CLI 接入 DeepSeek、Kimi、OpenRouter 或其他 OpenAI-compatible Chat Completions provider。
- 在本机、内网或服务器上统一管理上游 provider、模型映射和调用密钥。
- 对外只分发 Chat2Responses 生成的 service API key，而不是直接暴露 provider API key。
- 为同一个上游模型发布更适合客户端使用的公开模型名。


## 功能概览

- `POST /v1/responses`：接收 Responses API 请求，转换为上游 Chat Completions 请求。
- `GET /v1/models`：按 service API key 返回可用模型。
- 管理台：初始化管理员、配置上游、同步模型、选择启用模型、创建公开模型映射、分发 service API key、查看请求日志、生成 Codex `model-catalog.json`。
- 模型路由：公开模型名可以映射到指定上游和上游模型；service API key 可以限制可用模型。
- 安全存储：provider API key 加密保存在 SQLite；service API key 以哈希存储，并可在管理台按需展示。

## 快速开始：Docker

使用 Docker Compose 启动：

```bash
cp .env.example .env
docker compose up -d
```

首次部署前请编辑 `.env`，把 `CHAT2RESPONSES_SECRET` 改成足够长的随机字符串。

查看日志：

```bash
docker compose logs -f
```

也可以直接使用已发布镜像启动：

```bash
docker run -d \
  --name chat2responses \
  -p 4444:4444 \
  -e CHAT2RESPONSES_SECRET="change-this-long-random-secret" \
  -v chat2responses-data:/app/data \
  lucentttt/chat2responses:latest
```

也可以先在本地构建镜像再启动：

```bash
docker build -t chat2responses:local .
```

```bash
docker run -d \
  --name chat2responses \
  -p 4444:4444 \
  -e CHAT2RESPONSES_SECRET="change-this-long-random-secret" \
  -v chat2responses-data:/app/data \
  chat2responses:local
```

打开 `http://127.0.0.1:4444/admin/`，创建第一个管理员，然后按管理台流程完成配置。

## 快速开始：源码运行

先构建管理台：

```bash
cd admin-ui
npm install
npm run build
cd ..
```

启动 Rust 服务：

```bash
CHAT2RESPONSES_SECRET="change-this-long-random-secret" \
CHAT2RESPONSES_DATABASE_URL="sqlite://data/chat2responses.db" \
cargo run -- --port 4444
```

默认服务地址是 `http://127.0.0.1:4444`，管理台入口是 `http://127.0.0.1:4444/admin/`。

## 管理台配置流程

1. 初始化管理员账号。
2. 在“渠道”里添加上游 provider，填写 OpenAI-compatible base URL 和 provider API key。
3. 同步上游模型，选择要启用的模型；必要时补充上下文长度、是否支持 reasoning summary、是否支持图片输入等元数据。
4. 在“模型映射”里创建公开模型名，例如把 `deepseek-reasoner` 发布为 `codex-deepseek`。
5. 在“密钥”里创建 service API key，分发给 Codex 或其他调用方。
6. 如需排查请求，在“设置”里开启请求日志，再到“日志”查看状态码、token usage、错误内容和耗时。

禁用上游、上游模型、模型映射或 service API key 后，相关请求不会继续路由。若一个 service API key 配置了模型限制，它只能调用被允许的模型。

## Codex 使用方式

在管理台创建 service API key 后，把 Codex 的 OpenAI-compatible base URL 指向：

```text
http://127.0.0.1:4444/v1
```

认证方式使用：

```text
Authorization: Bearer <service-api-key>
```

如果 Codex 提示某个模型缺少 metadata，可以在管理台“教程”页生成 `model-catalog.json`，放到：

```text
~/.codex/model-catalog.json
```

然后在 `~/.codex/config.toml` 顶层加入：

```toml
model_catalog_json = "~/.codex/model-catalog.json"
```

重启 Codex CLI 或当前 Codex 会话后，模型 metadata 会重新加载。

## 上游要求

上游需要提供 OpenAI-compatible Chat Completions 接口。base URL 应填写到 API 根路径，例如：

```text
https://api.example.com/v1
```

Chat2Responses 会基于该地址调用：

- `GET /models`：用于发现上游模型。
- `POST /chat/completions`：用于实际转发请求。

## Runtime Interfaces

| Endpoint | Purpose |
| --- | --- |
| `GET /v1/models` | 返回当前 service API key 可用的模型列表。 |
| `POST /v1/responses` | 接收 Responses API 请求，按 `model` 路由到上游 Chat Completions provider。 |
| `/admin/` | React/Vite 管理台，由 Axum 服务托管。 |
| `/admin/api/*` | 管理台 JSON API，使用 cookie session 登录保护。 |

## 配置项

| Variable | Default | Description |
| --- | --- | --- |
| `CHAT2RESPONSES_HOST` | `127.0.0.1` | 监听地址。Docker 镜像内默认为 `0.0.0.0`。 |
| `CHAT2RESPONSES_PORT` | `4444` | 监听端口。 |
| `CHAT2RESPONSES_DATABASE_URL` | `sqlite://data/chat2responses.db` | SQLite 数据库 URL。Docker 镜像内默认为 `sqlite:///app/data/chat2responses.db`。 |
| `CHAT2RESPONSES_SECRET` | insecure development fallback | 用于加密 provider key、哈希 service key 和 session token。实际部署必须显式设置。 |
| `CHAT2RESPONSES_ADMIN_UI_DIR` | `admin-ui/dist` | 已构建管理台静态文件目录。Docker 镜像内默认为 `/app/admin-ui/dist`。 |
| `RUST_LOG` | `chat2responses=info,tower_http=warn` | 日志级别。 |

## 开发与测试

常用检查：

```bash
cargo test
cd admin-ui && npm run build
```

离线回归测试覆盖协议转换和已捕获的 Codex payload。fixture 位于 `tests/fixtures/codex_<version>/`；当 Codex CLI wire shape 或项目自身转换行为变化时，应新增或更新 fixture，并同步更新 `tests/fixtures/VERSIONS.md`。

Live provider compatibility tests 默认 `#[ignore]`，需要真实 provider API key，并应在默认 `cargo test` 路径之外手动运行。例如：

```bash
DEEPSEEK_API_KEY=sk-... cargo test --test compat_deepseek_live -- --ignored --test-threads=1
MOONSHOT_API_KEY=sk-... cargo test --test compat_kimi_live -- --ignored --test-threads=1
```

## 发布

项目版本号放在 `VERSION` 文件中。发布脚本会读取该文件，推送 Docker 镜像的 `v<version>` 和 `latest` 标签，并创建同名 GitHub Release：

```bash
scripts/release.sh --dry-run
scripts/release.sh
```

如只创建 GitHub Release、跳过 Docker 镜像：

```bash
scripts/release.sh --skip-docker
```

## 兼容性边界

Chat2Responses 会尽量保持 Codex 常用 Responses API 请求路径可用，包括文本输入、部分多模态内容、工具调用、namespace tools flatten、`previous_response_id` 历史续接，以及 reasoning summary 的流式事件转换。

仍需注意：

- 未实现完整 OpenAI Responses API 的全部字段和事件类型。
- 非流式路径保持精简，不等同于官方 Responses API 的完整输出结构。
- 上游 provider 对工具调用、图片输入、reasoning content 和 token usage 的支持差异会直接影响实际行为。
- 请求日志默认关闭；开启后只记录路由、状态、usage、错误摘要和耗时，不保存完整请求正文。
