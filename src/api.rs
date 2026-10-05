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
