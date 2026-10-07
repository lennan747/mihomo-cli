//! mihomo-cli 入口：解析命令行、解析 secret、构造 API 客户端，并分发子命令。
//!
//! 各子命令的实现见 `cmds/` 模块；命令行定义见 `cli.rs`。

mod api;
mod cli;
mod cmds;
mod env;
mod github;
mod models;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command, ConfigAction, GroupAction, RuleAction, SubAction};

/// 恢复 `SIGPIPE` 的默认处置。
///
/// Rust 启动时会忽略 `SIGPIPE`，于是输出被下游提前关闭（如 `mihomo-cli rule list | head`）时
/// `println!` 会因 `EPIPE` panic 并打印 backtrace。恢复默认后进程会像普通 Unix 程序一样被
/// `SIGPIPE` 静默终止，`head`/`less` 等下游提前退出不再产生噪音。
#[cfg(unix)]
fn restore_sigpipe() {
    // SAFETY: 仅在进程启动阶段把 SIGPIPE 的处置设为默认值，不改动其他信号。
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_sigpipe() {}

#[tokio::main]
async fn main() {
    restore_sigpipe();
    // 统一错误出口：anyhow 链式上下文按 "错误: 原因1: 原因2" 打印，退出码固定 1
    if let Err(e) = run().await {
        eprintln!("错误: {e:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args = Cli::parse();
    let secret = env::resolve_secret(args.secret);
    let client = api::ApiClient::new(&args.api, secret)?;

    match args.command {
        Command::Status => cmds::status::run(&client).await,
        Command::Proxy { action } => cmds::proxy::run(&client, action).await,
        Command::Group {
            action: GroupAction::List,
        } => cmds::group::run(&client).await,
        Command::Rule {
            action: RuleAction::List,
        } => cmds::rule::run(&client).await,
        Command::Conn { action } => cmds::conn::run(&client, action).await,
        Command::Logs { level } => cmds::logs::run(&client, &level).await,
        Command::Mode { mode } => cmds::mode::run(&client, mode).await,
        Command::Config {
            action: ConfigAction::Reload,
        } => cmds::config::run(&client).await,
        Command::Service { action } => cmds::service::run(&client, action).await,
        Command::Sub { action } => match action {
            SubAction::Update => cmds::sub::run(),
            SubAction::Url => cmds::sub::url(),
            SubAction::SetUrl { url } => cmds::sub::set_url(&url),
        },
        Command::Version => cmds::version::run(&client).await,
        Command::Upgrade => cmds::upgrade::run(&client).await,
        Command::SelfUpgrade {
            check,
            version,
            force,
        } => cmds::self_upgrade::run(check, version, force).await,
    }
}
