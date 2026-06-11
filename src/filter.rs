use std::collections::{HashMap, HashSet};

use crate::process::Process;

/// Returns the set of pids that should be visible given the filter criteria.
/// A pid is visible if it is on the path from the root to any matching node,
/// or is a descendant of a matching node.
///
/// If no filters are active (pid_filter=None, user_filter=None), all pids are visible.
pub fn visible_pids(
    proc_map: &HashMap<i32, Process>,
    children: &HashMap<i32, Vec<i32>>,
    root: i32,
    pid_filter: Option<i32>,
    user_filter: Option<u32>,
    max_depth: Option<usize>,
) -> HashSet<i32> {
    // No filters: collect all reachable pids from root up to max_depth.
    if pid_filter.is_none() && user_filter.is_none() {
        let mut visible = HashSet::new();
        collect_subtree(children, root, 0, max_depth, &mut visible);
        return visible;
    }

    // With filters: find matching pids, then mark ancestors + subtree.
    let mut matching: HashSet<i32> = HashSet::new();

    // Collect all reachable pids first.
    let mut reachable: HashSet<i32> = HashSet::new();
    collect_subtree(children, root, 0, None, &mut reachable);

    for &pid in &reachable {
        let p = match proc_map.get(&pid) {
            Some(p) => p,
            None => continue,
        };
        let pid_match = pid_filter.is_none_or(|f| f == pid);
        let user_match = user_filter.is_none_or(|f| f == p.uid);
        if pid_match && user_match {
            matching.insert(pid);
        }
    }

    let mut visible: HashSet<i32> = HashSet::new();

    for &target in &matching {
        // Mark ancestors from root down to target.
        mark_ancestors(proc_map, root, target, &mut visible);
        // Mark full subtree of target (respecting max_depth relative to target).
        collect_subtree(children, target, 0, max_depth, &mut visible);
    }

    visible
}

/// Recursively collect all pids in subtree rooted at `pid`.
fn collect_subtree(
    children: &HashMap<i32, Vec<i32>>,
    pid: i32,
    depth: usize,
    max_depth: Option<usize>,
    out: &mut HashSet<i32>,
) {
    if let Some(max) = max_depth {
        if depth > max {
            return;
        }
    }
    out.insert(pid);
    if let Some(kids) = children.get(&pid) {
        for &child in kids {
            collect_subtree(children, child, depth + 1, max_depth, out);
        }
    }
}

/// Walk from root toward target, marking each node on the path as visible.
/// Returns true if target was found in this subtree.
fn mark_ancestors(
    proc_map: &HashMap<i32, Process>,
    current: i32,
    target: i32,
    visible: &mut HashSet<i32>,
) -> bool {
    if current == target {
        visible.insert(current);
        return true;
    }
    // Use ppid chain upward from target to root — more efficient than
    // recursive descent for deep trees.
    let mut cursor = target;
    let mut path = vec![cursor];
    loop {
        let p = match proc_map.get(&cursor) {
            Some(p) => p,
            None => return false,
        };
        if p.ppid == cursor || p.ppid == 0 {
            break;
        }
        cursor = p.ppid;
        path.push(cursor);
        if cursor == current {
            // Found the root of our subtree.
            for pid in path {
                visible.insert(pid);
            }
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::Process;

    fn make_proc(pid: i32, ppid: i32, uid: u32) -> Process {
        Process {
            pid,
            ppid,
            uid,
            name: format!("proc{}", pid),
        }
    }

    fn make_tree() -> (HashMap<i32, Process>, HashMap<i32, Vec<i32>>) {
        // 1 -> 2 -> 4
        //        -> 5
        //   -> 3
        let procs = vec![
            make_proc(1, 0, 0),
            make_proc(2, 1, 1000),
            make_proc(3, 1, 0),
            make_proc(4, 2, 1000),
            make_proc(5, 2, 0),
        ];
        crate::process::build_tree(procs)
    }

    #[test]
    fn no_filter_returns_all() {
        let (pm, ch) = make_tree();
        let v = visible_pids(&pm, &ch, 1, None, None, None);
        assert_eq!(v.len(), 5);
    }

    #[test]
    fn pid_filter_includes_ancestors_and_subtree() {
        let (pm, ch) = make_tree();
        // Filter for pid=4: should include 1->2->4
        let v = visible_pids(&pm, &ch, 1, Some(4), None, None);
        assert!(v.contains(&1), "root must be visible");
        assert!(v.contains(&2), "ancestor must be visible");
        assert!(v.contains(&4), "target must be visible");
        assert!(!v.contains(&3), "unrelated branch must be hidden");
        assert!(!v.contains(&5), "sibling must be hidden");
    }

    #[test]
    fn user_filter_includes_ancestors() {
        let (pm, ch) = make_tree();
        // uid=1000: pids 2 and 4
        let v = visible_pids(&pm, &ch, 1, None, Some(1000), None);
        assert!(v.contains(&1));
        assert!(v.contains(&2));
        assert!(v.contains(&4));
        // pid 3 uid=0, pid 5 uid=0 — not matched, but 5 is descendant of matched 2
        // subtree of 2 includes 5
        assert!(v.contains(&5));
        assert!(!v.contains(&3));
    }

    #[test]
    fn max_depth_limits_output() {
        let (pm, ch) = make_tree();
        let v = visible_pids(&pm, &ch, 1, None, None, Some(1));
        assert!(v.contains(&1));
        assert!(v.contains(&2));
        assert!(v.contains(&3));
        assert!(!v.contains(&4));
        assert!(!v.contains(&5));
    }
}
