//! `status` 子命令：版本、运行模式、端口与各策略组当前选中的总览。

use crate::api::ApiClient;
use crate::models::{Configs, ProxiesResp, Version};
use anyhow::Result;
use comfy_table::{presets::UTF8_FULL_CONDENSED, Table};

pub async fn run(client: &ApiClient) -> Result<()> {
    let version: Version = serde_json::from_value(client.get(&["version"], &[]).await?)?;
    let configs: Configs = serde_json::from_value(client.get(&["configs"], &[]).await?)?;
    let proxies: ProxiesResp = serde_json::from_value(client.get(&["proxies"], &[]).await?)?;

    println!("mihomo {} (meta: {})", version.version, version.meta);
    println!("运行模式    : {}", configs.mode);
    println!("日志级别    : {}", configs.log_level);
    println!("允许局域网  : {}", configs.allow_lan);
    if let Some(p) = configs.mixed_port.filter(|p| *p > 0) {
        println!("混合端口    : {p}");
    }
    if let Some(p) = configs.socks_port.filter(|p| *p > 0) {
        println!("SOCKS 端口  : {p}");
    }
    if let Some(p) = configs.port.filter(|p| *p > 0) {
        println!("HTTP 端口   : {p}");
    }
    if let Some(p) = configs.redir_port.filter(|p| *p > 0) {
        println!("redir 端口  : {p}");
    }
    if let Some(p) = configs.tproxy_port.filter(|p| *p > 0) {
        println!("tproxy 端口 : {p}");
    }

    let mut groups: Vec<_> = proxies.proxies.values().filter(|p| p.is_group()).collect();
    groups.sort_by(|a, b| a.name.cmp(&b.name));

    println!("\n策略组（{} 个）:", groups.len());
    let mut table = Table::new();
    table.load_style(UTF8_FULL_CONDENSED);
    table.set_header(["策略组", "类型", "当前选中"]);
    for g in groups {
        table.add_row([
            g.name.as_str(),
            g.ptype.as_str(),
            g.now.as_deref().unwrap_or("-"),
        ]);
    }
    println!("{table}");
    Ok(())
}
