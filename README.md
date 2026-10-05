# mihomo-cli

[![GitHub Release](https://img.shields.io/github/v/release/lennan747/mihomo-cli)](https://github.com/lennan747/mihomo-cli/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/lennan747/mihomo-cli/ci.yml?branch=master&label=ci)](https://github.com/lennan747/mihomo-cli/actions)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

管理本机 mihomo (Clash Meta) 实例的命令行工具。通过 mihomo 的外部控制器 REST API（默认 `http://127.0.0.1:9090`）与 `systemctl --user` 用户服务进行交互，提供状态查看、节点切换、延迟测试、日志跟踪、订阅更新、内核升级等功能。

## 特性

- **状态总览**：版本、运行模式、监听端口、各策略组当前选中，一屏可见。
- **节点管理**：节点/策略组列表、延迟测试（单节点并发测或按组整体测）、切换选中节点。
- **分流与连接**：分流规则列表、活动连接查看（含链路与流量）、一键关闭全部连接。
- **实时日志**：WebSocket 跟踪 mihomo 日志流，按级别过滤，Ctrl-C 退出。
- **热操作**：运行模式切换、配置热重载，均不重启服务。
- **服务与订阅**：systemd 用户服务重启/状态查询；订阅地址管理与手动更新（与每小时定时任务走同一脚本）。
- **版本与升级**：对照 GitHub 最新 release 检查更新，调用 `/upgrade` 端点在线升级内核。

## 目录

- [安装](#安装)
- [快速开始](#快速开始)
- [命令](#命令)
  - [通用](#通用)
  - [status](#status)
  - [proxy](#proxy)
  - [group](#group)
  - [rule](#rule)
  - [conn](#conn)
  - [logs](#logs)
  - [mode](#mode)
  - [config](#config)
  - [service](#service)
  - [sub](#sub)
  - [version](#version)
  - [upgrade](#upgrade)
- [运行时环境](#运行时环境)
- [配置与凭据安全](#配置与凭据安全)
- [退出码](#退出码)
- [故障排查](#故障排查)
- [开发](#开发)

## 安装

> 要求：Linux / macOS arm64 / Windows x86_64（`service`/`sub` 子命令依赖 systemd 用户服务与订阅脚本，仅 Linux 可用；其余命令跨平台）；源码安装要求 Rust >= 1.80；一键安装支持 SHA256 校验。

### 一键安装（推荐）

从 GitHub Releases 拉取最新二进制，自动校验 SHA256。

Linux / macOS：

```bash
curl -fsSL https://raw.githubusercontent.com/lennan747/mihomo-cli/master/install.sh | sh
```

- 安装到 `~/.local/bin/mihomo-cli`（可用 `MIHOMO_CLI_INSTALL_DIR` 覆盖）。
- 指定版本：`MIHOMO_CLI_VERSION=v0.1.1 curl -fsSL https://raw.githubusercontent.com/lennan747/mihomo-cli/master/install.sh | sh`。

Windows（PowerShell 5.1+，x86_64）：

```powershell
irm https://raw.githubusercontent.com/lennan747/mihomo-cli/master/install.ps1 | iex
```

- 安装到 `%LOCALAPPDATA%\mihomo-cli\bin\mihomo-cli.exe`，并自动加入用户 PATH（**新开终端生效**）。
- 可用环境变量覆盖：`MIHOMO_CLI_VERSION`（指定版本）、`MIHOMO_CLI_INSTALL_DIR`（安装目录）、`MIHOMO_CLI_NO_PATH=1`（跳过 PATH 写入）。

### cargo 安装

```bash
cargo install --git https://github.com/lennan747/mihomo-cli.git --locked
```

### 手动构建

```bash
git clone https://github.com/lennan747/mihomo-cli.git
cd mihomo-cli
cargo build --release --locked
install -m 0755 target/release/mihomo-cli ~/.local/bin/
```

安装后运行 `mihomo-cli status` 验证（见[快速开始](#快速开始)）；面向 AI Agent 的完整分步说明见 [docs/install-guide.md](docs/install-guide.md)。本工具强耦合于本机的 mihomo 部署环境（见[运行时环境](#运行时环境)），大多数命令需要 mihomo 已在本机运行。

## 快速开始

```bash
# 1. 状态总览：版本、模式、端口、各策略组当前选中
mihomo-cli status

# 2. 查看某策略组的节点与延迟
mihomo-cli proxy list PROXY
mihomo-cli proxy test PROXY

# 3. 切换节点
mihomo-cli proxy select PROXY "香港 01"

# 4. 跟踪实时日志（Ctrl-C 退出）
mihomo-cli logs

# 5. 更新订阅并热重载
mihomo-cli sub update
```

## 命令

### 通用

**全局选项**（所有子命令均可用）：

| 选项 | 说明 |
|---|---|
| `--api <url>` | mihomo 外部控制器地址，默认 `http://127.0.0.1:9090`；环境变量 `MIHOMO_API` |
| `-s, --secret <secret>` | API 鉴权密钥；环境变量 `MIHOMO_SECRET`；均未提供时读 `~/.config/mihomo/mihomo.env`（见[配置与凭据安全](#配置与凭据安全)） |

**输出样式**：表格统一使用 `UTF8_FULL_CONDENSED` 预设；延迟测试结果按延迟升序排列，超时节点排在末尾；流量字节数按 1024 进制转为人类可读。

---

### status

运行状态总览。

```
mihomo-cli status
```

输出：mihomo 版本、运行模式、日志级别、局域网开关、监听端口（混合/SOCKS/HTTP/redir/tproxy，仅显示已启用的），以及全部策略组的类型与当前选中。

---

### proxy

节点操作：列出 / 测速 / 切换 / 更新。

#### proxy list

列出节点；指定策略组名则列出该组节点（标记当前选中 `★`），否则列出全部节点。

```
mihomo-cli proxy list [group]
```

```bash
mihomo-cli proxy list          # 全部节点（含类型与最近一次延迟）
mihomo-cli proxy list PROXY    # 某策略组的节点，★ 标记当前选中
```

#### proxy test

延迟测试并按延迟排序；指定策略组名只测该组（mihomo 组内整体测），否则并发测全部节点。

```
mihomo-cli proxy test [group] [--timeout <毫秒>]
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `--timeout <毫秒>` | 否 | 超时时间，默认 5000 |

```bash
mihomo-cli proxy test
mihomo-cli proxy test PROXY --timeout 3000
```

> 按组测试时无响应的节点不会出现在结果里，末尾会提示 `（x/y 个节点有响应，其余超时或不可用）`。

#### proxy select

切换策略组的选中节点。

```
mihomo-cli proxy select <group> <name>
```

```bash
mihomo-cli proxy select PROXY "香港 01"
```

> 切换前会校验：策略组存在且目标节点在该组内，否则报错并提示可用的查看命令。

#### proxy update

从订阅拉取最新节点并热更新，等价于 [`sub update`](#sub-update)。

```
mihomo-cli proxy update
```

---

### group

#### group list

列出全部策略组、类型、节点数与当前选中。

```
mihomo-cli group list
```

---

### rule

#### rule list

列出分流规则，按配置文件中的顺序编号展示。

```
mihomo-cli rule list
```

---

### conn

活动连接操作。

#### conn list

列出活动连接。

```
mihomo-cli conn list [-l <条数>]
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `-l, --limit <条数>` | 否 | 最多显示条数，默认 20 |

输出：目标（域名优先，否则 IP:端口）、网络（tcp/udp）、命中规则、上下行流量、代理链路；顶部汇总累计流量。

#### conn close

关闭连接。影响面大，必须显式指定 `--all`。

```
mihomo-cli conn close --all
```

---

### logs

实时跟踪 mihomo 日志（WebSocket 日志流），Ctrl-C 退出。

```
mihomo-cli logs [--level <级别>]
```

| 选项 | 必填 | 说明 |
|---|---|---|
| `--level <级别>` | 否 | info / warning / error / debug / silent，默认 info |

---

### mode

查看或切换运行模式。切换经 API 热生效，不写配置文件（重启后回落到 `config.yaml` 的值）。

```
mihomo-cli mode [rule|global|direct]
```

```bash
mihomo-cli mode          # 查看当前模式
mihomo-cli mode global   # 切换为全局代理
```

---

### config

#### config reload

热重载 `~/.config/mihomo/config.yaml`（`PUT /configs?force=true`），不重启服务。

```
mihomo-cli config reload
```

---

### service

管理 systemd 用户服务 `mihomo.service`。

#### service restart

重启 mihomo 服务并轮询等待 API 恢复（最长 10 秒）。

```
mihomo-cli service restart
```

#### service status

查看 mihomo 服务状态（透传 `systemctl --user status mihomo` 输出）。

```
mihomo-cli service status
```

---

### sub

订阅管理：查看/设置订阅地址，拉取最新节点。

#### sub update

拉取订阅并热更新配置。执行本机脚本 `~/.local/bin/mihomo-update`（curl 拉取 → sed 修正 → `mihomo -t` 校验 → 热重载），与每小时执行的 `mihomo-update.timer` 定时任务走同一脚本。

```
mihomo-cli sub update
```

#### sub url

查看当前订阅地址：优先 `~/.config/mihomo/subscription.url`，其次从更新脚本中提取默认值。

```
mihomo-cli sub url
```

#### sub set-url

设置订阅地址，写入 `~/.config/mihomo/subscription.url`（每小时定时更新同样读取此文件）。

```
mihomo-cli sub set-url <URL>
```

```bash
mihomo-cli sub set-url https://example.com/subscribe?token=xxx
```

> 仅接受 `http://` / `https://` URL；订阅地址含个人 token，注意不要泄露。

---

### version

查看 mihomo 当前版本并检查 GitHub 最新 release。直连 GitHub 失败时自动回退本机混合代理端口 7890。

```
mihomo-cli version
```

---

### upgrade

在线升级 mihomo 内核（调用 `/upgrade` 自升级端点，下载新内核并原地重启进程），完成后等待 API 恢复并报告前后版本。

```
mihomo-cli upgrade
```

> 已是最新版本时 mihomo 返回错误，CLI 会识别并提示「已是最新版本」；升级期间连接中断属正常现象。

---

## 运行时环境

本工具强耦合于本机的 mihomo 部署环境（大多数命令离开此环境无法真正执行）：

- mihomo 内核二进制在 `~/.local/bin/mihomo`，由 systemd 用户服务 `mihomo.service` 管理；`~/.config/systemd/user/` 下另有 `mihomo-update.service` / `mihomo-update.timer`，每小时自动更新订阅。
- 配置目录 `~/.config/mihomo/`：`config.yaml`（运行配置）、`mihomo.env`（服务环境变量，含 `MIHOMO_SECRET`）、`subscription.url`（订阅地址）、geoip/geosite 数据文件。
- 订阅更新委托给 shell 脚本 `~/.local/bin/mihomo-update`（curl 拉取 → sed 修正 → `mihomo -t` 校验 → 热重载）；`sub update` 与 `proxy update` 都是执行该脚本。
- `version` 命令需要访问 GitHub API，失败时自动回退走 `http://127.0.0.1:7890` 混合代理端口。

## 配置与凭据安全

API 鉴权密钥按以下顺序解析：

1. `-s/--secret` 命令行参数
2. `MIHOMO_SECRET` 环境变量
3. `~/.config/mihomo/mihomo.env` 中的 `MIHOMO_SECRET`（与 systemd 用户服务共享的环境文件）

安全约定：

- `MIHOMO_SECRET` 绝不会出现在日志或表格输出中。
- `~/.config/mihomo/` 下的文件多为机密或敏感数据（`mihomo.env` 含鉴权密钥、`subscription.url` 含订阅 token），请勿提交到仓库或分享给他人。
- 订阅更新脚本使用 `mihomo -t` 校验通过后才替换 `config.yaml`，CLI 的 `sub` 相关命令不绕过这一安全步骤。

## 退出码

| 退出码 | 含义 |
|---|---|
| 0 | 成功 |
| 1 | 任何错误（连接失败、API 报错、校验不通过等），错误信息打印到 stderr |

## 故障排查

- `无法连接 mihomo API`：服务未运行，用 `mihomo-cli service status` 查看、`mihomo-cli service restart` 或 `systemctl --user start mihomo` 启动。
- `策略组 xxx 不存在`：用 `mihomo-cli group list` 查看实际策略组名（区分大小写，含中文/emoji 的名称需加引号）。
- `等待 10 秒后 API 仍未恢复`：重启后未起来，用 `mihomo-cli logs` 或 `journalctl --user -u mihomo` 查看服务日志。
- `查询最新版本失败`：GitHub 直连与代理均失败，检查本机代理端口 7890 是否可用。
- secret 未配置时如果 API 返回 401，确认 `~/.config/mihomo/mihomo.env` 中 `MIHOMO_SECRET` 与 `config.yaml` 的 `external-controller` 密钥一致。

## 开发

```bash
cargo build
cargo test                        # 单元测试（纯函数：字节格式化/环境文件解析/URL 拼接/版本比较）
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --release --locked
```

代码结构：

```text
src/main.rs      入口：解析 CLI、分发子命令
src/cli.rs       clap 派生宏定义的全部子命令/参数
src/api.rs       ApiClient：REST API 薄封装（鉴权/编码/错误中文化）
src/env.rs       secret 解析（命令行/环境变量/mihomo.env 文件回退）
src/models.rs    API 响应的 serde 模型
src/cmds/        每个子命令一个模块（status/proxy/group/rule/conn/logs/mode/config/service/sub/version/upgrade）
```

新增子命令的惯例：在 `cli.rs` 的 `Command` 枚举加变体并写中文 doc 注释 → 在 `cmds/` 下建同名模块 → 在 `main.rs` 的 match 中分发。
