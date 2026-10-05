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
