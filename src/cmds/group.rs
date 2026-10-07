//! `group` 子命令：列出策略组、类型、节点数与当前选中。

use crate::api::ApiClient;
use crate::cmds::histogram;
use crate::models::ProxiesResp;
use crate::ui::{self, Column};
use anyhow::Result;

pub async fn run(client: &ApiClient) -> Result<()> {
    let proxies: ProxiesResp = serde_json::from_value(client.get(&["proxies"], &[]).await?)?;
    let mut groups: Vec<_> = proxies.proxies.values().filter(|p| p.is_group()).collect();
    groups.sort_by(|a, b| a.name.cmp(&b.name));

    let rows: Vec<Vec<String>> = groups
        .iter()
        .map(|g| {
            vec![
                g.name.clone(),
                ui::dim(&g.ptype),
                g.all.as_ref().map_or(0, Vec::len).to_string(),
                match g.now.as_deref() {
                    Some(n) => n.to_string(),
                    None => ui::dim("-"),
                },
            ]
        })
        .collect();
    let columns = [
        Column::flex("name", 20),
        Column::new("type", 10),
        Column::new("nodes", 6),
        Column::new("selected", 30),
    ];
    let summary = histogram(groups.iter().map(|g| g.ptype.as_str()));
    ui::table_dashboard(
        "Groups",
        &format!("shown={}", groups.len()),
        &columns,
        &rows,
        Some(&summary),
    );
    Ok(())
}
