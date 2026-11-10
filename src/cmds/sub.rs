//! `sub` 子命令：命名订阅的列表/添加/切换/删除，订阅地址的查看与设置，以及手动触发订阅更新。
//!
//! 数据布局（`~/.config/mihomo/` 下）：
//! - `subscription.url`：当前生效地址，更新脚本 `~/.local/bin/mihomo-update` 读取它（保持旧行为）；
//! - `subscriptions/<名称>.url`：命名保存的订阅地址（0600）；
//! - `subscriptions/current`：当前生效的命名订阅名（用 `sub set-url` 直接设置时不存在）。
//!
//! 切换订阅只改写 `subscription.url` 与 `current`，不改动更新脚本，因此每小时的
//! `mihomo-update.timer` 与 `sub update` 始终读取同一份「当前地址」。

use anyhow::{bail, Context, Result};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ui::{self, Column};

/// 订阅名长度上限（按字符计）
const MAX_NAME_CHARS: usize = 64;

/// 脱敏订阅地址：保留 scheme/host 与 path 目录前缀便于辨认，隐去最后一段与 query（token 所在）；
/// 无法解析的地址整体隐藏。
fn mask_url(u: &str) -> String {
    let suffix = format!("（已隐藏 token · 全长 {} 字符）", u.chars().count());
    format!("{}{suffix}", mask_url_brief(u))
}

/// 同 [`mask_url`]，但不附带全长说明，便于放进表格列。
fn mask_url_brief(u: &str) -> String {
    let Ok(parsed) = reqwest::Url::parse(u) else {
        return "****".to_string();
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
    out
}

/// 订阅名会作为文件名使用：拒绝路径分隔符、Windows 保留字符与控制字符，避免越界或跨平台失败。
fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        bail!("订阅名不能为空");
    }
    if name != name.trim() {
        bail!("订阅名不能以空白开头或结尾");
    }
    if name.chars().count() > MAX_NAME_CHARS {
        bail!("订阅名过长（最多 {MAX_NAME_CHARS} 个字符）");
    }
    if name == "." || name == ".." || name.starts_with('.') {
        bail!("订阅名不能以 . 开头");
    }
    if name.chars().any(|c| {
        c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
    }) {
        bail!("订阅名不能包含 / \\ : * ? \" < > | 或控制字符");
    }
    Ok(())
}

fn validate_url(u: &str) -> Result<()> {
    // URL 会原样写入文件，先排除空白/控制字符，避免换行注入到更新脚本读取的内容里
    let has_blank = u.chars().any(|c| c.is_whitespace() || c.is_control());
    let parsed = reqwest::Url::parse(u)
        .ok()
        .map(|p| matches!(p.scheme(), "http" | "https") && p.host_str().is_some());
    if has_blank || parsed != Some(true) {
        bail!(
            "订阅地址必须是合法的 http:// 或 https:// URL: {}",
            mask_url(u)
        );
    }
    Ok(())
}

