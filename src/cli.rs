//! 命令行接口定义（clap derive 模式）。
//!
//! 全局参数 `--api` / `--secret` 支持环境变量回退（`MIHOMO_API` / `MIHOMO_SECRET`）。
//! 新增子命令的流程：在 `Command` 枚举加变体并写中文 doc 注释 → 在 `cmds/` 下建同名模块 → 在 `main.rs` 的 match 中分发。

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    version,
    about = "mihomo-cli — 管理本机 mihomo (Clash Meta) 实例",
    long_about = "通过 mihomo 外部控制器 REST API (默认 127.0.0.1:9090) 与 systemd 用户服务进行管理"
)]
pub struct Cli {
    /// mihomo 外部控制器地址
    #[arg(
        long,
        global = true,
        default_value = "http://127.0.0.1:9090",
        env = "MIHOMO_API"
    )]
    pub api: String,

    /// API secret（未提供时读取 ~/.config/mihomo/mihomo.env）
    #[arg(short, long, global = true, env = "MIHOMO_SECRET")]
    pub secret: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// 运行状态总览：版本、模式、端口、各策略组当前选中
    Status,

    /// 节点操作：列出 / 测速 / 切换
    Proxy {
        #[command(subcommand)]
        action: ProxyAction,
    },

    /// 策略组操作
    Group {
        #[command(subcommand)]
        action: GroupAction,
    },

    /// 分流规则
    Rule {
        #[command(subcommand)]
        action: RuleAction,
    },

    /// 活动连接
    Conn {
        #[command(subcommand)]
        action: ConnAction,
    },

    /// 实时跟踪 mihomo 日志（Ctrl-C 退出）
    Logs {
        /// 日志级别: info / warning / error / debug / silent
        #[arg(long, default_value = "info")]
        level: String,
    },

    /// 查看或切换运行模式
    Mode {
        /// 不提供时显示当前模式
        mode: Option<ModeArg>,
    },

    /// 配置操作
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// 管理 systemd 用户服务 mihomo.service
    Service {
        #[command(subcommand)]
        action: ServiceAction,
    },

    /// 订阅管理：查看/设置订阅地址，拉取最新节点
    Sub {
        #[command(subcommand)]
        action: SubAction,
    },

    /// 查看 mihomo 当前版本并检查是否有新版本
    Version,

    /// 在线升级 mihomo 内核（调用 /upgrade 自升级端点）
    Upgrade,
}

#[derive(Subcommand)]
pub enum ProxyAction {
    /// 列出节点；指定策略组名则列出该组节点，否则列出全部节点
    List { group: Option<String> },

    /// 延迟测试并排序；指定策略组名只测该组，否则测全部节点
    Test {
        group: Option<String>,
        /// 超时时间（毫秒）
        #[arg(long, default_value_t = 5000)]
        timeout: u64,
    },

    /// 切换策略组的选中节点
    Select { group: String, name: String },

    /// 从订阅拉取最新节点并热更新（同 sub update）
    Update,
}

#[derive(Subcommand)]
pub enum GroupAction {
    /// 列出策略组、类型与当前选中
    List,
}

#[derive(Subcommand)]
pub enum RuleAction {
    /// 列出分流规则
    List,
}

#[derive(Subcommand)]
pub enum ConnAction {
    /// 列出活动连接
    List {
        /// 最多显示条数
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },

    /// 关闭连接
    Close {
        /// 关闭全部活动连接
        #[arg(long)]
        all: bool,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// 热重载 ~/.config/mihomo/config.yaml
    Reload,
}

#[derive(Subcommand)]
pub enum ServiceAction {
    /// 重启 mihomo 服务并等待 API 恢复
    Restart,

    /// 查看 mihomo 服务状态
    Status,
}

#[derive(Subcommand)]
pub enum SubAction {
    /// 拉取订阅并热更新配置
    Update,

    /// 查看当前订阅地址
    Url,

    /// 设置订阅地址（写入 ~/.config/mihomo/subscription.url，每小时定时更新同样生效）
    SetUrl { url: String },
}

#[derive(Clone, ValueEnum)]
pub enum ModeArg {
    Rule,
    Global,
    Direct,
}

impl ModeArg {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModeArg::Rule => "rule",
            ModeArg::Global => "global",
            ModeArg::Direct => "direct",
        }
    }
}
