//! `group` 子命令：列出策略组、类型、节点数与当前选中。

use crate::api::ApiClient;
use crate::models::ProxiesResp;
use anyhow::Result;
use comfy_table::{presets::UTF8_FULL_CONDENSED, Table};

pub async fn run(client: &ApiClient) -> Result<()> {
    let proxies: ProxiesResp = serde_json::from_value(client.get(&["proxies"], &[]).await?)?;
    let mut groups: Vec<_> = proxies.proxies.values().filter(|p| p.is_group()).collect();
    groups.sort_by(|a, b| a.name.cmp(&b.name));

    let mut table = Table::new();
    table.load_style(UTF8_FULL_CONDENSED);
    table.set_header(["策略组", "类型", "节点数", "当前选中"]);
    for g in groups {
        table.add_row([
            g.name.as_str(),
            g.ptype.as_str(),
            &g.all.as_ref().map_or(0, Vec::len).to_string(),
            g.now.as_deref().unwrap_or("-"),
        ]);
    }
    println!("{table}");
    Ok(())
}
