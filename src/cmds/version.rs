//! `version` 子命令：显示 mihomo 当前版本，并查询 GitHub 最新 release 判断是否需要升级。
//!
//! 查询逻辑见 `crate::github`（GitHub API → 302 跳转，直连失败回退本机 7890 代理）。

use crate::api::ApiClient;
use crate::github::{latest_release_tag, normalize_version};
use crate::models::Version;
use crate::ui;
use anyhow::Result;

const REPO: &str = "MetaCubeX/mihomo";

pub async fn run(client: &ApiClient) -> Result<()> {
    let current: Version = serde_json::from_value(client.get(&["version"], &[]).await?)?;

    let mut items: Vec<(&str, String)> = vec![("current", ui::bold(&current.version))];
    match latest_release_tag(REPO).await {
        Ok(latest) => {
            let up_to_date = normalize_version(&current.version) >= normalize_version(&latest);
            items.push((
                "latest",
                if up_to_date {
                    ui::green(&latest)
                } else {
                    ui::yellow(&latest)
                },
            ));
            let summary = if up_to_date {
                ui::green("已是最新版本")
            } else {
                ui::yellow("有新版本可用，运行 `mihomo-cli upgrade` 升级内核")
            };
            ui::kv_panel("Version", "mihomo core", &items, Some(&summary));
        }
        Err(e) => {
            ui::kv_panel(
                "Version",
                "mihomo core",
                &items,
                Some(&ui::red(&format!("查询最新版本失败: {e}"))),
            );
        }
    }
    Ok(())
}
