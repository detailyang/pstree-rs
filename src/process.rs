use std::collections::HashMap;

/// Minimal process info extracted from kinfo_proc.
#[derive(Debug, Clone)]
pub struct Process {
    pub pid: i32,
    pub ppid: i32,
    pub uid: u32,
    pub name: String,
}

// Darwin kinfo_proc field offsets (verified with clang on macOS arm64/x86_64):
//   sizeof(kinfo_proc)       = 648
//   offsetof(kp_proc.p_pid)  = 40
//   offsetof(kp_proc.p_comm) = 243  (MAXCOMLEN+1 = 17 bytes)
//   offsetof(kp_eproc.e_ppid)= 560
//   offsetof(kp_eproc.cr_uid)= 420
const KINFO_PROC_SIZE: usize = 648;
const OFFSET_PID: usize = 40;
const OFFSET_COMM: usize = 243;
const OFFSET_PPID: usize = 560;
const OFFSET_UID: usize = 420;
const MAXCOMLEN: usize = 16;

/// Read the full command line for `pid` via KERN_PROCARGS2 and return
/// argv joined by spaces (e.g. "tmux attach-session -t main").
/// Returns None on permission error or any parse failure; callers fall back to p_comm.
#[cfg(target_os = "macos")]
fn procargs2_cmdline(pid: i32) -> Option<String> {
    let mut mib: [libc::c_int; 3] = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid];
    let mut size: libc::size_t = 0;

    let ret = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            3,
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if ret != 0 || size < 4 {
        return None;
    }

    let mut buf: Vec<u8> = vec![0u8; size];
    let mut buf_size = size;
    let ret = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            3,
            buf.as_mut_ptr() as *mut libc::c_void,
            &mut buf_size,
            std::ptr::null_mut(),
            0,
        )
    };
    if ret != 0 {
        return None;
    }
    buf.truncate(buf_size);

    // Layout: [argc: i32][exec_path\0][null padding][argv[0]\0][argv[1]\0]...
    if buf.len() < 4 {
        return None;
    }
    let argc = i32::from_ne_bytes(buf[..4].try_into().unwrap());
    if argc <= 0 {
        return None;
    }

    let mut p = &buf[4..];

    // Skip exec_path.
    let nul = p.iter().position(|&b| b == 0)?;
    p = &p[nul + 1..];

    // Skip null padding between exec_path and argv.
    let non_null = p.iter().position(|&b| b != 0)?;
    p = &p[non_null..];

    // Collect argv[0..argc].
    let mut args: Vec<&str> = Vec::with_capacity(argc as usize);
    for _ in 0..argc {
        if p.is_empty() {
            break;
        }
        let nul = p.iter().position(|&b| b == 0).unwrap_or(p.len());
        if let Ok(s) = std::str::from_utf8(&p[..nul]) {
            if !s.is_empty() {
                args.push(s);
            }
        }
        if nul + 1 >= p.len() {
            break;
        }
        p = &p[nul + 1..];
    }

    if args.is_empty() {
        return None;
    }
    Some(args.join(" "))
}

