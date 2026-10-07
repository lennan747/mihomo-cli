//! `rule` 子命令：列出分流规则（按配置文件中的顺序编号展示）。

use crate::api::ApiClient;
use crate::cmds::histogram;
use crate::models::RulesResp;
use crate::ui::{self, Column};
use anyhow::Result;

pub async fn run(client: &ApiClient) -> Result<()> {
    let rules: RulesResp = serde_json::from_value(client.get(&["rules"], &[]).await?)?;

    let rows: Vec<Vec<String>> = rules
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| {
            vec![
                ui::dim(&(i + 1).to_string()),
                ui::dim(&r.rtype),
                r.payload.clone(),
                r.proxy.clone(),
            ]
        })
        .collect();
    let columns = [
        Column::new("#", 6),
        Column::new("type", 14),
        Column::flex("payload", 24),
        Column::new("target", 20),
    ];
    let summary = histogram(rules.rules.iter().map(|r| r.rtype.as_str()));
    ui::table_dashboard(
        "Rules",
        &format!("shown={}", rules.rules.len()),
        &columns,
        &rows,
        Some(&summary),
    );
    Ok(())
}
