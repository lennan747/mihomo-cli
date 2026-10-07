//! `config` 子命令：热重载 `~/.config/mihomo/config.yaml`（PUT `/configs?force=true`）。

use crate::api::ApiClient;
use crate::ui;
use anyhow::{bail, Result};
use serde_json::json;

pub async fn run(client: &ApiClient) -> Result<()> {
    let home = std::env::var_os("HOME").unwrap_or_default();
    let path = std::path::Path::new(&home).join(".config/mihomo/config.yaml");
    if !path.exists() {
        bail!("配置文件不存在: {}", path.display());
    }
    client
        .put(&["configs"], &[("force", "true")], json!({ "path": path }))
        .await?;
    println!(
        "{}",
        ui::green(&format!("✓ 已热重载配置: {}", path.display()))
    );
    Ok(())
}
