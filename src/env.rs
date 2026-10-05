//! secret 解析：命令行参数 / `MIHOMO_SECRET` 环境变量（clap 已处理）之外的文件回退。
//!
//! 读取 `~/.config/mihomo/mihomo.env`（与 systemd 用户服务共享的环境文件），仅提取
//! `MIHOMO_SECRET`，不读取、不输出文件中的其他机密内容。

use std::collections::HashMap;
use std::path::Path;

/// secret 解析优先级：命令行参数 > MIHOMO_SECRET 环境变量（clap 已处理）> mihomo.env 文件
pub fn resolve_secret(cli_secret: Option<String>) -> Option<String> {
    if cli_secret.is_some() {
        return cli_secret;
    }
    let home = std::env::var_os("HOME")?;
    let file = Path::new(&home).join(".config/mihomo/mihomo.env");
    let vars = parse_env_file(&file).ok()?;
    vars.get("MIHOMO_SECRET").cloned()
}

fn parse_env_file(path: &Path) -> std::io::Result<HashMap<String, String>> {
    let content = std::fs::read_to_string(path)?;
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                .unwrap_or(value);
            map.insert(key.trim().to_string(), value.to_string());
        }
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::parse_env_file;
    use std::collections::HashMap;

    /// 每个测试写独立的临时文件，避免并发测试互相覆盖
    fn parse(name: &str, content: &str) -> HashMap<String, String> {
        let file = std::env::temp_dir().join(format!("mihomo-cli-env-test-{name}.env"));
        std::fs::write(&file, content).unwrap();
        parse_env_file(&file).unwrap()
    }

    #[test]
    fn 解析键值并剥离引号() {
        let map = parse("quotes", "MIHOMO_SECRET=\"abc123\"\nFOO='bar'\nBAZ=plain\n");
        assert_eq!(map.get("MIHOMO_SECRET").unwrap(), "abc123");
        assert_eq!(map.get("FOO").unwrap(), "bar");
        assert_eq!(map.get("BAZ").unwrap(), "plain");
    }

    #[test]
    fn 跳过空行与注释() {
        let map = parse("comments", "\n# MIHOMO_SECRET=commented\n\nKEY=value\n");
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("KEY").unwrap(), "value");
        assert!(!map.contains_key("MIHOMO_SECRET"));
    }

    #[test]
    fn 无等号的行被忽略() {
        let map = parse("no-eq", "INVALID_LINE\nKEY=value\n");
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn 同名键后者覆盖前者() {
        let map = parse("override", "KEY=old\nKEY=new\n");
        assert_eq!(map.get("KEY").unwrap(), "new");
    }
}
