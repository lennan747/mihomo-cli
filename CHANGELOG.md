# 变更日志

本项目遵循[语义化版本](https://semver.org/lang/zh-CN/)。所有值得注意的变更记录于此。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)。

## [Unreleased]

### 修复

- 输出被下游提前关闭（如 `mihomo-cli rule list | head`）不再 panic：Unix 下启动时恢复 `SIGPIPE` 默认处置，进程静默退出（退出码 141），取代原来 `println!` 因 `EPIPE` panic 并打印 backtrace 的行为。

## [0.2.0] - 2026-10-07

### 变更

- 输出样式重写（参考 qq-triage）：`status`/`group`/`proxy`/`rule`/`conn`/`version`/`mode` 等改用统一的圆角框 dashboard（标题内嵌顶边、`├─┤` 分隔、底栏汇总），面板标题与表头改用英文；新增 `src/ui.rs` 渲染工具箱，按可见宽度对齐（中文按 2 列、ANSI 不计宽），超长内容截断补 `…`；引入 TTY 自适应颜色（`NO_COLOR` 可关闭，管道/重定向自动无色）。移除 `comfy-table` 依赖，新增 `unicode-width` / `terminal_size`。

### 新增

- `self-upgrade [--check] [--version <vX.Y.Z>] [--force]`：升级 mihomo-cli 客户端自身（区别于升级内核的 `upgrade`）。从 GitHub Releases 下载对应平台资产，强制校验 `SHA256SUMS` 后原地替换当前可执行文件；直连失败自动回退本机 7890 代理。支持 Linux x86_64 / macOS arm64 / Windows x86_64。

## [0.1.1] - 2026-10-05

### 修复

- 适配 comfy-table 8（`load_preset` → `load_style`），解除 dependabot 升级阻塞；本仓库所有表格输出不变。

### 新增

- 一键安装：`install.sh`（Linux/macOS）与 `install.ps1`（Windows x86_64），从 GitHub Releases 拉取二进制并强制 SHA256 校验；支持 `MIHOMO_CLI_VERSION` / `MIHOMO_CLI_INSTALL_DIR` / `MIHOMO_CLI_NO_PATH` 环境变量。
- 发布流水线（`release.yml`）：推送 `v*` 标签触发，门禁（fmt+clippy+test）→ 多平台构建（Linux x86_64 / macOS arm64 / Windows x86_64）→ 生成 `SHA256SUMS` 并发布 GitHub Release → 一键安装端到端验证。
- 手动回归工作流 `install-test.yml`（`workflow_dispatch` 触发一键安装检查）。
- `docs/install-guide.md`：面向 AI Agent 的安装与验证指南。

## [0.1.0] - 2026-10-05

### 新增

- 首个版本：管理本机 mihomo (Clash Meta) 实例的命令行工具，经外部控制器 REST API（默认 `http://127.0.0.1:9090`）与 `systemctl --user` 用户服务交互。
- `status`：版本、运行模式、日志级别、监听端口与全部策略组当前选中的总览。
- `proxy list [group]` / `proxy test [group] [--timeout]` / `proxy select <group> <name>` / `proxy update`：节点列表（`★` 标记选中）、延迟测试（按组整体测或全节点并发测，结果按延迟升序）、切换选中节点（切换前校验组与成员存在）、订阅更新。
- `group list`：策略组、类型、节点数与当前选中。
- `rule list`：分流规则（按配置顺序编号）。
- `conn list [--limit]` / `conn close --all`：活动连接查看（目标/网络/规则/流量/链路）与关闭（需显式 `--all`）。
- `logs [--level]`：WebSocket 实时日志流，Ctrl-C 退出。
- `mode [rule|global|direct]`：查看或热切换运行模式。
- `config reload`：热重载 `~/.config/mihomo/config.yaml`。
- `service restart` / `service status`：systemd 用户服务管理，重启后轮询等待 API 恢复。
- `sub update` / `sub url` / `sub set-url <URL>`：订阅管理，更新与 `mihomo-update.timer` 定时任务走同一脚本。
- `version`：对照 GitHub 最新 release 检查更新，直连失败自动回退本机 7890 代理。
- `upgrade`：调用 `/upgrade` 端点在线升级内核并等待恢复。
- secret 解析链：`-s/--secret` > `MIHOMO_SECRET` 环境变量 > `~/.config/mihomo/mihomo.env` 文件。
- URL 拼接统一走 `ApiClient`，含中文/emoji 的节点名自动百分号编码。

### 工程化

- 单元测试：字节格式化（`human_bytes`）、环境文件解析（`parse_env_file`）、URL 拼接与编码、版本号比较（`normalize`）。
- CI（GitHub Actions）：`cargo fmt --check` + `cargo clippy --all-targets -- -D warnings` + `cargo test`。
- Dependabot 跟踪 cargo 与 GitHub Actions 依赖。
- 项目文档：README、CONTRIBUTING、SECURITY、MIT LICENSE。
