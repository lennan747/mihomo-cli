use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::process::Command;

fn url_file() -> PathBuf {
    let home = std::env::var_os("HOME").unwrap_or_default();
    PathBuf::from(home).join(".config/mihomo/subscription.url")
}

fn update_script() -> PathBuf {
    let home = std::env::var_os("HOME").unwrap_or_default();
    PathBuf::from(home).join(".local/bin/mihomo-update")
}

/// 拉取订阅、校验并热更新配置（与 mihomo-update.timer 走同一脚本）
pub fn run() -> Result<()> {
    let script = update_script();
    if !script.exists() {
        bail!("未找到订阅更新脚本: {}", script.display());
    }
    println!("执行 {} ...", script.display());
    let status = Command::new("bash")
        .arg(&script)
        .status()
        .with_context(|| format!("运行 {} 失败", script.display()))?;
    if !status.success() {
        bail!("订阅更新失败，退出码: {status}");
    }
    Ok(())
}

/// 显示当前订阅地址：优先订阅文件，其次从更新脚本中提取
pub fn url() -> Result<()> {
    let file = url_file();
    if let Ok(u) = std::fs::read_to_string(&file) {
        let u = u.trim();
        if !u.is_empty() {
            println!("订阅地址: {u}\n（来源: {}）", file.display());
            return Ok(());
        }
    }
    let script = std::fs::read_to_string(update_script()).unwrap_or_default();
    for line in script.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("url=\"http") {
            if let Some(u) = rest.strip_suffix('"') {
                println!("订阅地址: http{u}\n（来源: 更新脚本内默认值，未通过 mihomo-cli 设置）");
                return Ok(());
            }
        }
    }
    bail!("未找到订阅地址，请用 `mihomo-cli sub set-url <URL>` 设置");
}

/// 设置订阅地址，写入订阅文件（定时任务 mihomo-update.timer 同样读取此文件）
pub fn set_url(u: &str) -> Result<()> {
    if !(u.starts_with("http://") || u.starts_with("https://")) {
        bail!("订阅地址必须是 http:// 或 https:// URL: {u}");
    }
    let file = url_file();
    std::fs::write(&file, format!("{u}\n"))
        .with_context(|| format!("写入 {} 失败", file.display()))?;
    println!("订阅地址已保存: {u}");
    println!(
        "提示: 每小时的 mihomo-update.timer 也会使用此地址；立即更新请运行 `mihomo-cli sub update`"
    );
    Ok(())
}
