//! 终端渲染工具箱：圆角边框、表格 dashboard、按可见宽度对齐、TTY 自适应颜色。
//!
//! 样式与 qq-triage 的 `cli/_styles.py` / `cli/_tables.py` 保持一致：
//! 顶边内嵌标题、表头 + `├─┤` 分隔、底栏汇总；CJK 按 2 列、几何符号按 1 列计算宽度，
//! ANSI 色码不计入宽度。颜色仅在 stdout 为 TTY 且未设置 `NO_COLOR` 时启用。

use std::io::IsTerminal;
use std::sync::OnceLock;
use unicode_width::UnicodeWidthChar;

// ---- ANSI 颜色码 ----

pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[90m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const RED: &str = "\x1b[31m";
pub const RESET: &str = "\x1b[0m";

/// 是否启用颜色：仅当 stdout 为 TTY 且未设置 `NO_COLOR`（存在即禁用，含空值）
pub fn color_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED
        .get_or_init(|| std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal())
}

/// 用一组 ANSI 码包裹字符串；未启用颜色或 `codes` 为空时原样返回。
pub fn style(s: &str, codes: &[&str]) -> String {
    if codes.is_empty() || !color_enabled() {
        return s.to_string();
    }
    format!("{}{s}{RESET}", codes.concat())
}

pub fn bold(s: &str) -> String {
    style(s, &[BOLD])
}
pub fn dim(s: &str) -> String {
    style(s, &[DIM])
}
pub fn green(s: &str) -> String {
    style(s, &[GREEN])
}
pub fn yellow(s: &str) -> String {
    style(s, &[YELLOW])
}
pub fn red(s: &str) -> String {
    style(s, &[RED])
}
pub fn bold_green(s: &str) -> String {
    style(s, &[BOLD, GREEN])
}

// ---- 可见宽度 ----

/// 跳过一段 CSI 序列（`ESC [ ... 终止符`），返回消费到的字符数（含 ESC）。
/// 非 CSI 的 ESC（如独立的 ESC）按 1 个字符处理。
fn ansi_len(chars: &[char], start: usize) -> usize {
    if chars[start] != '\x1b' || chars.get(start + 1) != Some(&'[') {
        return 1;
    }
    let mut i = start + 2;
    while i < chars.len() {
        let c = chars[i];
        i += 1;
        if ('\x40'..='\x7e').contains(&c) {
            break;
        }
    }
    i - start
}

/// 压成单物理行：控制字符替换为空格（避免节点名含控制字符时破框），ANSI 序列原样保留。
pub fn one_line(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\x1b' {
            let n = ansi_len(&chars, i);
            out.extend(&chars[i..i + n]);
            i += n;
        } else {
            out.push(if ch.is_control() { ' ' } else { ch });
            i += 1;
        }
    }
    out
}

/// 单个字符的终端列宽（CJK=2，几何符号/Ambiguous=1）。
///
/// 变体选择符 VS16（U+FE0F）要求前一个字符按 emoji 呈现，终端实际占 2 列，但
/// `unicode-width` 把它算作 0 列——这会让 `♻️`/`➡️` 这类 emoji 所在行少补一格空格，
/// 右侧边框外凸、整行错位。故这里把 VS16 记为 1 列（与基字符合计 2），其余变体选择符记为 0。
fn char_width(c: char) -> usize {
    if c == '\u{FE0F}' {
        return 1;
    }
    if ('\u{FE00}'..='\u{FE0E}').contains(&c) || ('\u{E0100}'..='\u{E01EF}').contains(&c) {
        return 0;
    }
    UnicodeWidthChar::width(c).unwrap_or(0)
}

/// 字符串可见宽度（剥离 ANSI；CJK=2，几何符号/Ambiguous=1）
pub fn vis_width(s: &str) -> usize {
    let chars: Vec<char> = s.chars().collect();
    let mut w = 0;
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\x1b' {
            i += ansi_len(&chars, i);
            continue;
        }
        w += char_width(chars[i]);
        i += 1;
    }
    w
}

