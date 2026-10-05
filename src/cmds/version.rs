//! `version` 子命令：显示 mihomo 当前版本，并查询 GitHub 最新 release 判断是否需要升级。
//!
//! 查询走两条路：GitHub API（未认证可能被限流）→ releases/latest 页面的 302 跳转目标；
//! 直连失败时回退本机混合代理端口 7890。

use crate::api::ApiClient;
use crate::models::Version;
use anyhow::{Context, Result};
use serde_json::Value;

const LATEST_RELEASE_API: &str = "https://api.github.com/repos/MetaCubeX/mihomo/releases/latest";
const LATEST_RELEASE_PAGE: &str = "https://github.com/MetaCubeX/mihomo/releases/latest";

fn http_client(proxy: Option<&str>) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .user_agent("mihomo-cli")
        .redirect(reqwest::redirect::Policy::none());
    if let Some(p) = proxy {
        builder = builder.proxy(reqwest::Proxy::all(p).context("无效的代理地址")?);
    }
    Ok(builder.build()?)
}

async fn try_fetch(proxy: Option<&str>) -> Result<String> {
    let client = http_client(proxy)?;
    // 首选 GitHub API（可能因未认证限流返回 403）
    match client.get(LATEST_RELEASE_API).send().await {
        Ok(resp) if resp.status().is_success() => {
            let v: Value = resp.json().await.context("解析 GitHub 响应失败")?;
            return v["tag_name"]
                .as_str()
                .map(String::from)
                .context("GitHub 响应中没有 tag_name");
        }
        _ => {}
    }
    // 回退：releases/latest 的 302 跳转目标即最新 tag
    let resp = client
        .get(LATEST_RELEASE_PAGE)
        .send()
        .await
        .context("请求 GitHub releases 页面失败")?;
    let location = resp
        .headers()
        .get("location")
        .and_then(|h| h.to_str().ok())
        .context("releases 页面未返回跳转地址")?;
    location
        .rsplit("/releases/tag/")
        .next()
        .filter(|t| t.starts_with('v'))
        .map(String::from)
        .context("无法从跳转地址解析版本号")
}

/// 查询最新 release；直连失败时回退到本机混合代理端口 7890
async fn latest_release() -> Result<String> {
    match try_fetch(None).await {
        Ok(v) => Ok(v),
        Err(direct_err) => match try_fetch(Some("http://127.0.0.1:7890")).await {
            Ok(v) => Ok(v),
            Err(proxy_err) => {
                anyhow::bail!("直连失败（{direct_err}）；经代理 7890 也失败（{proxy_err}）")
            }
        },
    }
}

fn normalize(v: &str) -> Vec<u64> {
    v.trim_start_matches('v')
        .split('.')
        .filter_map(|p| {
            p.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .ok()
        })
        .collect()
}

pub async fn run(client: &ApiClient) -> Result<()> {
    let current: Version = serde_json::from_value(client.get(&["version"], &[]).await?)?;
    println!("当前版本: {}", current.version);

    match latest_release().await {
        Ok(latest) => {
            println!("最新版本: {latest}");
            if normalize(&current.version) >= normalize(&latest) {
                println!("已是最新版本");
            } else {
                println!("有新版本可用，运行 `mihomo-cli upgrade` 升级");
            }
        }
        Err(e) => println!("查询最新版本失败: {e}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn 去掉_v_前缀并按段解析() {
        assert_eq!(normalize("v1.18.9"), vec![1, 18, 9]);
        assert_eq!(normalize("1.18.9"), vec![1, 18, 9]);
    }

    #[test]
    fn 忽略非数字后缀() {
        // mihomo 的 Alpha 版本号形如 v1.19.2-alpha
        assert_eq!(normalize("v1.19.2-alpha"), vec![1, 19, 2]);
    }

    #[test]
    fn 版本比较() {
        assert!(normalize("v1.19.0") > normalize("v1.18.9"));
        assert!(normalize("v1.19.0") >= normalize("v1.19.0"));
        assert!(normalize("v1.19") < normalize("v1.19.1"));
        // 同主版本时短版本视为更小：v1.19 < v1.19.0
        assert!(normalize("v1.19") < normalize("v1.19.0"));
    }
}
