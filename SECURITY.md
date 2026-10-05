# 安全策略

## 支持范围

| 版本 | 支持 |
|---|---|
| 最新版本（当前 `v0.1.0`） | ✅ 安全更新 |
| 历史版本 | ❌ 不支持，请升级到最新版 |

## 上报漏洞

如果你发现安全漏洞，请**不要**提交公开 Issue。按以下方式私密上报：

1. 通过 GitHub 的[私密安全通告](https://github.com/lennan747/mihomo-cli/security/advisories/new)（推荐）；或
2. 发送邮件至 [lennan747@gmail.com](mailto:lennan747@gmail.com)，标题注明 `[mihomo-cli security]`。

请在报告中包含：

- 影响版本
- 复现步骤或概念验证
- 潜在影响与利用条件

我们会尽快确认并回复，修复后按流程发布并在变更日志中致谢（除非你要求匿名）。

## 安全承诺

- 48 小时内确认收到报告。
- 修复前不公开细节；修复发布后披露。

## 安全设计

- `MIHOMO_SECRET` 按命令行参数 > 环境变量 > `mihomo.env` 文件的顺序解析，绝不打印到日志、表格或错误信息。
- 默认只连接本机外部控制器地址（`http://127.0.0.1:9090`）。
- 关闭全部活动连接（`conn close`）必须显式携带 `--all`，防止误操作。
- 订阅更新脚本以 `mihomo -t` 校验通过后才替换 `config.yaml`，CLI 不绕过该步骤。

## 已知限制

- 外部控制器若配置为非本机地址，API 流量为明文 HTTP（无 TLS），secret 可能被中间人截获；请保持 `external-controller` 绑定在 127.0.0.1。
- `mihomo.env` 与 `subscription.url` 为本机明文文件，依赖文件系统权限保护，请勿提交到仓库或分享。
