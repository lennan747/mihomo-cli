# mihomo-cli 安装指南（面向 AI Agent）
# mihomo-cli Install Guide (for AI Agents)

本指南面向 AI Agent（Claude Code、opencode、Cursor、Trae 等）。安装过程中需要用户参与的步骤已明确标注。
This guide is designed for AI Agents (Claude Code, opencode, Cursor, Trae, etc.). Steps that require user interaction are marked explicitly.

## 前置条件 / Prerequisites

- Linux/macOS：`curl` 与 `sh`；Windows：PowerShell 5.1+（x86_64）。若从源码构建需 Rust ≥ 1.80。
- Linux/macOS: `curl` and `sh`; Windows: PowerShell 5.1+ (x86_64). Rust ≥ 1.80 if building from source.
- 本机运行 mihomo (Clash Meta) 内核，且外部控制器已开启（默认 `http://127.0.0.1:9090`）。`service`/`sub` 子命令额外依赖 systemd 用户服务（仅 Linux）。
- A local running mihomo (Clash Meta) kernel with external controller enabled (default `http://127.0.0.1:9090`). `service`/`sub` additionally require a systemd user service (Linux only).

## Step 1 — 安装 CLI / Install CLI

Linux / macOS（arm64）：

```shell
curl -fsSL https://raw.githubusercontent.com/lennan747/mihomo-cli/master/install.sh | sh
```

- 安装到 `~/.local/bin/mihomo-cli`（可用 `MIHOMO_CLI_INSTALL_DIR` 覆盖）。
- 指定版本：`MIHOMO_CLI_VERSION=v0.2.0 curl -fsSL ... | sh`。

Windows（PowerShell 5.1+，x86_64）：

```powershell
irm https://raw.githubusercontent.com/lennan747/mihomo-cli/master/install.ps1 | iex
```

- 安装到 `%LOCALAPPDATA%\mihomo-cli\bin` 并自动加入用户 PATH（**新开终端生效**）。可用 `MIHOMO_CLI_VERSION` / `MIHOMO_CLI_INSTALL_DIR` / `MIHOMO_CLI_NO_PATH` 覆盖。

安装后确认版本：

```shell
mihomo-cli --version   # 期望输出 mihomo-cli 0.1.x
```

## Step 2 — 验证 / Verify

```shell
mihomo-cli status
```

能打印版本、运行模式与策略组表格即安装成功。

- 报 `无法连接 mihomo API`：mihomo 未运行或外部控制器地址/端口与 `--api` 不符。
- 报 `API 返回 401`：需要 secret，见下节。

## Step 3 — Secret（仅在 mihomo 配置了鉴权时需要 / only if auth is enabled）

secret 按以下顺序解析，通常**无需任何配置**：

1. `~/.config/mihomo/mihomo.env` 中的 `MIHOMO_SECRET`（与 systemd 用户服务共享的环境文件，自动读取）；
2. 环境变量 `MIHOMO_SECRET`；
3. 命令行 `-s/--secret`。

仅当以上均未提供且 mihomo 开启了鉴权时才需要用户参与：向用户索取 secret 后 `export MIHOMO_SECRET=<secret>` 或传 `-s`。
Only ask the user for the secret when none of the sources above provide it and auth is enabled.

> secret 属于用户敏感信息，不得输出到公开渠道或写入仓库提交。

## 安全须知 / Security

- 不向公开渠道输出 secret、订阅地址（含 token）、`mihomo.env`、`subscription.url` 内容。
- `conn close --all` 会关闭全部活动连接，非用户明确要求不得执行。
- `upgrade` 会重启 mihomo 内核进程，执行前须告知用户。

## 退出码参考 / Exit Codes

| 退出码 | 含义 |
|---|---|
| 0 | 成功 |
| 1 | 任何错误（连接失败、API 报错、校验不通过等） |

更多命令说明见仓库 `README.md`。
