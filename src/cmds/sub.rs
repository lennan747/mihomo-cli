//! `sub` 子命令：订阅地址的查看与设置，以及手动触发订阅更新。
//!
//! 订阅更新委托给本机脚本 `~/.local/bin/mihomo-update`（curl 拉取 → sed 修正 →
//! `mihomo -t` 校验 → 热重载），与每小时执行的 `mihomo-update.timer` 走同一路径，
//! 保证经 CLI 更新与定时更新行为一致。

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ui;

/// 脱敏订阅地址：保留 scheme/host 与 path 目录前缀便于辨认，隐去最后一段与 query（token 所在）；
/// 无法解析的地址整体隐藏。
fn mask_url(u: &str) -> String {
    let suffix = format!("（已隐藏 token · 全长 {} 字符）", u.chars().count());
    let Ok(parsed) = reqwest::Url::parse(u) else {
        return format!("****{suffix}");
    };
    let mut out = format!(
        "{}://{}",
        parsed.scheme(),
        parsed.host_str().unwrap_or_default()
    );
    if let Some(port) = parsed.port() {
        out.push_str(&format!(":{port}"));
    }
    let path = parsed.path();
    match path.rfind('/') {
        // 有目录前缀：保留前缀、隐去最后一段
        Some(i) if i > 0 => out.push_str(&format!("{}/****", &path[..i])),
        // 只有一段（如 /token）：整段隐去
        _ if path.len() > 1 => out.push_str("/****"),
        _ => {}
    }
    if parsed.query().is_some() {
        out.push_str("?****");
    }
    format!("{out}{suffix}")
}

/// 写入订阅文件：Unix 下权限固定 0600（订阅地址含 token，属凭据，与 mihomo.env 同一约定）。
/// `OpenOptions::mode` 仅对新建文件生效，已存在的文件需再补一次 set_permissions。
#[cfg(unix)]
fn write_url_file(file: &Path, content: &str) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(file)
        .with_context(|| format!("写入 {} 失败", file.display()))?;
    f.write_all(content.as_bytes())
        .with_context(|| format!("写入 {} 失败", file.display()))?;
    std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("设置 {} 权限失败", file.display()))?;
    Ok(())
}

/// 非 Unix 平台无 umask 语义，依赖文件系统 ACL，直接写入即可
#[cfg(not(unix))]
fn write_url_file(file: &Path, content: &str) -> Result<()> {
    std::fs::write(file, content).with_context(|| format!("写入 {} 失败", file.display()))
}

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

/// 显示当前订阅地址：优先订阅文件，其次从更新脚本中提取。
/// 默认脱敏（订阅地址含 token，与 secret 一样不进输出）；`--reveal` 才完整回显。
pub fn url(reveal: bool) -> Result<()> {
    let render = |u: &str| -> String {
        if reveal {
            u.to_string()
        } else {
            mask_url(u)
        }
    };
    let file = url_file();
    if let Ok(u) = std::fs::read_to_string(&file) {
        let u = u.trim();
        if !u.is_empty() {
            println!("订阅地址: {}\n（来源: {}）", render(u), file.display());
            return Ok(());
        }
    }
    let script = std::fs::read_to_string(update_script()).unwrap_or_default();
    for line in script.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("url=\"http") {
            if let Some(u) = rest.strip_suffix('"') {
                println!(
                    "订阅地址: {}\n（来源: 更新脚本内默认值，未通过 mihomo-cli 设置）",
                    render(&format!("http{u}"))
                );
                return Ok(());
            }
        }
    }
    bail!("未找到订阅地址，请用 `mihomo-cli sub set-url <URL>` 设置");
}

/// 设置订阅地址，写入订阅文件（定时任务 mihomo-update.timer 同样读取此文件）
pub fn set_url(u: &str) -> Result<()> {
    if !(u.starts_with("http://") || u.starts_with("https://")) {
        bail!("订阅地址必须是 http:// 或 https:// URL: {}", mask_url(u));
    }
    let file = url_file();
    write_url_file(&file, &format!("{u}\n"))?;
    println!(
        "{}",
        ui::green(&format!("✓ 订阅地址已保存: {}", mask_url(u)))
    );
    println!(
        "提示: 每小时的 mihomo-update.timer 也会使用此地址；立即更新请运行 `mihomo-cli sub update`"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::mask_url;

    #[test]
    fn 脱敏保留域名与目录前缀() {
        let m = mask_url("https://example.com/sub/8f3a9c2e1b7f4a6d");
        assert!(m.starts_with("https://example.com/sub/****"), "{m}");
        assert!(!m.contains("8f3a9c2e1b7f4a6d"), "{m}");
    }

    #[test]
    fn 单段路径与_query_整体隐藏() {
        let m = mask_url("https://example.com/abcdef?token=xxx");
        assert!(m.starts_with("https://example.com/****?****"), "{m}");
        assert!(!m.contains("abcdef"), "{m}");
        assert!(!m.contains("token=xxx"), "{m}");
    }

    #[test]
    fn 无路径地址仅显示域名() {
        let m = mask_url("https://example.com");
        assert!(m.starts_with("https://example.com（"), "{m}");
        assert!(!m.contains("****"), "{m}");
    }

    #[test]
    fn 无法解析的地址整体隐藏() {
        assert!(mask_url("不是地址").starts_with("****"));
    }

    #[test]
    fn 保留端口() {
        let m = mask_url("http://example.com:8080/sub/token123");
        assert!(m.starts_with("http://example.com:8080/sub/****"), "{m}");
    }

    #[cfg(unix)]
    #[test]
    fn 写入文件权限固定为_0600() {
        use super::write_url_file;
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("mihomo-cli-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("subscription.url");

        write_url_file(&f, "https://example.com/sub/x\n").unwrap();
        let mode = std::fs::metadata(&f).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        // 已存在且权限过宽的文件，再次写入时补正权限
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o664)).unwrap();
        write_url_file(&f, "https://example.com/sub/y\n").unwrap();
        let mode = std::fs::metadata(&f).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(
            std::fs::read_to_string(&f).unwrap(),
            "https://example.com/sub/y\n"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
