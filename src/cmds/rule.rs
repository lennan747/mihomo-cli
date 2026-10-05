use crate::api::ApiClient;
use crate::models::RulesResp;
use anyhow::Result;
use comfy_table::{presets::UTF8_FULL_CONDENSED, Table};

pub async fn run(client: &ApiClient) -> Result<()> {
    let rules: RulesResp = serde_json::from_value(client.get(&["rules"], &[]).await?)?;

    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(["#", "类型", "匹配内容", "去向"]);
    for (i, r) in rules.rules.iter().enumerate() {
        table.add_row([&(i + 1).to_string(), &r.rtype, &r.payload, &r.proxy]);
    }
    println!("{table}");
    Ok(())
}
