//! `conn` 子命令：活动连接的查看与关闭。
//!
//! 关闭连接影响面大，`close` 必须显式携带 `--all` 才会执行。

use crate::api::ApiClient;
use crate::cli::ConnAction;
use crate::cmds::human_bytes;
use crate::models::{Connection, ConnectionsResp};
use anyhow::{bail, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Table};

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
    let chain = c.chains.join(" -> ");
    [
        target,
        net,
        c.rule.clone().unwrap_or_else(|| "-".into()),
        format!("↓{} ↑{}", human_bytes(c.download), human_bytes(c.upload)),
        chain,
    ]
}

async fn list(client: &ApiClient, limit: usize) -> Result<()> {
    let resp: ConnectionsResp = serde_json::from_value(client.get(&["connections"], &[]).await?)?;
    let conns = resp.connections.unwrap_or_default();
    println!(
        "活动连接: {} 条（累计 ↓{} ↑{}）",
        conns.len(),
        human_bytes(resp.download_total),
        human_bytes(resp.upload_total)
    );

    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(["目标", "网络", "规则", "流量", "链路"]);
    for c in conns.iter().take(limit) {
        table.add_row(conn_row(c));
    }
    println!("{table}");
    if conns.len() > limit {
        println!("（仅显示前 {limit} 条，共 {} 条）", conns.len());
    }
    Ok(())
}

async fn close(client: &ApiClient, all: bool) -> Result<()> {
    if !all {
        bail!("关闭连接影响范围较大，请明确指定 --all 关闭全部活动连接");
    }
    client.delete(&["connections"]).await?;
    println!("已关闭全部活动连接");
    Ok(())
}
