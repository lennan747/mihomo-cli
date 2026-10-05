# 贡献指南

感谢你考虑为 mihomo-cli 做贡献！本文说明如何本地开发、测试、提交 PR。

## 环境要求

- Rust >= 1.80（见 `Cargo.toml` 的 `rust-version`）
- 建议安装 `rustfmt` 与 `clippy` 组件
- 大多数命令的真实运行需要本机 mihomo 部署环境（见 README「运行时环境」）

## 本地开发

```bash
git clone https://github.com/lennan747/mihomo-cli.git
cd mihomo-cli

cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release --locked
```

## 代码结构

```text
src/main.rs      入口：解析 CLI、分发子命令
src/cli.rs       clap 派生宏定义的全部子命令/参数
src/api.rs       ApiClient：REST API 薄封装（鉴权/编码/错误中文化）
src/env.rs       secret 解析（命令行/环境变量/mihomo.env 文件回退）
src/models.rs    API 响应的 serde 模型
src/cmds/        每个子命令一个模块
```

约定：

- 错误处理统一用 `anyhow`：底层错误加 `.context()` 中文上下文，业务错误用 `bail!`，面向用户的消息一律中文。
- 表格输出统一用 `comfy-table` 的 `UTF8_FULL_CONDENSED` 预设。
- API 路径段一律通过 `ApiClient` 的 `url()` 拼接（保证非 ASCII 名称正确编码），不要手工拼 URL 字符串。
- 涉及本机路径时使用 `$HOME` 下的固定位置，不要硬编码绝对路径。
- 解释性中文注释只在行为不显然处添加。

## 测试与门禁

提交 PR 前请确保全部通过（CI 也会执行同样检查）：

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

涉及 API 交互的命令无法在 CI 中真实验证，请在装有 mihomo 的本机做只读冒烟（如 `mihomo-cli status`、`mihomo-cli proxy list`）。

## 提交信息约定

- 格式：`类型: 简述`（中文），例如 `feat: 支持 conn close 单条关闭`、`fix: 修复组名含 emoji 时切换失败`、`docs: 更新命令说明`
- 类型：`feat` / `fix` / `docs` / `refactor` / `test` / `chore` / `ci`
- 一个提交只做一件事

## 提交流程

1. Fork 本仓库并创建功能分支（`feat/xxx` 或 `fix/xxx`）。
2. 提交前运行本地门禁。
3. 更新 `CHANGELOG.md`（`Unreleased` 部分）。
4. 推送分支并提交 Pull Request，描述改动动机、影响范围与验证方式。
5. 等待 CI 通过与 review。

## 安全相关约定

- **不得提交** `mihomo.env`、`subscription.url`、订阅地址或任何含真实 token 的内容。
- 测试数据统一使用脱敏值（`https://example.com/subscribe` 等）。
- 不要在代码、日志或错误信息中打印 secret。

## 报告问题

- Bug：使用 [Issue 模板](https://github.com/lennan747/mihomo-cli/issues/new?template=bug_report.md)，附复现步骤。
- 功能请求：使用[功能请求模板](https://github.com/lennan747/mihomo-cli/issues/new?template=feature_request.md)。
- 安全漏洞：见 [SECURITY.md](SECURITY.md)，请勿公开披露。
