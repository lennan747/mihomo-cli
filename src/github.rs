//! GitHub Release 查询与版本号比较的共享工具。
//!
//! 查询走两条路：GitHub API（未认证可能被限流）→ `releases/latest` 页面的 302 跳转目标；
//! 直连失败时回退本机混合代理端口 7890。被 `version`（mihomo 内核）与 `self-upgrade`（客户端自身）复用。

use anyhow::{Context, Result};
use serde_json::Value;

/// 构造查询 GitHub 用的 HTTP 客户端；`proxy` 为 `None` 时直连。
///
/// 为便于从 302 跳转解析 tag，此处禁用自动重定向；下载资产请另建跟随重定向的客户端。
fn http_client(proxy: Option<&str>) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .user_agent("mihomo-cli")
        .redirect(reqwest::redirect::Policy::none());
    if let Some(p) = proxy {
        builder = builder.proxy(reqwest::Proxy::all(p).context("无效的代理地址")?);
    }
    Ok(builder.build()?)
}

async fn try_fetch(proxy: Option<&str>, repo: &str) -> Result<String> {
    let client = http_client(proxy)?;
    // 首选 GitHub API（可能因未认证限流返回 403）
    let api = format!("https://api.github.com/repos/{repo}/releases/latest");
    match client.get(&api).send().await {
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
    let page = format!("https://github.com/{repo}/releases/latest");
    let resp = client
        .get(&page)
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

/// 查询指定仓库（`owner/name`）最新 release 的 tag；直连失败时回退到本机混合代理端口 7890
pub async fn latest_release_tag(repo: &str) -> Result<String> {
    match try_fetch(None, repo).await {
        Ok(v) => Ok(v),
        Err(direct_err) => match try_fetch(Some("http://127.0.0.1:7890"), repo).await {
            Ok(v) => Ok(v),
            Err(proxy_err) => {
                anyhow::bail!("直连失败（{direct_err}）；经代理 7890 也失败（{proxy_err}）")
            }
        },
    }
}

/// 把 `vX.Y.Z[-suffix]` 解析为可比较的数字段，忽略非数字后缀与缺失段
pub fn normalize_version(v: &str) -> Vec<u64> {
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

#[cfg(test)]
mod tests {
    use super::normalize_version;

    #[test]
    fn 去掉_v_前缀并按段解析() {
        assert_eq!(normalize_version("v1.18.9"), vec![1, 18, 9]);
        assert_eq!(normalize_version("1.18.9"), vec![1, 18, 9]);
    }

    #[test]
    fn 忽略非数字后缀() {
        // mihomo 的 Alpha 版本号形如 v1.19.2-alpha
        assert_eq!(normalize_version("v1.19.2-alpha"), vec![1, 19, 2]);
    }

    #[test]
    fn 版本比较() {
        assert!(normalize_version("v1.19.0") > normalize_version("v1.18.9"));
        assert!(normalize_version("v1.19.0") >= normalize_version("v1.19.0"));
        assert!(normalize_version("v1.19") < normalize_version("v1.19.1"));
        // 同主版本时短版本视为更小：v1.19 < v1.19.0
        assert!(normalize_version("v1.19") < normalize_version("v1.19.0"));
    }
}
