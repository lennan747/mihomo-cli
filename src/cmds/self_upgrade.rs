//! `self-upgrade` 子命令：从 GitHub Releases 下载 mihomo-cli 客户端最新版并原地替换当前可执行文件。
//!
//! 与 `upgrade`（升级 mihomo 内核）不同，本命令升级的是 CLI 自身。
//! 安全：下载后强制校验发布资产对应的 SHA256SUMS，校验不通过即中止，不提供绕过选项。

use crate::github::{latest_release_tag, normalize_version};
use anyhow::{bail, Context, Result};
use std::io::Read;
use std::path::{Path, PathBuf};

const REPO: &str = "lennan747/mihomo-cli";
const PROXY: &str = "http://127.0.0.1:7890";
const BIN_NAME: &str = "mihomo-cli";

/// 把当前平台映射为发布资产的目标三元组
fn target_for(os: &str, arch: &str) -> Result<&'static str> {
    match (os, arch) {
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("windows", "x86_64") => Ok("x86_64-pc-windows-msvc"),
        _ => bail!(
            "暂不支持当前平台: {os}/{arch}（发布资产仅覆盖 linux-x86_64 / macos-arm64 / windows-x86_64）"
        ),
    }
}

/// 返回发布资产名与其归档格式（"tar.gz" / "zip"）
fn asset_name(target: &str) -> (String, &'static str) {
    if target.contains("windows") {
        (format!("{BIN_NAME}-{target}.zip"), "zip")
    } else {
        (format!("{BIN_NAME}-{target}.tar.gz"), "tar.gz")
    }
}

/// 是否为当前平台的二进制文件名（Windows 带 .exe）
fn bin_file_name(target: &str) -> &'static str {
    if target.contains("windows") {
        "mihomo-cli.exe"
    } else {
        BIN_NAME
    }
}

/// 从 SHA256SUMS 文本中提取指定资产的校验值（小写 hex）；兼容 `<hex>  <name>` 与 `<hex> *<name>`
fn parse_sha256sums(text: &str, asset: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        (name == asset && hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// 从 `.tar.gz` 中取出指定文件名条目的内容
fn extract_tar_gz(bytes: &[u8], target_name: &str) -> Result<Vec<u8>> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    for entry in archive.entries().context("读取 tar 归档失败")? {
        let mut entry = entry.context("读取 tar 条目失败")?;
        let path = entry.path().context("读取 tar 条目路径失败")?;
        if path.file_name().and_then(|n| n.to_str()) == Some(target_name) {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).context("读取二进制内容失败")?;
            return Ok(buf);
        }
    }
    bail!("压缩包内未找到 {target_name}")
}

/// 从 `.zip` 中取出指定文件名条目的内容
fn extract_zip(bytes: &[u8], target_name: &str) -> Result<Vec<u8>> {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).context("读取 zip 归档失败")?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).context("读取 zip 条目失败")?;
        let is_target = std::path::Path::new(file.name())
            .file_name()
            .and_then(|n| n.to_str())
            == Some(target_name);
        if is_target {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).context("读取二进制内容失败")?;
            return Ok(buf);
        }
    }
    bail!("压缩包内未找到 {target_name}")
}

async fn download_once(proxy: Option<&str>, url: &str) -> Result<Vec<u8>> {
    let mut builder = reqwest::Client::builder().user_agent("mihomo-cli");
    if let Some(p) = proxy {
        builder = builder.proxy(reqwest::Proxy::all(p).context("无效的代理地址")?);
    }
    let client = builder.build()?;
    let resp = client.get(url).send().await.context("请求下载地址失败")?;
    let status = resp.status();
    if !status.is_success() {
        bail!("HTTP {status}");
    }
    Ok(resp.bytes().await.context("读取下载内容失败")?.to_vec())
}

/// 下载资产（跟随重定向）；直连失败时回退到本机混合代理端口 7890
async fn download(url: &str) -> Result<Vec<u8>> {
    match download_once(None, url).await {
        Ok(b) => Ok(b),
        Err(direct_err) => match download_once(Some(PROXY), url).await {
            Ok(b) => Ok(b),
            Err(proxy_err) => {
                bail!("下载失败: 直连（{direct_err}）；经代理 7890 也失败（{proxy_err}）\n  地址: {url}")
            }
        },
    }
}

/// 替换前写入的临时文件路径，与目标可执行文件同一目录
/// （保证同一文件系统，rename 替换才能原子完成；跨设备 rename 会失败）
fn tmp_binary_path(exe: &Path) -> PathBuf {
    let dir = exe.parent().unwrap_or_else(|| Path::new("."));
    let mut p = dir.join(format!(".{BIN_NAME}-{}.tmp", std::process::id()));
    if cfg!(windows) {
        p.set_extension("exe");
    }
    p
}

