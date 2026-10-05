//! `mode` 子命令：查看或切换运行模式（rule/global/direct）。
//!
//! 切换经 PATCH `/configs` 热生效，不写配置文件（重启后回落到 config.yaml 的值）。

use crate::api::ApiClient;
use crate::cli::ModeArg;
use crate::models::Configs;
use anyhow::Result;
use serde_json::json;

pub async fn run(client: &ApiClient, mode: Option<ModeArg>) -> Result<()> {
    match mode {
        None => {
            let configs: Configs = serde_json::from_value(client.get(&["configs"], &[]).await?)?;
            println!("当前模式: {}", configs.mode);
        }
        Some(m) => {
            client
                .patch(&["configs"], json!({ "mode": m.as_str() }))
                .await?;
            println!("已切换模式: {}", m.as_str());
        }
    }
    Ok(())
}
