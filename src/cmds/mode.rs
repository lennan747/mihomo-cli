//! `mode` 子命令：查看或切换运行模式（rule/global/direct）。
//!
//! 切换经 PATCH `/configs` 热生效，不写配置文件（重启后回落到 config.yaml 的值）。

use crate::api::ApiClient;
use crate::cli::ModeArg;
use crate::models::Configs;
use crate::ui;
use anyhow::Result;
use serde_json::json;

/// 运行模式着色：direct 红 / global 黄 / rule 绿
fn mode_colored(mode: &str) -> String {
    match mode {
        "global" => ui::yellow(mode),
        "direct" => ui::red(mode),
        _ => ui::green(mode),
    }
}

pub async fn run(client: &ApiClient, mode: Option<ModeArg>) -> Result<()> {
    match mode {
        None => {
            let configs: Configs = serde_json::from_value(client.get(&["configs"], &[]).await?)?;
            ui::status_bar(&[
                ui::bold("mihomo-cli mode"),
                format!("{} {}", ui::dim("current ="), mode_colored(&configs.mode)),
            ]);
        }
        Some(m) => {
            client
                .patch(&["configs"], json!({ "mode": m.as_str() }))
                .await?;
            println!(
                "{}",
                ui::green(&format!("✓ 已切换模式: {}", mode_colored(m.as_str())))
            );
        }
    }
    Ok(())
}
