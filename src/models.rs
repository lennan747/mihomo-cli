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
