# AGENTS.md

## 项目概述

mihomo-cli 是一个用 Rust 编写的命令行工具，用于管理本机运行的 mihomo (Clash Meta) 代理内核。它通过 mihomo 的外部控制器 REST API（默认 `http://127.0.0.1:9090`）与 `systemctl --user` 用户服务进行交互，提供状态查看、节点切换、延迟测试、日志跟踪、订阅更新、内核升级等功能。

所有用户界面文案、注释和错误提示均为**简体中文**。

## 构建与运行

- 构建：`cargo build`（release 版本用 `cargo build --release`）
- 运行：`cargo run -- <子命令>`，或使用已编译的二进制（`target/release/mihomo-cli`）
- 检查：`cargo check`；代码检查可用 `cargo clippy`
- 依赖：`anyhow`、`clap` 4（derive 模式，支持 env 回退）、`reqwest` 0.12（rustls）、`tokio`、`tokio-tungstenite`（WebSocket 日志流）、`serde`/`serde_json`、`comfy-table`（表格输出）、`futures-util`

Rust edition 2021，无 workspace，单 crate，无 README、无 git 仓库。

## 代码结构

- `src/main.rs` — 入口，`#[tokio::main]`，解析 CLI 后把子命令分发到 `cmds::*`
- `src/cli.rs` — clap 派生宏定义的全部子命令/参数；全局参数 `--api`（env `MIHOMO_API`，默认 `http://127.0.0.1:9090`）和 `-s/--secret`（env `MIHOMO_SECRET`）
- `src/api.rs` — `ApiClient`：对 REST API 的薄封装（get/put/patch/delete/post），统一处理 Bearer 鉴权、URL 路径段百分号编码（节点名含中文和 emoji）、错误信息中文化
- `src/env.rs` — secret 解析：命令行/环境变量优先，否则读 `~/.config/mihomo/mihomo.env`
- `src/models.rs` — API 响应的 serde 模型（Version/Configs/Proxy/Rule/Connection 等）
- `src/cmds/` — 每个子命令一个模块：`status`、`proxy`（list/test/select/update）、`group`、`rule`、`conn`、`logs`（WebSocket 实时日志）、`mode`、`config`（热重载）、`service`（systemd 用户服务）、`sub`（订阅管理）、`version`（查 GitHub 最新 release，直连失败回退本机 7890 代理）、`upgrade`（调 `/upgrade` 自升级）；`cmds/mod.rs` 提供共享工具函数如 `human_bytes`

新增子命令的惯例：在 `cli.rs` 的 `Command` 枚举加变体并写中文 doc 注释 → 在 `cmds/` 下建同名模块 → 在 `main.rs` 的 match 中分发。

## 运行时环境与外部依赖

该工具强耦合于本机的 mihomo 部署环境（大多数命令离开此环境无法真正执行）：

- mihomo 内核二进制在 `~/.local/bin/mihomo`，由 systemd 用户服务 `mihomo.service` 管理（`~/.config/systemd/user/` 下还有 `mihomo-update.service` / `mihomo-update.timer`，每小时自动更新订阅）
- 配置目录 `~/.config/mihomo/`：`config.yaml`（运行配置）、`mihomo.env`（含 `MIHOMO_SECRET`，机密文件，**不要读取或输出其内容**）、`subscription.url`（订阅地址）、geoip/geosite 数据文件
- 订阅更新委托给 shell 脚本 `~/.local/bin/mihomo-update`（curl 拉取 → sed 修正 → `mihomo -t` 校验 → 热重载 API）；`sub update` 与 `proxy update` 都是执行该脚本
- `version` 命令需要访问 GitHub API，失败时自动回退走 `http://127.0.0.1:7890` 混合代理端口

## 测试

项目目前**没有测试代码**（无 tests/ 目录、无 #[test]）。验证方式：

1. `cargo build` 编译通过
2. 在装有 mihomo 的本机上实际运行子命令验证（如 `mihomo-cli status`、`mihomo-cli proxy list`）

不要自行新增测试脚手架，除非用户明确要求。

## 代码风格与约定

- 错误处理统一用 `anyhow`：底层错误加 `.context()` 中文上下文，业务错误用 `bail!`，面向用户的消息一律中文
- 表格输出统一用 `comfy-table` 的 `UTF8_FULL_CONDENSED` 预设
- API 路径段一律通过 `ApiClient` 的 `url()` 拼接（保证非 ASCII 名称正确编码），不要手工拼 URL 字符串
- 涉及本机路径时使用 `$HOME` 下的固定位置（见上文），不要硬编码 `/home/zln`
- 默认不写注释；解释性中文注释只在行为不显然处保留（现有代码即如此）

## 安全注意事项

- `MIHOMO_SECRET` 是 API 鉴权密钥，可通过命令行、环境变量或 `~/.config/mihomo/mihomo.env` 提供；绝不在日志/输出中打印它
- `~/.config/mihomo/` 下的文件多为机密或敏感数据（订阅 URL 含用户 token），工具写入 `subscription.url` 等内容时注意不泄露
- 订阅下载使用 `mihomo -t` 校验后才替换 `config.yaml`，修改 `sub`/更新脚本逻辑时必须保留这一安全步骤
