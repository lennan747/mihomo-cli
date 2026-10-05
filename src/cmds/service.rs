//! `service` 子命令：管理 systemd 用户服务 `mihomo.service`。
//!
//! `restart` 重启后轮询 API（最长 10 秒）等待恢复，避免重启期间调用方误判服务失败。

use crate::api::ApiClient;
use crate::cli::ServiceAction;
use crate::models::Version;
use anyhow::{bail, Context, Result};
use std::process::Command;

fn systemctl(args: &[&str]) -> Result<()> {
    let status = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .status()
        .context("执行 systemctl --user 失败")?;
    if !status.success() {
        bail!("systemctl --user {} 退出码: {status}", args.join(" "));
    }
    Ok(())
}

pub async fn run(client: &ApiClient, action: ServiceAction) -> Result<()> {
    match action {
        ServiceAction::Restart => {
            systemctl(&["restart", "mihomo"])?;
            println!("已发送重启命令，等待 API 恢复...");
            for _ in 0..20 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if let Ok(v) = client.get(&["version"], &[]).await {
                    if let Ok(ver) = serde_json::from_value::<Version>(v) {
                        println!("mihomo 已恢复运行 ({})", ver.version);
                        return Ok(());
                    }
                }
            }
            bail!("等待 10 秒后 API 仍未恢复，请用 `mihomo-cli logs` 或 journalctl --user -u mihomo 排查");
        }
        ServiceAction::Status => {
            let output = Command::new("systemctl")
                .args(["--user", "status", "mihomo", "--no-pager"])
                .output()
                .context("执行 systemctl --user status 失败")?;
            print!("{}", String::from_utf8_lossy(&output.stdout));
            print!("{}", String::from_utf8_lossy(&output.stderr));
            if !output.status.success() {
                bail!("systemctl status 退出码: {}", output.status);
            }
            Ok(())
        }
    }
}
