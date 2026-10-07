//! 子命令实现：每个子命令一个模块（与 `cli.rs` 的枚举变体同名），本模块只放共享工具。

use crate::ui;
use std::collections::BTreeMap;

pub mod config;
pub mod conn;
pub mod group;
pub mod logs;
pub mod mode;
pub mod proxy;
pub mod rule;
pub mod self_upgrade;
pub mod service;
pub mod status;
pub mod sub;
pub mod upgrade;
pub mod version;

/// 字节数转人类可读（1024 进制，保留 1 位小数；小于 1 KB 时输出整数）
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

/// 统计各取值出现次数，格式化为 `key=count`（按次数降序、同次数按名称升序），键名 dim 着色。
/// 用于表格底栏汇总，如 `Selector=12  Fallback=1`。
pub fn histogram<'a, I: Iterator<Item = &'a str>>(items: I) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for k in items {
        *counts.entry(k).or_insert(0) += 1;
    }
    let mut pairs: Vec<(&str, usize)> = counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    pairs
        .iter()
        .map(|(k, n)| format!("{}={n}", ui::dim(k)))
        .collect::<Vec<_>>()
        .join("  ")
}

#[cfg(test)]
mod tests {
    use super::human_bytes;

    #[test]
    fn 小于_1kb_输出整数() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1023), "1023 B");
    }

    #[test]
    fn 进位与单位() {
        assert_eq!(human_bytes(1024), "1.0 KB");
        assert_eq!(human_bytes(1536), "1.5 KB");
        assert_eq!(human_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(human_bytes(1024_u64.pow(3)), "1.0 GB");
        assert_eq!(human_bytes(1024_u64.pow(4)), "1.0 TB");
    }

    #[test]
    fn 超出_tb_不再进位() {
        assert_eq!(human_bytes(1024_u64.pow(5)), "1024.0 TB");
    }
}
