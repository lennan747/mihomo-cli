//! `status` 子命令：版本、运行模式、端口与各策略组当前选中的总览。

use crate::api::ApiClient;
use crate::models::{Configs, ProxiesResp, Version};
use crate::ui::{self, Column};
use anyhow::Result;

/// 运行模式着色：direct 红 / global 黄 / rule 绿
fn mode_colored(mode: &str) -> String {
    match mode {
        "global" => ui::yellow(mode),
        "direct" => ui::red(mode),
        _ => ui::green(mode),
    }
}

pub async fn run(client: &ApiClient) -> Result<()> {
    let version: Version = serde_json::from_value(client.get(&["version"], &[]).await?)?;
    let configs: Configs = serde_json::from_value(client.get(&["configs"], &[]).await?)?;
    let proxies: ProxiesResp = serde_json::from_value(client.get(&["proxies"], &[]).await?)?;

    let ports: Vec<(&str, u16)> = [
        ("mixed-port", configs.mixed_port),
        ("socks-port", configs.socks_port),
        ("http-port", configs.port),
        ("redir-port", configs.redir_port),
        ("tproxy-port", configs.tproxy_port),
    ]
    .into_iter()
    .filter_map(|(label, port)| port.filter(|p| *p > 0).map(|p| (label, p)))
    .collect();

    // 顶部状态条
    let mut bar = vec![
        format!("{} {}", mode_colored("●"), ui::bold("mihomo-cli status")),
        ui::dim(&format!("{} meta={}", version.version, version.meta)),
        mode_colored(&configs.mode),
    ];
    if let Some((_, p)) = ports.iter().find(|(l, _)| *l == "mixed-port") {
        bar.push(ui::dim(&format!("mixed={p}")));
    }
    ui::status_bar(&bar);

    // Status 面板
    let mut items: Vec<(&str, String)> = vec![
        ("mode", mode_colored(&configs.mode)),
        ("log-level", configs.log_level.clone()),
        ("allow-lan", configs.allow_lan.to_string()),
    ];
    items.extend(ports.iter().map(|(l, p)| (*l, p.to_string())));
    ui::kv_panel(
        "Status",
        &format!("mihomo {} · meta={}", version.version, version.meta),
        &items,
        None,
    );

    // Groups 表格
    let mut groups: Vec<_> = proxies.proxies.values().filter(|p| p.is_group()).collect();
    groups.sort_by(|a, b| a.name.cmp(&b.name));
    let rows: Vec<Vec<String>> = groups
        .iter()
        .map(|g| {
            vec![
                g.name.clone(),
                ui::dim(&g.ptype),
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
        Column::new("selected", 30),
    ];
    let summary = crate::cmds::histogram(groups.iter().map(|g| g.ptype.as_str()));
    ui::table_dashboard(
        "Groups",
        &format!("shown={}", groups.len()),
        &columns,
        &rows,
        Some(&summary),
    );
    Ok(())
}
