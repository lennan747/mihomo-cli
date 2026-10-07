# AGENTS.md

## 项目概述

mihomo-cli 是一个用 Rust 编写的命令行工具，用于管理本机运行的 mihomo (Clash Meta) 代理内核。它通过 mihomo 的外部控制器 REST API（默认 `http://127.0.0.1:9090`）与 `systemctl --user` 用户服务进行交互，提供状态查看、节点切换、延迟测试、日志跟踪、订阅更新、内核升级、客户端自升级等功能。

所有用户界面文案、注释和错误提示均为**简体中文**。

## 构建与运行

- 构建：`cargo build`（release 版本用 `cargo build --release`）
- 运行：`cargo run -- <子命令>`，或使用已编译的二进制（`target/release/mihomo-cli`）
- 检查：`cargo check`；代码检查可用 `cargo clippy`（门禁为 `cargo clippy --all-targets -- -D warnings`）；格式 `cargo fmt --check`
- 依赖：`anyhow`、`clap` 4（derive 模式，支持 env 回退）、`reqwest` 0.12（rustls）、`tokio`、`tokio-tungstenite`（WebSocket 日志流）、`serde`/`serde_json`、`futures-util`、`unicode-width` + `terminal_size`（终端渲染）；`self-upgrade` 另用 `sha2`/`tar`/`flate2`/`zip`/`self-replace`

Rust edition 2021，无 workspace，单 crate（`rust-version = 1.80`）。GitHub Actions：CI（`.github/workflows/ci.yml`，fmt + clippy + test）、发布流水线（`release.yml`，推送 `v*` 标签触发：门禁 → 多平台构建 → SHA256SUMS → GitHub Release → 一键安装验证）、一键安装回归（`install-test.yml`，手动触发）。

## 项目文档

- `README.md` — 面向用户的完整说明（安装、全部命令、运行时环境、故障排查）
- `CHANGELOG.md` — 变更日志（Keep a Changelog 格式，发版时更新）
- `CONTRIBUTING.md` / `SECURITY.md` — 贡献流程与安全策略
- `LICENSE` — MIT
- `install.sh` / `install.ps1` / `docs/install-guide.md` — 一键安装脚本与安装指南

## 代码结构

- `src/main.rs` — 入口，`#[tokio::main]`，解析 CLI 后把子命令分发到 `cmds::*`
- `src/cli.rs` — clap 派生宏定义的全部子命令/参数；全局参数 `--api`（env `MIHOMO_API`，默认 `http://127.0.0.1:9090`）和 `-s/--secret`（env `MIHOMO_SECRET`）
- `src/api.rs` — `ApiClient`：对 REST API 的薄封装（get/put/patch/delete/post），统一处理 Bearer 鉴权、URL 路径段百分号编码（节点名含中文和 emoji）、错误信息中文化
- `src/env.rs` — secret 解析：命令行/环境变量优先，否则读 `~/.config/mihomo/mihomo.env`
- `src/github.rs` — GitHub Release 查询与版本号比较的共享工具（GitHub API → 302 跳转，直连失败回退本机 7890 代理；`normalize_version`），供 `version` 与 `self-upgrade` 复用
- `src/models.rs` — API 响应的 serde 模型（Version/Configs/Proxy/Rule/Connection 等）
- `src/ui.rs` — 终端渲染工具箱：圆角框（`Box`）+ 表格 dashboard（`table_dashboard`）+ 键值面板（`kv_panel`）+ 状态条（`status_bar`）；可见宽度按 CJK=2 计算、ANSI 不计宽，颜色仅在 TTY 且未设 `NO_COLOR` 时启用。**所有表格/框线输出必须走此模块**，参考 qq-triage 的样式
- `src/cmds/` — 每个子命令一个模块：`status`、`proxy`（list/test/select/update）、`group`、`rule`、`conn`、`logs`（WebSocket 实时日志）、`mode`、`config`（热重载）、`service`（systemd 用户服务）、`sub`（订阅管理）、`version`（查 GitHub 最新 release）、`upgrade`（调 `/upgrade` 自升级内核）、`self_upgrade`（从 GitHub Releases 下载并 SHA256 校验后原地替换 CLI 自身）；`cmds/mod.rs` 提供共享工具函数如 `human_bytes`

新增子命令的惯例：在 `cli.rs` 的 `Command` 枚举加变体并写中文 doc 注释 → 在 `cmds/` 下建同名模块 → 在 `main.rs` 的 match 中分发。

## 运行时环境与外部依赖

该工具强耦合于本机的 mihomo 部署环境（大多数命令离开此环境无法真正执行）：

- mihomo 内核二进制在 `~/.local/bin/mihomo`，由 systemd 用户服务 `mihomo.service` 管理（`~/.config/systemd/user/` 下还有 `mihomo-update.service` / `mihomo-update.timer`，每小时自动更新订阅）
- 配置目录 `~/.config/mihomo/`：`config.yaml`（运行配置）、`mihomo.env`（含 `MIHOMO_SECRET`，机密文件，**不要读取或输出其内容**）、`subscription.url`（订阅地址）、geoip/geosite 数据文件
- 订阅更新委托给 shell 脚本 `~/.local/bin/mihomo-update`（curl 拉取 → sed 修正 → `mihomo -t` 校验 → 热重载 API）；`sub update` 与 `proxy update` 都是执行该脚本
- `version` 命令需要访问 GitHub API，失败时自动回退走 `http://127.0.0.1:7890` 混合代理端口

## 测试

- 单元测试：`cargo test`（纯函数：`human_bytes`、`parse_env_file`、`ApiClient::url` 编码、版本号 `normalize_version`、`self_upgrade` 的平台映射/资产名/SHA256SUMS 解析、`ui` 的可见宽度/截断/框对齐；以 `#[cfg(test)]` 模块内联在各源文件）
- 静态检查：`cargo clippy --all-targets -- -D warnings`
- 真实环境冒烟：在装有 mihomo 的本机上实际运行子命令验证（如 `mihomo-cli status`、`mihomo-cli proxy list`），只读优先

不新增依赖真实 mihomo 实例的测试脚手架（CI 无该环境）；纯逻辑变化优先补内联单元测试。

## 代码风格与约定

- 错误处理统一用 `anyhow`：底层错误加 `.context()` 中文上下文，业务错误用 `bail!`，面向用户的消息一律中文
- 表格/框线输出统一走 `crate::ui`（`table_dashboard` / `kv_panel` / `status_bar`）：面板标题与表头用英文，单元格可含 ANSI 色码（对齐按可见宽度计算）；不要在命令模块里手工拼框线或自行计算宽度
- API 路径段一律通过 `ApiClient` 的 `url()` 拼接（保证非 ASCII 名称正确编码），不要手工拼 URL 字符串
- 涉及本机路径时使用 `$HOME` 下的固定位置（见上文），不要硬编码 `/home/zln`
- 注释风格：每个源文件有简短中文模块级 `//!` 文档；函数注释只在行为不显然处添加，不写冗余注释

## 安全注意事项

- `MIHOMO_SECRET` 是 API 鉴权密钥，可通过命令行、环境变量或 `~/.config/mihomo/mihomo.env` 提供；绝不在日志/输出中打印它
- `~/.config/mihomo/` 下的文件多为机密或敏感数据（订阅 URL 含用户 token），工具写入 `subscription.url` 等内容时注意不泄露
- 订阅下载使用 `mihomo -t` 校验后才替换 `config.yaml`，修改 `sub`/更新脚本逻辑时必须保留这一安全步骤