/// 按可见宽度补齐（保留 ANSI）；`right_align` 为真时左补空格。
pub fn pad_visible(s: &str, width: usize, right_align: bool) -> String {
    let w = vis_width(s);
    if w >= width {
        return s.to_string();
    }
    let pad = " ".repeat(width - w);
    if right_align {
        format!("{pad}{s}")
    } else {
        format!("{s}{pad}")
    }
}

/// 按可见宽度截断（保留 ANSI，截断处补 `…`；若含色码则补回 RESET 防止颜色泄漏）。
pub fn truncate_visible(s: &str, width: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut used = 0usize;
    let mut had_ansi = false;
    let mut i = 0;
    let mut cut = false;
    while i < chars.len() {
        if chars[i] == '\x1b' {
            had_ansi = true;
            let n = ansi_len(&chars, i);
            out.extend(&chars[i..i + n]);
            i += n;
            continue;
        }
        let cw = char_width(chars[i]);
        if used + cw > width {
            cut = true;
            break;
        }
        out.push(chars[i]);
        used += cw;
        i += 1;
    }
    if !cut {
        return s.to_string();
    }
    if used < width {
        out.push('…');
    }
    if had_ansi {
        out.push_str(RESET);
    }
    out
}

/// 终端宽度；取不到时用 `default`，并按 `[min, max]` 夹取。
pub fn panel_width(min: usize, max: usize, default: usize) -> usize {
    let w = terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(default);
    w.clamp(min, max)
}

// ---- 盒装渲染器 ----

/// 固定宽度的圆角框渲染器（顶层 / 分隔 / 底边 / 行 / 空行）。
pub struct Box {
    width: usize,
}

impl Box {
    pub fn new(width: usize) -> Self {
        Self {
            width: width.max(20),
        }
    }

    /// 框内可用宽度（左右各 1 边距 = 4 列）
    pub fn inner(&self) -> usize {
        self.width - 4
    }

    /// 顶边：`╭─── title · right ───╮`（标题按可见宽度居中）
    pub fn hline(&self, title: &str, right: &str) -> String {
        let usable = self.width - 2;
        let center = if right.is_empty() {
            format!(" {} ", one_line(title))
        } else {
            format!(" {} · {} ", one_line(title), one_line(right))
        };
        let center = if vis_width(&center) > usable {
            truncate_visible(&center, usable)
        } else {
            center
        };
        let cw = vis_width(&center).min(usable);
        let side = (usable - cw) / 2;
        let extra = (usable - cw) % 2;
        format!("╭{}{center}{}╮", "─".repeat(side), "─".repeat(side + extra))
    }

    pub fn hbot(&self) -> String {
        format!("╰{}╯", "─".repeat(self.width - 2))
    }

    pub fn hdiv(&self) -> String {
        format!("├{}┤", "─".repeat(self.width - 2))
    }

    /// 一行「左标签 + 右值」`│ label            value │`
    pub fn row(&self, left: &str, right: &str) -> String {
        self.row_badge(left, right, "")
    }

    /// 同 [`Box::row`]，末尾追加一个徽章（如告警 `!`）
    pub fn row_badge(&self, left: &str, right: &str, badge: &str) -> String {
        let inner = self.inner();
        let left = one_line(left);
        let mut tail = one_line(right);
        if !badge.is_empty() {
            let badge = one_line(badge);
            tail = if tail.trim().is_empty() {
                badge
            } else {
                format!("{tail} {badge}")
            };
        }
        let max_tail = inner.saturating_sub(2);
        let tail = if vis_width(&tail) > max_tail {
            truncate_visible(&tail, max_tail)
        } else {
            tail
        };
        let tw = vis_width(&tail);
        let head_w = if tw == 0 {
            inner
        } else {
            inner.saturating_sub(tw + 1).max(1)
        };
        let head = pad_visible(&truncate_visible(&left, head_w), head_w, false);
        let gap = inner.saturating_sub(head_w + tw);
        format!("│ {head}{}{tail} │", " ".repeat(gap))
    }

    /// 一行多列（单元格已按列宽截断+补齐），整体再补齐到内宽
    pub fn row_cells(&self, cells: &[String], widths: &[usize]) -> String {
        let parts: Vec<String> = cells
            .iter()
            .zip(widths)
            .map(|(c, w)| pad_visible(&truncate_visible(&one_line(c), *w), *w, false))
            .collect();
        let joined = parts.join(" ");
        let pad = self.inner().saturating_sub(vis_width(&joined));
        format!("│ {joined}{} │", " ".repeat(pad))
    }
}

