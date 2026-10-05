//! `logs` 子命令：经 WebSocket 实时跟踪 mihomo 日志，Ctrl-C 退出。
//!
//! ws/wss 地址由 `--api` 的 http/https 地址换 scheme 得来，鉴权复用 Bearer secret。

use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use serde_json::Value;
use tokio_tungstenite::{connect_async, tungstenite::client::IntoClientRequest};

use crate::api::ApiClient;

pub async fn run(client: &ApiClient, level: &str) -> Result<()> {
    let mut url = client.base_url().clone();
    match url.scheme() {
        "http" => url.set_scheme("ws").expect("http -> ws"),
        "https" => url.set_scheme("wss").expect("https -> wss"),
        s => bail!("不支持的协议 {s}（日志需要 http/https API 地址）"),
    }
    url.path_segments_mut().expect("API URL").push("logs");
    url.query_pairs_mut().append_pair("level", level).finish();

    let mut request = url
        .as_str()
        .into_client_request()
        .context("构建日志请求失败")?;
    if let Some(secret) = client.secret() {
        request.headers_mut().insert(
            "Authorization",
            format!("Bearer {secret}")
                .parse()
                .context("无效的 secret")?,
        );
    }

    let (mut ws, _) = connect_async(request)
        .await
        .context("连接日志流失败（服务未运行？）")?;

    println!("开始跟踪日志（级别: {level}），Ctrl-C 退出");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\n已退出");
                return Ok(());
            }
            msg = ws.next() => {
                match msg {
                    Some(Ok(msg)) => match msg {
                        tokio_tungstenite::tungstenite::protocol::Message::Text(text) => {
                            match serde_json::from_str::<Value>(&text) {
                                Ok(v) => {
                                    let lvl = v["type"].as_str().unwrap_or("");
                                    let payload = v["payload"].as_str().unwrap_or(&text);
                                    println!("[{lvl}] {payload}");
                                }
                                Err(_) => println!("{text}"),
                            }
                        }
                        tokio_tungstenite::tungstenite::protocol::Message::Close(_) => {
                            println!("日志流已关闭");
                            return Ok(());
                        }
                        _ => {}
                    },
                    None => {
                        println!("日志流已关闭");
                        return Ok(());
                    }
                    Some(Err(e)) => bail!("日志流错误: {e}"),
                }
            }
        }
    }
}
