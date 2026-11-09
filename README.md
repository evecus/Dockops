# DockOps

> 现代化 Docker 管理面板，以 Compose 为核心，**单二进制**部署，前端静态资源编译期内嵌，无需 Nginx、无需配置文件。
>
> 本版本将原 Go 后端 1:1 改写为 Rust，API 路径、请求/响应 JSON、认证、数据库、Compose 行为、WebSocket、SSE 均与 Go 版一致，可直接复用原 `data/dockops.db`（账号与设置）。

![Rust](https://img.shields.io/badge/Rust-1.75+-dea584?style=for-the-badge&logo=rust)
![Vue](https://img.shields.io/badge/Vue-3.4-4FC08D?style=for-the-badge&logo=vue.js)

## 功能特性

- 🐳 **Compose 管理** — 以 docker-compose 为核心管理所有容器
- 📦 **4 种创建方式** — 上传文件 / 粘贴内容 / 解析 docker run / 表单填写
- 🖥️ **Web 终端** — 浏览器直接进入容器终端（WebSocket + Docker exec attach）
- 📋 **实时日志** — WebSocket 实时日志流，支持搜索、高亮、下载
- 📁 **文件管理** — 容器内文件浏览、上传、下载、删除（GNU ls / BusyBox stat / 普通 ls 三级回退）
- 🔄 **镜像更新** — 检测镜像是否有新版本，有更新才拉取
- 🌐 **网络 & 存储** — 完整的 Docker 网络和 Volume 管理
- 🔒 **单二进制** — 前端 `web/dist` 通过 `rust-embed` 内嵌（`//go:embed all:web/dist` 的等价实现）
- 🔐 **HTTPS 自动启用** — 在 `cert/` 目录放置证书文件即可自动开启 TLS（rustls / ring，无 OpenSSL 依赖）

## 快速开始

```bash
# 直接运行，默认 HTTP 9080 端口
./dockops

# 指定端口和数据目录（所有参数可选，支持 -x / --x / -x=value）
./dockops --http 8080 --https 8443 --dir /opt/dockops/data
```

首次访问 `http://your-server:9080` 将引导创建管理员账号。

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `--http` | `9080` | HTTP 监听端口 |
| `--https` | `9443` | HTTPS 监听端口（需有证书才生效） |
| `--dir` | 二进制同级 `data/` 目录 | 数据存储目录 |

### 启用 HTTPS

将证书文件放入数据目录下的 `cert/` 文件夹，程序启动时自动检测并开启 HTTPS：

```
data/
└── cert/
    ├── cert.pem      # 或 fullchain.pem / server.crt
    └── key.pem       # 或 privkey.pem / server.key
```

### Docker 连接

与 Go 版（Docker SDK `FromEnv`）一致，默认连接 `unix:///var/run/docker.sock`，支持：

| 环境变量 | 说明 |
|---|---|
| `DOCKER_HOST` | `unix:///path/docker.sock` 或 `tcp://host:port` |
| `DOCKER_API_VERSION` | 指定 API 版本；未指定时 `GET /_ping` 自动协商 |

## 从源码构建

**依赖：** Rust 1.75+（stable）、Node.js 20+、Docker Engine、`docker compose` CLI。

```bash
# 1. 构建前端（产物写入 web/dist）
cd web && npm ci && npm run build && cd ..

# 2. 编译 Rust 二进制（前端被内嵌）
DOCKOPS_VERSION=v1.0.0 cargo build --release

# 3. 运行
./target/release/dockops
```

`build.rs` 会在 `web/dist/index.html` 缺失时生成占位页面，保证任何情况下 `cargo build` 都不会因缺少前端产物而失败；正式构建请务必先执行第 1 步（仓库内已附带一份预构建 `web/dist`）。

`make build` 等价于上面两步；`make run` 构建后直接运行。

## GitHub Actions（两个 musl 交叉编译 workflow）

| Workflow | Target | 产物 |
|---|---|---|
| `.github/workflows/build-linux-amd64-musl.yml` | `x86_64-unknown-linux-musl` | `dockops-linux-amd64-musl` |
| `.github/workflows/build-linux-arm64-musl.yml` | `aarch64-unknown-linux-musl` | `dockops-linux-arm64-musl` |

两者流程一致：`npm ci && npm run build` → `cross build --release --locked --target <triple>` → 校验二进制为 `statically linked` → 上传 artifact。触发方式：

- `workflow_dispatch`（可填 `tag` 作为版本号）
- `push` 到 `v*` 标签
- `pull_request`（改动源码/前端/Cargo 时）

版本注入：CI 写入 `VERSION` 文件并同时设置 `DOCKOPS_VERSION`，`build.rs` 二者择一读取，等价于 Go 版 `-ldflags "-X main.Version=vX.Y.Z"`。

## Go 版 → Rust 版对照

| Go | Rust | 说明 |
|---|---|---|
| `main.go` | `src/main.rs` | CLI flag、`go:embed` 版本注入、HTTP/HTTPS 双监听 |
| `internal/config` | `src/config.rs` | 默认端口、data 目录、证书自动探测 |
| `internal/db` (`glebarez/sqlite`) | `src/db.rs` (`rusqlite` bundled) | 同库同表 `dockops.db`，WAL + foreign_keys |
| `internal/auth` (jwt/v5 + bcrypt) | `src/auth.rs` + `src/jwt.rs` | 同一 secret、HS256、24h 有效期，bcrypt `$2a$` 互认 |
| `internal/middleware` | `src/handler.rs::auth_middleware` | Header / `?token=` / `Bearer ` 前缀，401 JSON 一致 |
| `internal/handler` (gin) | `src/handler.rs`、`src/handlers*.rs` | 全部 `/api` 路由、`ok`/`fail` JSON 包装、SSE |
| `internal/docker` (Docker SDK) | `src/docker.rs` + `src/docker_api.rs` | 直连 Docker Engine HTTP API（Unix socket/TCP），版本协商、hijack、chunked 解码 |
| `internal/compose` | `src/compose.rs` | `docker compose up/down/pull`、compose 文件读写、名称提取 |
| `internal/parser` | `src/parser.rs` | `docker run` → compose YAML + service JSON |
| `internal/scheduler` (cron) | `src/scheduler.rs` | 仪表盘采集 / 镜像更新检查，cron 对齐语义 |
| `internal/ws` (gorilla) | `src/ws.rs` (axum) | 终端 / 实时日志 WebSocket |
| `web/` (`//go:embed`) | `web/` (`rust-embed`) | 前端源码原样保留，构建产物内嵌 |

### 关键实现说明

- **HTTP 客户端**：不依赖 hyper/openssl，基于 tokio `UnixStream`/`TcpStream` 手写 HTTP/1.1，支持 Content-Length、chunked、无长度流以及 `101 Switching Protocols` 的 exec attach，全部依赖可静态链接到 musl。
- **JWT**：本地实现 HS256（sha2 + hmac + base64），与 `golang-jwt/jwt/v5` 生成的 token 双向兼容。
- **密码**：`bcrypt` crate cost=10，输出前缀归一化为 `$2a$`，与 Go 版 `x/crypto/bcrypt` 互通。
- **TLS**：服务端使用 `rustls` + `ring` provider（避开 aws-lc-rs 的 CMake 交叉编译依赖）。
- **数据库**：`rusqlite` 的 `bundled` 特性在 cross 容器内用 musl 工具链编译 SQLite amalgamation，最终二进制无动态依赖。

## 项目结构

```
dockops-rs/
├── Cargo.toml / Cargo.lock / build.rs
├── src/                     # Rust 后端
├── web/                     # Vue 3 前端（源码 + dist 产物）
├── .github/workflows/       # amd64 / arm64 musl 构建
├── Makefile
└── README.md
```

## License

MIT