// ---- 上层构件 ----

/// 表格的一列；`flex` 为真的列弹性填满面板内宽（应恰好一列）
pub struct Column {
    pub header: String,
    pub min_width: usize,
    pub flex: bool,
}

impl Column {
    pub fn new(header: &str, min_width: usize) -> Self {
        Self {
            header: header.to_string(),
            min_width,
            flex: false,
        }
    }

    pub fn flex(header: &str, min_width: usize) -> Self {
        Self {
            header: header.to_string(),
            min_width,
            flex: true,
        }
    }
}

/// 收缩列宽使总和不超过 `target`：每次把最宽的列减 1，直到达标或全部触底（下限 4）。
fn shrink_to_fit(widths: &mut [usize], target: usize) {
    const FLOOR: usize = 4;
    while widths.iter().sum::<usize>() > target {
        match (0..widths.len())
            .filter(|&i| widths[i] > FLOOR)
            .max_by_key(|&i| widths[i])
        {
            Some(i) => widths[i] -= 1,
            None => break,
        }
    }
}

/// 计算各列实际宽度：弹性列填满内宽；若总和仍超出则按比例收缩，保证不溢出面板。
fn column_widths(columns: &[Column], inner: usize) -> Vec<usize> {
    let mut widths: Vec<usize> = columns.iter().map(|c| c.min_width).collect();
    let gaps = columns.len().saturating_sub(1);
    let avail = inner.saturating_sub(gaps);

    if let Some(i) = columns.iter().position(|c| c.flex) {
        let others: usize = widths
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, w)| *w)
            .sum();
        let want = avail.saturating_sub(others);
        widths[i] = want.max(columns[i].min_width);
    }
    if widths.iter().sum::<usize>() > avail {
        shrink_to_fit(&mut widths, avail);
    }
    widths
}

/// 带边框的表格 dashboard。`rows` 为逐行的单元格文本（可含 ANSI 色码）。
///
/// 面板宽度由列的最小宽度之和决定（不低于 80），随终端上限放宽，避免出现大片空白列。
pub fn table_dashboard(
    title: &str,
    title_right: &str,
    columns: &[Column],
    rows: &[Vec<String>],
    summary: Option<&str>,
) {
    let term_max = panel_width(80, 140, 100);
    let required =
        columns.iter().map(|c| c.min_width).sum::<usize>() + columns.len().saturating_sub(1) + 4;
    let b = Box::new(required.clamp(80, term_max));
    let inner = b.inner();
    let widths = column_widths(columns, inner);

    println!();
    println!("{}", b.hline(title, title_right));

    let header: Vec<String> = columns.iter().map(|c| c.header.clone()).collect();
    println!("{}", b.row_cells(&header, &widths));
    println!("{}", b.hdiv());

    if rows.is_empty() {
        let msg = dim("（无数据）");
        let pad = inner.saturating_sub(vis_width(&msg));
        println!("│ {msg}{} │", " ".repeat(pad));
        println!("{}", b.hbot());
        if let Some(s) = summary {
            println!("  {s}");
        }
        return;
    }

    for r in rows {
        println!("{}", b.row_cells(r, &widths));
    }
    println!("{}", b.hbot());
    if let Some(s) = summary {
        println!("  {s}");
    }
}

/// 键值面板 `│ 标签                     值 │`
///
/// 宽度由标签/值/标题的自然宽度决定（不低于 50）。
pub fn kv_panel(title: &str, title_right: &str, items: &[(&str, String)], summary: Option<&str>) {
    let term_max = panel_width(50, 74, 80);
    let rows_max = items
        .iter()
        .map(|(k, v)| vis_width(k) + vis_width(v) + 6)
        .max()
        .unwrap_or(0);
    let title_w =
        vis_width(title) + vis_width(title_right) + if title_right.is_empty() { 2 } else { 6 };
    let b = Box::new((rows_max.max(title_w) + 4).clamp(50, term_max));
    println!();
    println!("{}", b.hline(title, title_right));
    for (k, v) in items {
        println!("{}", b.row(k, v));
    }
    println!("{}", b.hbot());
    if let Some(s) = summary {
        println!("  {s}");
    }
}

