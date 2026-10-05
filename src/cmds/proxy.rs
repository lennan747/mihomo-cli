//! `proxy` 子命令：节点列表、延迟测试、切换选中节点。
//!
//! 延迟测试走 mihomo 的 `/group/{name}/delay`（组内整体测）与 `/proxies/{name}/delay`
//! （单节点测）端点；`update` 等价于 `sub update`（执行本机订阅更新脚本）。

use crate::api::ApiClient;
use crate::cli::ProxyAction;
use crate::models::{ProxiesResp, Proxy};
use anyhow::{bail, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Table};
use serde_json::json;

const TEST_URL: &str = "https://cp.cloudflare.com/generate_204";

pub async fn run(client: &ApiClient, action: ProxyAction) -> Result<()> {
    match action {
        ProxyAction::List { group } => list(client, group.as_deref()).await,
        ProxyAction::Test { group, timeout } => test(client, group.as_deref(), timeout).await,
        ProxyAction::Select { group, name } => select(client, &group, &name).await,
        ProxyAction::Update => crate::cmds::sub::run(),
    }
}

async fn fetch_proxies(client: &ApiClient) -> Result<ProxiesResp> {
    Ok(serde_json::from_value(
        client.get(&["proxies"], &[]).await?,
    )?)
}

fn group_names(proxies: &ProxiesResp) -> Vec<String> {
    let mut names: Vec<String> = proxies
        .proxies
        .values()
        .filter(|p| p.is_group())
        .map(|p| p.name.clone())
        .collect();
    names.sort();
    names
}

fn find_group<'a>(proxies: &'a ProxiesResp, name: &str) -> Result<&'a Proxy> {
    match proxies.proxies.get(name) {
        Some(p) if p.is_group() => Ok(p),
        Some(_) => bail!("{name} 不是策略组，是 {}", proxies.proxies[name].ptype),
        None => bail!(
            "策略组 {name} 不存在；可用策略组: {}",
            group_names(proxies).join(", ")
        ),
    }
}

fn print_proxy_table(rows: Vec<[String; 4]>) {
    let mut table = Table::new();
    table.load_style(UTF8_FULL_CONDENSED);
    table.set_header(["节点", "类型", "延迟", "选中"]);
    for r in rows {
        table.add_row(r);
    }
    println!("{table}");
}

async fn list(client: &ApiClient, group: Option<&str>) -> Result<()> {
    let proxies = fetch_proxies(client).await?;
    match group {
        Some(name) => {
            let g = find_group(&proxies, name)?;
            println!(
                "策略组 {name}（{} 个节点）:",
                g.all.as_ref().map_or(0, Vec::len)
            );
            let mut rows = Vec::new();
            for node in g.all.as_deref().unwrap_or(&[]) {
                let p = proxies.proxies.get(node);
                let (ptype, delay) = match p {
                    Some(p) => (
                        p.ptype.clone(),
                        p.last_delay()
                            .map_or_else(|| "-".into(), |d| format!("{d} ms")),
                    ),
                    None => ("?".into(), "-".into()),
                };
                let selected = if g.now.as_deref() == Some(node.as_str()) {
                    "★"
                } else {
                    ""
                };
                rows.push([node.clone(), ptype, delay, selected.into()]);
            }
            print_proxy_table(rows);
        }
        None => {
            let mut nodes: Vec<&Proxy> =
                proxies.proxies.values().filter(|p| !p.is_group()).collect();
            nodes.sort_by(|a, b| a.name.cmp(&b.name));
            println!("全部节点（{} 个）:", nodes.len());
            let rows = nodes
                .into_iter()
                .map(|p| {
                    [
                        p.name.clone(),
                        p.ptype.clone(),
                        p.last_delay()
                            .map_or_else(|| "-".into(), |d| format!("{d} ms")),
                        String::new(),
                    ]
                })
                .collect();
            print_proxy_table(rows);
        }
    }
    Ok(())
}

fn print_test_results(name: &str, results: Vec<(String, Option<u64>)>, footer: Option<String>) {
    let mut ok: Vec<_> = results.iter().filter(|(_, d)| d.is_some()).collect();
    ok.sort_by_key(|(_, d)| d.unwrap());
    let mut fail: Vec<_> = results.iter().filter(|(_, d)| d.is_none()).collect();
    fail.sort_by(|a, b| a.0.cmp(&b.0));

    let mut table = Table::new();
    table.load_style(UTF8_FULL_CONDENSED);
    table.set_header(["节点", "延迟"]);
    for (node, delay) in ok {
        table.add_row([node.as_str(), &format!("{} ms", delay.unwrap())]);
    }
    for (node, _) in fail {
        table.add_row([node.as_str(), "超时"]);
    }
    println!("{name}:\n{table}");
    if let Some(f) = footer {
        println!("{f}");
    }
}

async fn test(client: &ApiClient, group: Option<&str>, timeout: u64) -> Result<()> {
    let t = timeout.to_string();
    match group {
        Some(name) => {
            let proxies = fetch_proxies(client).await?;
            find_group(&proxies, name)?;
            // /group/{name}/delay 返回扁平 map: {节点名: 延迟毫秒数}，无响应的节点不出现
            let result = client
                .get(
                    &["group", name, "delay"],
                    &[("timeout", &t), ("url", TEST_URL)],
                )
                .await?;
            let map = result.as_object().cloned().unwrap_or_default();
            let results: Vec<(String, Option<u64>)> =
                map.into_iter().map(|(k, v)| (k, v.as_u64())).collect();
            let total = proxies.proxies[name].all.as_ref().map_or(0, Vec::len);
            let footer = (results.len() < total).then(|| {
                format!(
                    "（{}/{total} 个节点有响应，其余超时或不可用）",
                    results.len()
                )
            });
            print_test_results(
                &format!("策略组 {name} 延迟测试（超时 {timeout} ms）"),
                results,
                footer,
            );
        }
        None => {
            let proxies = fetch_proxies(client).await?;
            let nodes: Vec<String> = proxies
                .proxies
                .values()
                .filter(|p| !p.is_group())
                .map(|p| p.name.clone())
                .collect();

            println!(
                "全部 {} 个节点延迟测试（超时 {timeout} ms）...",
                nodes.len()
            );
            let mut set = tokio::task::JoinSet::new();
            for node in nodes {
                let c = client.clone();
                let t = t.clone();
                set.spawn(async move {
                    // /proxies/{name}/delay 返回 {delay: 毫秒数}
                    let r = c
                        .get(
                            &["proxies", &node, "delay"],
                            &[("timeout", &t), ("url", TEST_URL)],
                        )
                        .await;
                    (node, r.ok().and_then(|v| v["delay"].as_u64()))
                });
            }
            let mut results: Vec<(String, Option<u64>)> = Vec::new();
            while let Some(res) = set.join_next().await {
                results.push(res?);
            }
            print_test_results(
                &format!("全部节点延迟测试（超时 {timeout} ms）"),
                results,
                None,
            );
        }
    }
    Ok(())
}

async fn select(client: &ApiClient, group: &str, name: &str) -> Result<()> {
    let proxies = fetch_proxies(client).await?;
    let g = find_group(&proxies, group)?;
    let members = g.all.as_deref().unwrap_or(&[]);
    if !members.iter().any(|m| m == name) {
        bail!("节点 {name} 不在策略组 {group} 中（该组有 {} 个节点，用 `mihomo-cli proxy list {group}` 查看）", members.len());
    }
    client
        .put(&["proxies", group], &[], json!({ "name": name }))
        .await?;
    println!("已将 {group} 切换为: {name}");
    Ok(())
}