pub async fn run(check: bool, version: Option<String>, force: bool) -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    let target = target_for(std::env::consts::OS, std::env::consts::ARCH)?;
    let (asset, format) = asset_name(target);

    let explicit = version.is_some();
    let tag = match version {
        Some(v) => {
            let v = v.trim();
            if v.starts_with('v') {
                v.to_string()
            } else {
                format!("v{v}")
            }
        }
        None => {
            println!("正在查询最新版本 ...");
            latest_release_tag(REPO).await.context("查询最新版本失败")?
        }
    };
    println!("当前版本: v{current}，目标版本: {tag}（平台: {target}）");

    let is_newer = normalize_version(&tag) > normalize_version(current);
    if check {
        if is_newer {
            println!("有新版本可用，运行 `mihomo-cli self-upgrade` 升级");
        } else {
            println!("已是最新版本");
        }
        return Ok(());
    }
    if !is_newer && !force && !explicit {
        println!("已是最新版本（如需强制重装：`mihomo-cli self-upgrade --force`）");
        return Ok(());
    }

    let base = format!("https://github.com/{REPO}/releases/download/{tag}");
    println!("下载 {asset} ...");
    let archive = download(&format!("{base}/{asset}")).await?;

    println!("下载并校验 SHA256SUMS ...");
    let sums = download(&format!("{base}/SHA256SUMS")).await?;
    let sums = String::from_utf8(sums).context("SHA256SUMS 不是合法文本")?;
    let expected = parse_sha256sums(&sums, &asset)
        .with_context(|| format!("SHA256SUMS 中未找到 {asset} 的校验值"))?;
    let actual = sha256_hex(&archive);
    if actual != expected {
        bail!("SHA256 校验失败，已中止升级（请勿绕过校验）\n  期望: {expected}\n  实际: {actual}");
    }
    println!("校验通过");

    let bytes = if format == "zip" {
        extract_zip(&archive, bin_file_name(target))?
    } else {
        extract_tar_gz(&archive, bin_file_name(target))?
    };
    println!("解压得到 {} 字节，准备替换当前可执行文件 ...", bytes.len());

    let exe = std::env::current_exe().context("获取当前可执行文件路径失败")?;
    let tmp = tmp_binary_path(&exe);
    std::fs::write(&tmp, &bytes).with_context(|| format!("写入临时文件 {} 失败", tmp.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
            .with_context(|| format!("设置 {} 权限失败", tmp.display()))?;
    }

    let replaced = self_replace::self_replace(&tmp);
    let _ = std::fs::remove_file(&tmp);
    replaced.with_context(|| {
        format!(
            "替换 {} 失败（可能无写权限，请用 sudo 重试或按 README 手动重装）",
            exe.display()
        )
    })?;

    if is_newer {
        println!("升级完成: v{current} -> {tag}");
    } else {
        println!("重装完成: {tag}");
    }
    println!("重新运行 `mihomo-cli --version` 确认生效");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{asset_name, bin_file_name, parse_sha256sums, sha256_hex, target_for};

    #[test]
    fn 平台映射到发布目标() {
        assert_eq!(
            target_for("linux", "x86_64").unwrap(),
            "x86_64-unknown-linux-gnu"
        );
        assert_eq!(
            target_for("macos", "aarch64").unwrap(),
            "aarch64-apple-darwin"
        );
        assert_eq!(
            target_for("windows", "x86_64").unwrap(),
            "x86_64-pc-windows-msvc"
        );
        assert!(target_for("linux", "aarch64").is_err());
    }

    #[test]
    fn 资产名与平台对应() {
        assert_eq!(
            asset_name("x86_64-unknown-linux-gnu"),
            (
                "mihomo-cli-x86_64-unknown-linux-gnu.tar.gz".to_string(),
                "tar.gz"
            )
        );
        assert_eq!(
            asset_name("x86_64-pc-windows-msvc"),
            ("mihomo-cli-x86_64-pc-windows-msvc.zip".to_string(), "zip")
        );
        assert_eq!(bin_file_name("x86_64-pc-windows-msvc"), "mihomo-cli.exe");
        assert_eq!(bin_file_name("aarch64-apple-darwin"), "mihomo-cli");
    }

    #[test]
    fn 解析校验和两种分隔格式() {
        let text = "aaaa  other.tar.gz\n\
                    ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  mihomo-cli-x86_64-unknown-linux-gnu.tar.gz\n";
        assert_eq!(
            parse_sha256sums(text, "mihomo-cli-x86_64-unknown-linux-gnu.tar.gz").unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // `sha256sum -b` 的二进制模式用 * 前缀，且大写 hash 归一化为小写
        assert_eq!(
            parse_sha256sums(
                "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD *a.zip",
                "a.zip"
            )
            .unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // 长度不足 / 非十六进制 / 资产不存在 均为 None
        assert_eq!(parse_sha256sums("ba78  a.zip", "a.zip"), None);
        assert_eq!(
            parse_sha256sums(&format!("{}  a.zip", "z".repeat(64)), "a.zip"),
            None
        );
        assert_eq!(parse_sha256sums("ba78  a.zip", "b.zip"), None);
    }

    #[test]
    fn sha256_已知向量() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
