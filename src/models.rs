//! mihomo REST API 响应的 serde 反序列化模型。
//!
//! 字段与 mihomo 外部控制器返回的 JSON 一一对应；API 返回的 camelCase 字段通过
//! `#[serde(rename)]` 映射为 snake_case。列表/详情字段大量可选，缺失时降级为 `-`。

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct Version {
    pub version: String,
    #[serde(default)]
    pub meta: bool,
}

#[derive(Deserialize)]
pub struct Configs {
    pub port: Option<u16>,
    #[serde(rename = "socks-port")]
    pub socks_port: Option<u16>,
    #[serde(rename = "redir-port")]
    pub redir_port: Option<u16>,
    #[serde(rename = "tproxy-port")]
    pub tproxy_port: Option<u16>,
    #[serde(rename = "mixed-port")]
    pub mixed_port: Option<u16>,
    pub mode: String,
    #[serde(rename = "log-level")]
    pub log_level: String,
    #[serde(rename = "allow-lan")]
    pub allow_lan: bool,
}

#[derive(Deserialize, Clone)]
pub struct HistoryItem {
    pub delay: Option<u64>,
}

#[derive(Deserialize, Clone)]
pub struct Proxy {
    pub name: String,
    #[serde(rename = "type")]
    pub ptype: String,
    #[serde(default)]
    pub now: Option<String>,
    #[serde(default)]
    pub all: Option<Vec<String>>,
    #[serde(default)]
    pub history: Vec<HistoryItem>,
}

impl Proxy {
    /// 组类型（Selector/Fallback/URLTest 等）的节点带有 all 列表
    pub fn is_group(&self) -> bool {
        self.all.is_some()
    }

    pub fn last_delay(&self) -> Option<u64> {
        self.history.last().and_then(|h| h.delay).filter(|d| *d > 0)
    }
}

#[derive(Deserialize)]
pub struct ProxiesResp {
    pub proxies: HashMap<String, Proxy>,
}

/// /providers/proxies 中的 provider：vehicleType 为 HTTP/File/Inline；
/// 内置的 Compatible provider 是顶层节点副本，使用方需自行跳过
#[derive(Deserialize)]
pub struct Provider {
    #[serde(rename = "vehicleType")]
    pub vehicle_type: String,
    #[serde(default)]
    pub proxies: Vec<Proxy>,
}

#[derive(Deserialize)]
pub struct ProvidersResp {
    pub providers: HashMap<String, Provider>,
}

#[derive(Deserialize)]
pub struct Rule {
    #[serde(rename = "type")]
    pub rtype: String,
    pub payload: String,
    pub proxy: String,
}

#[derive(Deserialize)]
pub struct RulesResp {
    pub rules: Vec<Rule>,
}

#[derive(Deserialize)]
pub struct ConnectionMeta {
    pub network: Option<String>,
    #[serde(rename = "type")]
    pub ctype: Option<String>,
    pub host: Option<String>,
    #[serde(rename = "destinationIP")]
    pub destination_ip: Option<String>,
    #[serde(rename = "destinationPort")]
    pub destination_port: Option<String>,
}

#[derive(Deserialize)]
pub struct Connection {
    pub metadata: ConnectionMeta,
    #[serde(default)]
    pub chains: Vec<String>,
    pub rule: Option<String>,
    #[serde(default)]
    pub download: u64,
    #[serde(default)]
    pub upload: u64,
}

#[derive(Deserialize)]
pub struct ConnectionsResp {
    pub connections: Option<Vec<Connection>>,
    #[serde(rename = "downloadTotal")]
    pub download_total: u64,
    #[serde(rename = "uploadTotal")]
    pub upload_total: u64,
}
