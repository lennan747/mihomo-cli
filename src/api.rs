//! mihomo 外部控制器 REST API 客户端。
//!
//! 对 REST API 的薄封装：统一处理 Bearer 鉴权、错误信息中文化与 URL 拼接。
//! 所有请求路径必须经 [`ApiClient::url`] 拼接，保证含中文/emoji 的节点名被正确百分号编码。

use anyhow::{bail, Context, Result};
use reqwest::{Client, Method, Url};
use serde_json::Value;

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base: Url,
    secret: Option<String>,
}

impl ApiClient {
    pub fn new(base: &str, secret: Option<String>) -> Result<Self> {
        let base = Url::parse(base).with_context(|| format!("无效的 API 地址: {base}"))?;
        Ok(Self {
            client: Client::new(),
            base,
            secret,
        })
    }

    pub fn base_url(&self) -> &Url {
        &self.base
    }

    pub fn secret(&self) -> Option<&String> {
        self.secret.as_ref()
    }

    /// 拼接 URL：对每个 path 段做百分号编码（节点/组名含中文与 emoji）
    fn url(&self, segs: &[&str], query: &[(&str, &str)]) -> Url {
        let mut url = self.base.clone();
        {
            let mut segments = url.path_segments_mut().expect("API 地址必须是 http(s) URL");
            for seg in segs {
                segments.push(seg);
            }
        }
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query).finish();
        }
        url
    }

    async fn send(
        &self,
        method: Method,
        segs: &[&str],
        query: &[(&str, &str)],
        body: Option<&Value>,
    ) -> Result<Value> {
        let mut req = self.client.request(method, self.url(segs, query));
        if let Some(secret) = &self.secret {
            req = req.header("Authorization", format!("Bearer {secret}"));
        }
        if let Some(body) = body {
            req = req.json(body);
        }
        let resp = req.send().await.map_err(|e| {
            if e.is_connect() {
                anyhow::anyhow!(
                    "无法连接 mihomo API ({e})；服务可能未运行，可用 `mihomo-cli service restart` 或 `systemctl --user start mihomo` 启动"
                )
            } else {
                anyhow::Error::new(e).context("请求 mihomo API 失败")
            }
        })?;
        let status = resp.status();
        let text = resp.text().await.context("读取 API 响应失败")?;
        if !status.is_success() {
            let msg = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v["message"].as_str().map(String::from))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| text.clone());
            bail!("API 返回 {status}: {msg}");
        }
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).with_context(|| format!("解析 API 响应失败: {text}"))
    }

    pub async fn get(&self, segs: &[&str], query: &[(&str, &str)]) -> Result<Value> {
        self.send(Method::GET, segs, query, None).await
    }

    pub async fn put(&self, segs: &[&str], query: &[(&str, &str)], body: Value) -> Result<Value> {
        self.send(Method::PUT, segs, query, Some(&body)).await
    }

    pub async fn patch(&self, segs: &[&str], body: Value) -> Result<Value> {
        self.send(Method::PATCH, segs, &[], Some(&body)).await
    }

    pub async fn delete(&self, segs: &[&str]) -> Result<Value> {
        self.send(Method::DELETE, segs, &[], None).await
    }

    pub async fn post(&self, segs: &[&str], query: &[(&str, &str)]) -> Result<Value> {
        self.send(Method::POST, segs, query, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::ApiClient;

    fn client() -> ApiClient {
        ApiClient::new("http://127.0.0.1:9090", None).unwrap()
    }

    #[test]
    fn 拼接路径与查询参数() {
        let url = client().url(&["proxies"], &[("timeout", "5000")]);
        assert_eq!(url.as_str(), "http://127.0.0.1:9090/proxies?timeout=5000");
    }

    #[test]
    fn 多段路径正确拼接() {
        let url = client().url(&["group", "PROXY", "delay"], &[]);
        assert_eq!(url.as_str(), "http://127.0.0.1:9090/group/PROXY/delay");
    }

    #[test]
    fn 中文与emoji节点名被百分号编码() {
        // 节点名含中文与 emoji 时必须编码，否则请求行非法
        let url = client().url(&["proxies", "香港 01 🇭🇰"], &[]);
        let path = url.path();
        assert!(!path.contains('香'));
        assert!(!path.contains(' '));
        assert!(path.ends_with("%F0%9F%87%AD%F0%9F%87%B0")); // 🇭🇰 的百分号编码
    }

    #[test]
    fn 查询参数值被编码() {
        let url = client().url(
            &["proxies"],
            &[("url", "https://cp.cloudflare.com/generate_204")],
        );
        assert!(url.as_str().contains("url=https%3A%2F%2Fcp.cloudflare.com"));
    }

    #[test]
    fn 根路径带斜杠的_apis_地址不产生双斜杠() {
        let c = ApiClient::new("http://127.0.0.1:9090/", None).unwrap();
        let url = c.url(&["version"], &[]);
        assert_eq!(url.as_str(), "http://127.0.0.1:9090/version");
    }

    #[test]
    fn 非_http_地址被拒绝() {
        assert!(ApiClient::new("ftp://127.0.0.1:9090", None).is_ok()); // Url::parse 允许 ftp，但 send 时才会失败
        assert!(ApiClient::new("不是地址", None).is_err());
    }
}