/// 私有目录：不存在时创建并收紧为 0700；已存在则保持原权限，不擅自改动用户目录。
fn ensure_private_dir(dir: &Path) -> Result<()> {
    if dir.as_os_str().is_empty() || dir.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(dir).with_context(|| format!("创建目录 {} 失败", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

fn ensure_parent_dir(file: &Path) -> Result<()> {
    match file.parent() {
        Some(dir) => ensure_private_dir(dir),
        None => Ok(()),
    }
}

/// 写入凭据文件：Unix 下权限固定 0600（订阅地址含 token，与 mihomo.env 同一约定）。
/// `OpenOptions::mode` 仅对新建文件生效，已存在的文件需再补一次 set_permissions。
#[cfg(unix)]
fn write_secret_file(file: &Path, content: &str) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    ensure_parent_dir(file)?;
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
fn write_secret_file(file: &Path, content: &str) -> Result<()> {
    ensure_parent_dir(file)?;
    std::fs::write(file, content).with_context(|| format!("写入 {} 失败", file.display()))
}

/// 订阅存储：`~/.config/mihomo` 下的 `subscription.url` 与 `subscriptions/` 目录。
struct SubStore {
    config_dir: PathBuf,
}

impl SubStore {
    fn from_home() -> Self {
        let home = std::env::var_os("HOME").unwrap_or_default();
        Self::at(PathBuf::from(home).join(".config").join("mihomo"))
    }

    fn at(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    fn url_file(&self) -> PathBuf {
        self.config_dir.join("subscription.url")
    }

    fn subs_dir(&self) -> PathBuf {
        self.config_dir.join("subscriptions")
    }

    fn current_file(&self) -> PathBuf {
        self.subs_dir().join("current")
    }

    fn named_file(&self, name: &str) -> PathBuf {
        self.subs_dir().join(format!("{name}.url"))
    }

    /// 当前生效的命名订阅名；`sub set-url` 设置的未命名地址没有该标记。
    fn active_name(&self) -> Option<String> {
        let s = std::fs::read_to_string(self.current_file()).ok()?;
        let s = s.trim();
        (!s.is_empty()).then(|| s.to_string())
    }

    /// 当前生效地址（更新脚本读取的同一份文件）
    fn active_url(&self) -> Option<String> {
        let s = std::fs::read_to_string(self.url_file()).ok()?;
        let s = s.trim();
        (!s.is_empty()).then(|| s.to_string())
    }

    /// 全部命名订阅（按名称排序）；目录不存在时返回空列表。
    /// 单条读取失败降级为空串，不因一个坏文件让整个列表失败。
    fn entries(&self) -> Result<Vec<(String, String)>> {
        let dir = self.subs_dir();
        let read = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => {
                return Err(e).with_context(|| format!("读取目录 {} 失败", dir.display()));
            }
        };
        let mut out = Vec::new();
        for entry in read {
            let entry = entry.with_context(|| format!("读取目录 {} 失败", dir.display()))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("url") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let url = std::fs::read_to_string(&path)
                .unwrap_or_default()
                .trim()
                .to_string();
            out.push((name.to_string(), url));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    fn add(&self, name: &str, url: &str, force: bool) -> Result<()> {
        validate_name(name)?;
        validate_url(url)?;
        let file = self.named_file(name);
        if file.exists() && !force {
            bail!("订阅 {name} 已存在；覆盖请加 --force");
        }
        write_secret_file(&file, &format!("{}\n", url.trim()))?;
        // 覆盖的正是当前生效订阅时同步刷新 `subscription.url`，避免命名条目与实际地址不一致
        if self.active_name().as_deref() == Some(name) {
            self.set_active(Some(name), url.trim())?;
        }
        Ok(())
    }

    /// 切换当前订阅：同步 `subscription.url`（更新脚本读取）与 `current`（标记生效名称）
    fn activate(&self, name: &str) -> Result<()> {
        validate_name(name)?;
        let file = self.named_file(name);
        let url = std::fs::read_to_string(&file).with_context(|| {
            format!("订阅 {name} 不存在；用 `mihomo-cli sub list` 查看已有订阅")
        })?;
        let url = url.trim();
        validate_url(url)?;
        self.set_active(Some(name), url)
    }

    fn set_active(&self, name: Option<&str>, url: &str) -> Result<()> {
        write_secret_file(&self.url_file(), &format!("{url}\n"))?;
        match name {
            Some(n) => write_secret_file(&self.current_file(), &format!("{n}\n")),
            None => {
                let _ = std::fs::remove_file(self.current_file());
                Ok(())
            }
        }
    }

    fn remove(&self, name: &str) -> Result<()> {
        validate_name(name)?;
        let file = self.named_file(name);
        if !file.exists() {
            bail!("订阅 {name} 不存在；用 `mihomo-cli sub list` 查看已有订阅");
        }
        if self.active_name().as_deref() == Some(name) {
            bail!("订阅 {name} 正在生效，请先 `mihomo-cli sub use <其他订阅>` 再删除");
        }
        std::fs::remove_file(&file).with_context(|| format!("删除 {} 失败", file.display()))
    }
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

/// 列出命名订阅（地址默认脱敏），★ 标记当前生效的订阅。
pub fn list() -> Result<()> {
    let store = SubStore::from_home();
    let entries = store.entries()?;
    let active = store.active_name();

    let rows: Vec<Vec<String>> = entries
        .iter()
        .map(|(name, url)| {
            let mark = if active.as_deref() == Some(name.as_str()) {
                ui::bold_green("★")
            } else {
                String::new()
            };
            let url_cell = if url.is_empty() {
                ui::dim("（无法读取）")
            } else {
                mask_url_brief(url)
            };
            vec![name.clone(), url_cell, mark]
        })
        .collect();

    let summary = if entries.is_empty() {
        Some(ui::dim("还没有命名订阅：mihomo-cli sub add <名称> <URL>"))
    } else if active.is_none() {
        Some(ui::dim(match store.active_url() {
            Some(_) => {
                "当前生效的是未命名地址（sub set-url 设置）；用 sub use <名称> 切换到命名订阅"
            }
            None => "尚无生效订阅；用 sub use <名称> 切换",
        }))
    } else {
        None
    };

    let columns = [
        Column::flex("name", 20),
        Column::new("url", 44),
        Column::new("sel", 3),
    ];
    ui::table_dashboard(
        "Subscriptions",
        &format!(
            "active={} · total={}",
            active.as_deref().unwrap_or("-"),
            entries.len()
        ),
        &columns,
        &rows,
        summary.as_deref(),
    );
    Ok(())
}

/// 保存命名订阅；不自动切换（切换用 `sub use`），覆盖已有名称需 `--force`。
pub fn add(name: &str, url: &str, force: bool) -> Result<()> {
    let store = SubStore::from_home();
    let was_active = store.active_name().as_deref() == Some(name);
    store.add(name, url, force)?;
    println!(
        "{}",
        ui::green(&format!("✓ 已添加订阅 {name}: {}", mask_url(url)))
    );
    if was_active {
        println!("{}", ui::dim("该订阅正在生效，当前地址已同步更新"));
    } else {
        println!("提示: 用 `mihomo-cli sub use {name}` 切换到此订阅");
    }
    Ok(())
}

/// 切换当前订阅；`update` 为真时切换后立即执行订阅更新脚本。
pub fn use_sub(name: &str, update: bool) -> Result<()> {
    let store = SubStore::from_home();
    store.activate(name)?;
    println!("{}", ui::green(&format!("✓ 已切换当前订阅: {name}")));
    if update {
        println!("开始更新订阅 ...");
        run().with_context(|| format!("已切换为订阅 {name}，但订阅更新失败"))?;
    } else {
        println!("提示: 运行 `mihomo-cli sub update` 立即拉取该订阅的节点");
    }
    Ok(())
}

/// 删除命名订阅；正在生效的订阅必须先切换走，避免当前地址指向已删除的条目。
pub fn remove(name: &str) -> Result<()> {
    let store = SubStore::from_home();
    store.remove(name)?;
    println!("{}", ui::green(&format!("✓ 已删除订阅: {name}")));
    Ok(())
}

/// 显示当前订阅地址：优先 `subscription.url`（并标注对应的命名订阅），其次从更新脚本中提取。
/// 默认脱敏（订阅地址含 token，与 secret 一样不进输出）；`--reveal` 才完整回显。
pub fn url(reveal: bool) -> Result<()> {
    let store = SubStore::from_home();
    let render = |u: &str| -> String {
        if reveal {
            u.to_string()
        } else {
            mask_url(u)
        }
    };
    if let Some(u) = store.active_url() {
        let source = match store.active_name() {
            Some(name) => format!("命名订阅 {name} · {}", store.named_file(&name).display()),
            None => store.url_file().display().to_string(),
        };
        println!("订阅地址: {}\n（来源: {source}）", render(&u));
        return Ok(());
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
    bail!("未找到订阅地址，请用 `mihomo-cli sub add <名称> <URL>` 添加后 `sub use <名称>`，或用 `sub set-url <URL>` 直接设置");
}

/// 设置「未命名」的当前地址（不属于任何命名订阅），写入 `subscription.url`；
/// 命名订阅通过 `sub add` + `sub use` 管理。
pub fn set_url(u: &str) -> Result<()> {
    validate_url(u)?;
    let store = SubStore::from_home();
    let had_name = store.active_name();
    store.set_active(None, u.trim())?;
    println!(
        "{}",
        ui::green(&format!("✓ 订阅地址已保存: {}", mask_url(u)))
    );
    if had_name.is_some() {
        println!(
            "{}",
            ui::dim("已取消命名订阅标记：该地址由 sub set-url 直接设置，不属于任何命名订阅")
        );
    }
    println!(
        "提示: 每小时的 mihomo-update.timer 也会使用此地址；立即更新请运行 `mihomo-cli sub update`"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        mask_url, mask_url_brief, validate_name, validate_url, write_secret_file, SubStore,
    };
    use std::path::PathBuf;

    /// 每个测试用独立的临时配置目录，避免并发测试互相干扰
    fn temp_store(tag: &str) -> (SubStore, PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("mihomo-cli-sub-test-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        (SubStore::at(dir.clone()), dir)
    }

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

    #[test]
    fn 表格用脱敏不带全长说明() {
        let m = mask_url_brief("https://example.com/sub/token123");
        assert_eq!(m, "https://example.com/sub/****");
        assert_eq!(mask_url_brief("不是地址"), "****");
    }

    #[test]
    fn 订阅名合法性校验() {
        for ok in ["机场A", "Airport-1", "a_b.c", "订阅 02"] {
            assert!(validate_name(ok).is_ok(), "{ok} 应被接受");
        }
        for bad in [
            "", "   ", " x", "x ", ".", "..", ".hidden", "a/b", "a\\b", "a:b", "a*b", "a?b",
            "a\"b", "a<b", "a|b", "a\nb",
        ] {
            assert!(validate_name(bad).is_err(), "{bad:?} 应被拒绝");
        }
        assert!(validate_name(&"名".repeat(65)).is_err());
    }

    #[test]
    fn 订阅地址合法性校验() {
        for ok in [
            "http://example.com",
            "https://example.com/sub?token=abc",
            "http://127.0.0.1:8080/sub",
        ] {
            assert!(validate_url(ok).is_ok(), "{ok} 应被接受");
        }
        for bad in [
            "",
            "example.com",
            "ftp://example.com",
            "http://",
            "https://a b",
            "https://example.com/a\nb",
        ] {
            assert!(validate_url(bad).is_err(), "{bad:?} 应被拒绝");
        }
    }

    #[test]
    fn 添加与列出命名订阅() {
        let (s, dir) = temp_store("add");
        s.add("机场A", "https://example.com/sub/a", false).unwrap();
        s.add("机场B", "https://example.com/sub/b", false).unwrap();

        let entries = s.entries().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0],
            ("机场A".into(), "https://example.com/sub/a".into())
        );
        assert_eq!(entries[1].0, "机场B");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 重名需_force_覆盖且当前订阅同步() {
        let (s, dir) = temp_store("force");
        s.add("A", "https://example.com/sub/a", false).unwrap();
        assert!(s.add("A", "https://example.com/sub/b", false).is_err());

        s.activate("A").unwrap();
        s.add("A", "https://example.com/sub/c", true).unwrap();
        // 覆盖生效中的订阅时，当前地址同步为最新值
        assert_eq!(s.active_url().unwrap(), "https://example.com/sub/c");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 切换订阅同步当前地址与标记() {
        let (s, dir) = temp_store("use");
        s.add("A", "https://example.com/sub/a", false).unwrap();
        s.add("B", "https://example.com/sub/b", false).unwrap();

        s.activate("A").unwrap();
        assert_eq!(s.active_name().as_deref(), Some("A"));
        assert_eq!(s.active_url().unwrap(), "https://example.com/sub/a");

        s.activate("B").unwrap();
        assert_eq!(s.active_name().as_deref(), Some("B"));
        assert_eq!(s.active_url().unwrap(), "https://example.com/sub/b");

        assert!(s.activate("不存在").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 未命名地址清除命名标记() {
        let (s, dir) = temp_store("unnamed");
        s.add("A", "https://example.com/sub/a", false).unwrap();
        s.activate("A").unwrap();

        s.set_active(None, "https://example.com/sub/manual")
            .unwrap();
        assert!(s.active_name().is_none());
        assert_eq!(s.active_url().unwrap(), "https://example.com/sub/manual");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 生效中的订阅不可删除() {
        let (s, dir) = temp_store("remove");
        s.add("A", "https://example.com/sub/a", false).unwrap();
        s.add("B", "https://example.com/sub/b", false).unwrap();
        s.activate("A").unwrap();

        assert!(s.remove("A").is_err());
        s.remove("B").unwrap();
        assert_eq!(s.entries().unwrap().len(), 1);
        assert!(s.remove("B").is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 目录不存在时列表为空() {
        let (s, dir) = temp_store("empty");
        assert!(s.entries().unwrap().is_empty());
        assert!(s.active_url().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn 写入文件权限固定为_0600() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("mihomo-cli-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("subscription.url");

        write_secret_file(&f, "https://example.com/sub/x\n").unwrap();
        let mode = std::fs::metadata(&f).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        // 已存在且权限过宽的文件，再次写入时补正权限
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o664)).unwrap();
        write_secret_file(&f, "https://example.com/sub/y\n").unwrap();
        let mode = std::fs::metadata(&f).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(
            std::fs::read_to_string(&f).unwrap(),
            "https://example.com/sub/y\n"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
