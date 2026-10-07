//! `conn` 子命令：活动连接的查看与关闭。
//!
//! 关闭连接影响面大，`close` 必须显式携带 `--all` 才会执行。

use crate::api::ApiClient;
use crate::cli::ConnAction;
use crate::cmds::human_bytes;
use crate::models::{Connection, ConnectionsResp};
use crate::ui::{self, Column};
use anyhow::{bail, Result};

pub async fn run(client: &ApiClient, action: ConnAction) -> Result<()> {
    match action {
        ConnAction::List { limit } => list(client, limit).await,
        ConnAction::Close { all } => close(client, all).await,
    }
}

fn conn_row(c: &Connection) -> [String; 5] {
    let target = match (&c.metadata.host, &c.metadata.destination_ip) {
        (Some(h), _) => h.clone(),
        (None, Some(ip)) => match &c.metadata.destination_port {
            Some(p) => format!("{ip}:{p}"),
            None => ip.clone(),
        },
        (None, None) => "-".into(),
    };
    let net = match (&c.metadata.network, &c.metadata.ctype) {
        (Some(n), Some(t)) => format!("{n}/{t}"),
        (Some(n), None) => n.clone(),
        (None, Some(t)) => t.clone(),
        (None, None) => "-".into(),
    };
    [
        target,
        ui::dim(&net),
        ui::dim(c.rule.as_deref().unwrap_or("-")),
        format!("↓{} ↑{}", human_bytes(c.download), human_bytes(c.upload)),
        ui::dim(&c.chains.join(" -> ")),
    ]
}

async fn list(client: &ApiClient, limit: usize) -> Result<()> {
    let resp: ConnectionsResp = serde_json::from_value(client.get(&["connections"], &[]).await?)?;
    let conns = resp.connections.unwrap_or_default();

    let rows: Vec<Vec<String>> = conns
        .iter()
        .take(limit)
        .map(|c| conn_row(c).to_vec())
        .collect();
    let columns = [
        Column::flex("target", 24),
        Column::new("net", 9),
        Column::new("rule", 10),
        Column::new("traffic", 18),
        Column::new("chain", 18),
    ];
    let title_right = format!(
        "shown={} / total={} · ↓{} ↑{}",
        rows.len(),
        conns.len(),
        human_bytes(resp.download_total),
        human_bytes(resp.upload_total)
    );
    let hint = (conns.len() > limit)
        .then(|| ui::dim(&format!("showing first {limit} of {}", conns.len())));
    ui::table_dashboard(
        "Connections",
        &title_right,
        &columns,
        &rows,
        hint.as_deref(),
    );
    Ok(())
}

async fn close(client: &ApiClient, all: bool) -> Result<()> {
    if !all {
        bail!("关闭连接影响范围较大，请明确指定 --all 关闭全部活动连接");
    }
    client.delete(&["connections"]).await?;
    println!("{}", ui::green("✓ 已关闭全部活动连接"));
    Ok(())
}