/// Collect all processes via sysctl(KERN_PROC_ALL).
#[cfg(target_os = "macos")]
pub fn collect_processes() -> Result<Vec<Process>, String> {
    let mut mib: [libc::c_int; 4] = [libc::CTL_KERN, libc::KERN_PROC, libc::KERN_PROC_ALL, 0];
    let mut size: libc::size_t = 0;

    // First call: query required buffer size.
    let ret = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            4,
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if ret != 0 {
        return Err(format!(
            "sysctl(KERN_PROC_ALL) size query failed: {}",
            std::io::Error::last_os_error()
        ));
    }

    // Allocate raw byte buffer with headroom.
    let capacity = size + KINFO_PROC_SIZE * 16;
    let mut buf: Vec<u8> = vec![0u8; capacity];

    loop {
        let mut buf_size = buf.len();
        let ret = unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                4,
                buf.as_mut_ptr() as *mut libc::c_void,
                &mut buf_size,
                std::ptr::null_mut(),
                0,
            )
        };

        if ret == 0 {
            buf.truncate(buf_size);
            break;
        }

        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::ENOMEM) {
            let new_len = buf.len() + KINFO_PROC_SIZE * 64;
            buf.resize(new_len, 0);
            continue;
        }

        return Err(format!("sysctl(KERN_PROC_ALL) failed: {}", err));
    }

    let count = buf.len() / KINFO_PROC_SIZE;
    let mut processes = Vec::with_capacity(count);

    for i in 0..count {
        let base = i * KINFO_PROC_SIZE;
        let entry = &buf[base..base + KINFO_PROC_SIZE];

        let pid = i32::from_ne_bytes(entry[OFFSET_PID..OFFSET_PID + 4].try_into().unwrap());
        let ppid = i32::from_ne_bytes(entry[OFFSET_PPID..OFFSET_PPID + 4].try_into().unwrap());
        let uid = u32::from_ne_bytes(entry[OFFSET_UID..OFFSET_UID + 4].try_into().unwrap());

        let comm = &entry[OFFSET_COMM..OFFSET_COMM + MAXCOMLEN + 1];
        let nul = comm.iter().position(|&b| b == 0).unwrap_or(MAXCOMLEN + 1);
        let comm_name = String::from_utf8_lossy(&comm[..nul]).into_owned();

        // Try KERN_PROCARGS2 for the full argv joined by spaces; fall back to p_comm.
        let name = procargs2_cmdline(pid).unwrap_or(comm_name);

        processes.push(Process {
            pid,
            ppid,
            uid,
            name,
        });
    }

    Ok(processes)
}

/// Build adjacency maps from a flat process list.
/// Returns (pid->Process, pid->sorted children pids).
/// Orphan processes (ppid not in pid set) are re-parented to pid 1.
pub fn build_tree(processes: Vec<Process>) -> (HashMap<i32, Process>, HashMap<i32, Vec<i32>>) {
    let pid_set: std::collections::HashSet<i32> = processes.iter().map(|p| p.pid).collect();

    let mut proc_map: HashMap<i32, Process> = HashMap::with_capacity(processes.len());
    let mut children: HashMap<i32, Vec<i32>> = HashMap::with_capacity(processes.len());

    for p in processes {
        let pid = p.pid;
        let ppid = if pid == 0 || pid == 1 {
            // Kernel task and launchd are their own roots.
            p.ppid
        } else if p.ppid == pid || (p.ppid != 0 && !pid_set.contains(&p.ppid)) {
            // Self-parent or dangling ppid → re-parent to launchd.
            1
        } else {
            p.ppid
        };

        children.entry(ppid).or_default().push(pid);
        children.entry(pid).or_default(); // ensure entry exists
        proc_map.insert(pid, Process { ppid, ..p });
    }

    // Sort children by pid for deterministic output.
    for v in children.values_mut() {
        v.sort_unstable();
    }

    (proc_map, children)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proc(pid: i32, ppid: i32) -> Process {
        Process {
            pid,
            ppid,
            uid: 0,
            name: format!("proc{}", pid),
        }
    }

    #[test]
    fn orphan_reparented_to_1() {
        // pid 99 has ppid 50 which doesn't exist → should be reparented to 1.
        let procs = vec![make_proc(1, 0), make_proc(99, 50)];
        let (pm, ch) = build_tree(procs);
        assert_eq!(pm[&99].ppid, 1);
        assert!(ch[&1].contains(&99));
    }

    #[test]
    fn children_are_sorted() {
        let procs = vec![
            make_proc(1, 0),
            make_proc(5, 1),
            make_proc(2, 1),
            make_proc(3, 1),
        ];
        let (_, ch) = build_tree(procs);
        assert_eq!(ch[&1], vec![2, 3, 5]);
    }

    #[test]
    fn all_entries_have_children_key() {
        let procs = vec![make_proc(1, 0), make_proc(2, 1)];
        let (_, ch) = build_tree(procs);
        assert!(ch.contains_key(&1));
        assert!(ch.contains_key(&2));
    }
}