/// 框外顶部状态条：`  ● mihomo-cli status  ·  rule  ·  mixed=7890`
pub fn status_bar(segments: &[String]) {
    let sep = format!("  {}  ", dim("·"));
    println!("  {}", segments.join(&sep));
}

#[cfg(test)]
mod tests {
    use super::{one_line, truncate_visible, vis_width, Box, Column};

    #[test]
    fn 中文与符号宽度() {
        assert_eq!(vis_width("abc"), 3);
        assert_eq!(vis_width("中文"), 4);
        assert_eq!(vis_width("运行模式"), 8);
        assert_eq!(vis_width("★"), 1);
        assert_eq!(vis_width("→"), 1);
    }

    #[test]
    fn emoji变体选择符按两列计() {
        // ♻️ / ➡️ 含 VS16，终端按 emoji 呈现占 2 列（不是 1）
        assert_eq!(vis_width("♻️"), 2);
        assert_eq!(vis_width("➡️"), 2);
        // 裸基字符仍按 1 列
        assert_eq!(vis_width("♻"), 1);
        // 组合后与宽字符混排的整行宽度：♻️(2) + 空格(1) + 故障切换(8) = 11
        assert_eq!(vis_width("♻️ 故障切换"), 11);
    }

    #[test]
    fn 宽度忽略_ansi_码() {
        let colored = "\x1b[32m中文\x1b[0m";
        assert_eq!(vis_width(colored), 4);
    }

    #[test]
    fn 截断按可见宽度并补省略号() {
        assert_eq!(truncate_visible("abcdef", 3), "abc");
        // 中文每字 2 列：4 列只能放 2 个字
        assert_eq!(truncate_visible("中文字符", 4), "中文");
        // 剩余 1 列时补 …（此处 3 个 ASCII + 1 列省略号 = 4 列）
        let t = truncate_visible("abc中文", 4);
        assert_eq!(vis_width(&t), 4);
        assert!(t.ends_with('…'));
        // 未超长则原样返回
        assert_eq!(truncate_visible("abc", 5), "abc");
    }

    #[test]
    fn 截断保留颜色并补重置() {
        let colored = "\x1b[32mabcdefgh\x1b[0m";
        let t = truncate_visible(colored, 3);
        assert!(t.starts_with("\x1b[32m"));
        assert!(t.ends_with("\x1b[0m"));
        assert_eq!(vis_width(&t), 3);
    }

    #[test]
    fn 控制字符被压成空格() {
        assert_eq!(one_line("a\nb\rc\td"), "a b c d");
        // ANSI 序列不受影响
        let colored = "\x1b[32m ok \x1b[0m";
        assert_eq!(one_line(colored), colored);
    }

    #[test]
    fn 行渲染可见宽度等于面板宽() {
        let b = Box::new(40);
        for line in [
            b.hline("Status", "shown=1"),
            b.hbot(),
            b.hdiv(),
            b.row("运行模式", "rule"),
            b.row("key", ""),
            b.row_badge("llm_failed", "3", "!"),
        ] {
            assert_eq!(vis_width(&line), 40, "行未对齐: {line:?}");
        }
    }

    #[test]
    fn 顶边中文标题不破边() {
        let b = Box::new(50);
        let line = b.hline("策略组", "shown=14");
        assert_eq!(vis_width(&line), 50);
    }

    #[test]
    fn 列宽弹性列填满内宽() {
        let cols = [
            Column::flex("节点", 10),
            Column::new("类型", 10),
            Column::new("延迟", 10),
        ];
        // 内宽 36，gaps=2 → flex = 36 - (10+10) - 2 = 14
        assert_eq!(super::column_widths(&cols, 36), vec![14, 10, 10]);
        // 内宽不足时按比例收缩，保证「列宽和 + 间隔」不超内宽
        let w = super::column_widths(&cols, 24);
        assert!(w.iter().sum::<usize>() + 2 <= 24);
        assert!(w.iter().all(|&x| x >= 4));
    }
}
