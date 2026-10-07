//! `version` 子命令：显示 mihomo 当前版本，并查询 GitHub 最新 release 判断是否需要升级。
//!
//! 查询逻辑见 `crate::github`（GitHub API → 302 跳转，直连失败回退本机 7890 代理）。

use crate::api::ApiClient;
use crate::github::{latest_release_tag, normalize_version};
use crate::models::Version;
use anyhow::Result;

const REPO: &str = "MetaCubeX/mihomo";

pub async fn run(client: &ApiClient) -> Result<()> {
    let current: Version = serde_json::from_value(client.get(&["version"], &[]).await?)?;
    println!("当前版本: {}", current.version);

    match latest_release_tag(REPO).await {
        Ok(latest) => {
            println!("最新版本: {latest}");
            if normalize_version(&current.version) >= normalize_version(&latest) {
                println!("已是最新版本");
            } else {
                println!("有新版本可用，运行 `mihomo-cli upgrade` 升级");
            }
        }
        Err(e) => println!("查询最新版本失败: {e}"),
    }
    Ok(())
}
