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
