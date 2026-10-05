//! `upgrade` 子命令：调用 `/upgrade` 端点让 mihomo 自升级内核并原地重启。

use crate::api::ApiClient;
use crate::models::Version;
use anyhow::Result;

pub async fn run(client: &ApiClient) -> Result<()> {
    let before: Version = serde_json::from_value(client.get(&["version"], &[]).await?)?;
    println!("当前版本: {}，开始升级...", before.version);

    // /upgrade 会下载新内核并原地重启进程，连接中断属于正常现象；
    // 已是最新时返回 500 "already using latest version"
    match client.post(&["upgrade"], &[]).await {
        Ok(_) => println!("升级请求已发送"),
        Err(e) if e.to_string().contains("already using latest") => {
            println!("已是最新版本 ({})", before.version);
            return Ok(());
        }
        Err(e) => println!("升级请求已发送（连接中断属正常: {e}）"),
    }

    println!("等待 mihomo 重启...");
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if let Ok(v) = client.get(&["version"], &[]).await {
            if let Ok(after) = serde_json::from_value::<Version>(v) {
                if after.version != before.version {
                    println!("升级完成: {} -> {}", before.version, after.version);
                } else {
                    println!("已是最新版本 ({})", after.version);
                }
                return Ok(());
            }
        }
    }
    anyhow::bail!("等待 20 秒后 API 仍未恢复，请用 `mihomo-cli service status` 排查")
}
