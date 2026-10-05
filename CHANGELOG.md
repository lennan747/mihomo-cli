# 变更日志

本项目遵循[语义化版本](https://semver.org/lang/zh-CN/)。所有值得注意的变更记录于此。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)。

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
