//! `proxy` 子命令：节点列表、延迟测试、切换选中节点。
//!
//! 延迟测试走 mihomo 的 `/group/{name}/delay`（组内整体测）与 `/proxies/{name}/delay`
//! （单节点测）端点；`update` 等价于 `sub update`（执行本机订阅更新脚本）。

use crate::api::ApiClient;
use crate::cli::ProxyAction;
use crate::cmds::histogram;
use crate::models::{ProxiesResp, Proxy};
use crate::ui::{self, Column};
use anyhow::{bail, Result};
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

/// 延迟单元格：<200ms 绿 / <500ms 黄 / 其余红，无数据灰
fn delay_cell(delay: Option<u64>) -> String {
    match delay {
        Some(d) if d < 200 => ui::green(&format!("{d} ms")),
        Some(d) if d < 500 => ui::yellow(&format!("{d} ms")),
        Some(d) => ui::red(&format!("{d} ms")),
        None => ui::dim("-"),
    }
}

fn selected_cell(is_selected: bool) -> String {
    if is_selected {
        ui::bold_green("★")
    } else {
        String::new()
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

/// 把用户输入解析为唯一名称：先精确匹配，再「唯一子串」匹配。
///
/// mihomo 的代理/组没有 ID，只能用名称寻址；名称多带 emoji 前缀（`🌐 国外流量`），
/// 纯前缀匹配对 `国外` 这类输入无效，故用子串。歧义或无匹配时报错并给出候选。
/// （大小写不敏感，便于用 `ai` 匹配 `🤖 AI平台`。）
fn resolve<'a, I>(query: &str, candidates: I, what: &str) -> Result<String>
where
    I: IntoIterator<Item = &'a str>,
{
    if query.is_empty() {
        bail!("{what}名不能为空");
    }
    let cands: Vec<&str> = candidates.into_iter().collect();
    if cands.contains(&query) {
        return Ok(query.to_string());
    }
    let q = query.to_lowercase();
    let mut hits: Vec<&str> = cands
        .iter()
        .copied()
        .filter(|c| c.to_lowercase().contains(&q))
        .collect();
    hits.sort_unstable();
    match hits.len() {
        0 => bail!("未找到包含 {query} 的{what}"),
        1 => Ok(hits[0].to_string()),
        _ => {
            let shown: Vec<&str> = hits.iter().take(8).copied().collect();
            let more = if hits.len() > shown.len() { " …" } else { "" };
            bail!(
                "{query} 匹配到多个{what}，请写得更具体：{}{more}",
                shown.join(" / ")
            )
        }
    }
}

/// 解析策略组名（对全部策略组做唯一子串匹配）
fn resolve_group(proxies: &ProxiesResp, query: &str) -> Result<String> {
    let groups = group_names(proxies);
    resolve(query, groups.iter().map(String::as_str), "策略组")
}

fn proxy_columns() -> [Column; 4] {
    [
        Column::flex("name", 28),
        Column::new("type", 12),
        Column::new("delay", 10),
        Column::new("sel", 3),
    ]
}

async fn list(client: &ApiClient, group: Option<&str>) -> Result<()> {
    let proxies = fetch_proxies(client).await?;
    match group {
        Some(name) => {
            let name = resolve_group(&proxies, name)?;
            let g = find_group(&proxies, &name)?;
            let nodes = g.all.as_deref().unwrap_or(&[]);
            let rows: Vec<Vec<String>> = nodes
                .iter()
                .map(|node| {
                    let (ptype, delay) = match proxies.proxies.get(node) {
                        Some(p) => (ui::dim(&p.ptype), delay_cell(p.last_delay())),
                        None => (ui::dim("?"), ui::dim("-")),
                    };
                    vec![
                        node.clone(),
                        ptype,
                        delay,
                        selected_cell(g.now.as_deref() == Some(node.as_str())),
                    ]
                })
                .collect();
            let summary = histogram(
                nodes
                    .iter()
                    .filter_map(|n| proxies.proxies.get(n))
                    .map(|p| p.ptype.as_str()),
            );
            ui::table_dashboard(
                "Proxies",
                &format!("group={name} · nodes={}", nodes.len()),
                &proxy_columns(),
                &rows,
                Some(&summary),
            );
        }
        None => {
            let mut nodes: Vec<&Proxy> =
                proxies.proxies.values().filter(|p| !p.is_group()).collect();
            nodes.sort_by(|a, b| a.name.cmp(&b.name));
            let rows: Vec<Vec<String>> = nodes
                .iter()
                .map(|p| {
                    vec![
                        p.name.clone(),
                        ui::dim(&p.ptype),
                        delay_cell(p.last_delay()),
                        String::new(),
                    ]
                })
                .collect();
            let summary = histogram(nodes.iter().map(|p| p.ptype.as_str()));
            ui::table_dashboard(
                "Proxies",
                &format!("scope=all · nodes={}", nodes.len()),
                &proxy_columns(),
                &rows,
                Some(&summary),
            );
        }
    }
    Ok(())
}

fn print_test_results(
    scope: &str,
    timeout: u64,
    results: Vec<(String, Option<u64>)>,
    total: usize,
) {
    let mut ok: Vec<&(String, Option<u64>)> = results.iter().filter(|(_, d)| d.is_some()).collect();
    ok.sort_by_key(|(_, d)| d.unwrap());
    let mut fail: Vec<&(String, Option<u64>)> =
        results.iter().filter(|(_, d)| d.is_none()).collect();
    fail.sort_by(|a, b| a.0.cmp(&b.0));

    let rows: Vec<Vec<String>> = ok
        .iter()
        .map(|(node, delay)| vec![node.clone(), delay_cell(*delay)])
        .chain(
            fail.iter()
                .map(|(node, _)| vec![node.clone(), ui::dim("timeout")]),
        )
        .collect();

    let title_right = format!(
        "{scope} · timeout={timeout} ms · shown={}/{total}",
        ok.len()
    );
    let summary = format!(
        "{} responded={}  {} timeout={}",
        ui::green("●"),
        ok.len(),
        ui::red("○"),
        fail.len()
    );
    let columns = [Column::flex("name", 28), Column::new("delay", 12)];
    ui::table_dashboard("Delay test", &title_right, &columns, &rows, Some(&summary));
}

async fn test(client: &ApiClient, group: Option<&str>, timeout: u64) -> Result<()> {
    let t = timeout.to_string();
    match group {
        Some(name) => {
            let proxies = fetch_proxies(client).await?;
            let name = resolve_group(&proxies, name)?;
            let g = find_group(&proxies, &name)?;
            let total = g.all.as_ref().map_or(0, Vec::len);
            // /group/{name}/delay 返回扁平 map: {节点名: 延迟毫秒数}，无响应的节点不出现
            let result = client
                .get(
                    &["group", name.as_str(), "delay"],
                    &[("timeout", &t), ("url", TEST_URL)],
                )
                .await?;
            let map = result.as_object().cloned().unwrap_or_default();
            let results: Vec<(String, Option<u64>)> =
                map.into_iter().map(|(k, v)| (k, v.as_u64())).collect();
            print_test_results(&format!("group={name}"), timeout, results, total);
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
                "{}",
                ui::dim(&format!(
                    "正在并发测试 {} 个节点（超时 {timeout} ms）...",
                    nodes.len()
                ))
            );
            let total = nodes.len();
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
            print_test_results("scope=all", timeout, results, total);
        }
    }
    Ok(())
}

async fn select(client: &ApiClient, group: &str, name: &str) -> Result<()> {
    let proxies = fetch_proxies(client).await?;
    let group = resolve_group(&proxies, group)?;
    let g = find_group(&proxies, &group)?;
    let members = g.all.as_deref().unwrap_or(&[]);
    let name = resolve(name, members.iter().map(String::as_str), "节点")?;
    client
        .put(&["proxies", group.as_str()], &[], json!({ "name": name }))
        .await?;
    println!("{}", ui::green(&format!("✓ 已将 {group} 切换为: {name}")));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::resolve;

    const GROUPS: [&str; 5] = [
        "GLOBAL",
        "🌐 国外流量",
        "🎬 国际流媒体",
        "🎬 大陆流媒体国际版",
        "🤖 AI平台",
    ];

    #[test]
    fn 精确匹配优先() {
        assert_eq!(resolve("GLOBAL", GROUPS, "策略组").unwrap(), "GLOBAL");
    }

    #[test]
    fn 唯一子串匹配() {
        // 名称带 emoji 前缀，纯前缀匹配对「国外」无效，子串可命中
        assert_eq!(resolve("国外", GROUPS, "策略组").unwrap(), "🌐 国外流量");
        // 大小写不敏感
        assert_eq!(resolve("ai", GROUPS, "策略组").unwrap(), "🤖 AI平台");
    }

    #[test]
    fn 歧义时报错并列出候选() {
        let err = resolve("流媒体", GROUPS, "策略组").unwrap_err().to_string();
        assert!(err.contains("匹配到多个"), "{err}");
        assert!(err.contains("大陆流媒体国际版"), "{err}");
    }

    #[test]
    fn 无匹配与空输入报错() {
        assert!(resolve("不存在", GROUPS, "策略组").is_err());
        assert!(resolve("", GROUPS, "策略组").is_err());
    }
}
