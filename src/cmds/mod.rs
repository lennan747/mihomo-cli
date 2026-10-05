pub mod config;
pub mod conn;
pub mod group;
pub mod logs;
pub mod mode;
pub mod proxy;
pub mod rule;
pub mod service;
pub mod status;
pub mod sub;
pub mod upgrade;
pub mod version;

/// 字节数转人类可读
pub fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = n as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}
