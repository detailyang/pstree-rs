use std::collections::{HashMap, HashSet};

use crate::process::Process;

pub struct RenderOptions {
    pub ascii: bool,
    /// None means no truncation.
    pub width: Option<usize>,
}

struct Chars {
    branch: &'static str, // ├─
    last: &'static str,   // └─
    pipe: &'static str,   // │
    indent: &'static str, // space continuation
}

const UTF8_CHARS: Chars = Chars {
    branch: "├─ ",
    last: "└─ ",
    pipe: "│  ",
    indent: "   ",
};

const ASCII_CHARS: Chars = Chars {
    branch: "|-- ",
    last: "\\-- ",
    pipe: "|   ",
    indent: "    ",
};

/// Read-only rendering context threaded through the recursive traversal.
struct RenderCtx<'a> {
    proc_map: &'a HashMap<i32, Process>,
    children: &'a HashMap<i32, Vec<i32>>,
    visible: &'a HashSet<i32>,
    chars: &'a Chars,
    opts: &'a RenderOptions,
}

pub fn render(
    proc_map: &HashMap<i32, Process>,
    children: &HashMap<i32, Vec<i32>>,
    visible: &HashSet<i32>,
    root: i32,
    opts: &RenderOptions,
) -> String {
    let chars = if opts.ascii {
        &ASCII_CHARS
    } else {
        &UTF8_CHARS
    };
    let ctx = RenderCtx {
        proc_map,
        children,
        visible,
        chars,
        opts,
    };
    let mut out = String::with_capacity(4096);
    render_node(&ctx, root, &[], &mut out);
    out
}

fn render_node(ctx: &RenderCtx<'_>, pid: i32, prefix_stack: &[bool], out: &mut String) {
    let p = match ctx.proc_map.get(&pid) {
        Some(p) => p,
        None => return,
    };

    // Build prefix string from ancestor continuation flags.
    let mut prefix = String::new();
    for (i, &has_more) in prefix_stack.iter().enumerate() {
        if i == prefix_stack.len() - 1 {
            // Last level: draw branch connector.
            prefix.push_str(if has_more {
                ctx.chars.branch
            } else {
                ctx.chars.last
            });
        } else {
            prefix.push_str(if has_more {
                ctx.chars.pipe
            } else {
                ctx.chars.indent
            });
        }
    }

    let line = format!("{}{} {}", prefix, p.pid, p.name);
    let line = match ctx.opts.width {
        Some(w) if w > 0 => truncate_to_width(&line, w),
        _ => line,
    };
    out.push_str(&line);
    out.push('\n');

    // Collect visible children in sorted order.
    let visible_children: Vec<i32> = ctx
        .children
        .get(&pid)
        .map(|kids| {
            kids.iter()
                .copied()
                .filter(|c| ctx.visible.contains(c))
                .collect()
        })
        .unwrap_or_default();

    let n = visible_children.len();
    for (i, &child) in visible_children.iter().enumerate() {
        let has_more = i < n - 1;
        let mut next_stack = prefix_stack.to_vec();
        next_stack.push(has_more);
        render_node(ctx, child, &next_stack, out);
    }
}

/// Truncate a string to at most `max_cols` display columns.
/// Handles multi-byte UTF-8 by character boundary.
fn truncate_to_width(s: &str, max_cols: usize) -> String {
    // Simple heuristic: treat each char as 1 column (no CJK width handling needed
    // for process names).
    if s.chars().count() <= max_cols {
        return s.to_owned();
    }
    s.chars().take(max_cols).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::Process;

    fn make_proc(pid: i32, ppid: i32, name: &str) -> Process {
        Process {
            pid,
            ppid,
            uid: 0,
            name: name.to_string(),
        }
    }

    fn make_tree() -> (HashMap<i32, Process>, HashMap<i32, Vec<i32>>, HashSet<i32>) {
        let procs = vec![
            make_proc(1, 0, "launchd"),
            make_proc(2, 1, "logd"),
            make_proc(3, 1, "configd"),
            make_proc(4, 2, "worker"),
        ];
        let (pm, ch) = crate::process::build_tree(procs);
        let visible: HashSet<i32> = pm.keys().copied().collect();
        (pm, ch, visible)
    }

    #[test]
    fn utf8_tree_structure() {
        let (pm, ch, visible) = make_tree();
        let opts = RenderOptions {
            ascii: false,
            width: None,
        };
        let out = render(&pm, &ch, &visible, 1, &opts);
        assert!(out.contains("├─"), "should use UTF-8 branch char");
        assert!(out.contains("└─"), "should use UTF-8 last char");
        // Root has no prefix.
        assert!(out.starts_with("1 launchd"), "root line has no prefix");
    }

    #[test]
    fn ascii_tree_structure() {
        let (pm, ch, visible) = make_tree();
        let opts = RenderOptions {
            ascii: true,
            width: None,
        };
        let out = render(&pm, &ch, &visible, 1, &opts);
        assert!(out.contains("|--"), "should use ASCII branch char");
    }

    #[test]
    fn truncation_applied() {
        let (pm, ch, visible) = make_tree();
        let opts = RenderOptions {
            ascii: false,
            width: Some(10),
        };
        let out = render(&pm, &ch, &visible, 1, &opts);
        for line in out.lines() {
            assert!(line.chars().count() <= 10, "line too long: {:?}", line);
        }
    }

    #[test]
    fn pid_and_name_on_each_line() {
        let (pm, ch, visible) = make_tree();
        let opts = RenderOptions {
            ascii: false,
            width: None,
        };
        let out = render(&pm, &ch, &visible, 1, &opts);
        assert!(out.contains("2 logd"));
        assert!(out.contains("4 worker"));
    }
}
