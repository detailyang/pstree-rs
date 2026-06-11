use pstree::{
    args::{parse_args, usage},
    filter::visible_pids,
    process::{build_tree, collect_processes},
    render::{render, RenderOptions},
    term::terminal_width,
    users::username_to_uid,
};

fn main() {
    let args = match parse_args(std::env::args()) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{}", e);
            eprintln!("{}", usage());
            std::process::exit(1);
        }
    };

    let processes = match collect_processes() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("pstree-rs: {}", e);
            std::process::exit(1);
        }
    };

    // Resolve username to uid if -u was given.
    let user_filter: Option<u32> = match &args.user_filter {
        None => None,
        Some(name) => match username_to_uid(name) {
            Some(uid) => Some(uid),
            None => {
                eprintln!("pstree-rs: unknown user: {}", name);
                std::process::exit(1);
            }
        },
    };

    let (proc_map, children) = build_tree(processes);

    // Validate root pid exists.
    if !proc_map.contains_key(&args.root_pid) {
        eprintln!("pstree-rs: pid {} not found", args.root_pid);
        std::process::exit(1);
    }

    let visible = visible_pids(
        &proc_map,
        &children,
        args.root_pid,
        args.pid_filter,
        user_filter,
        args.max_depth,
    );

    if visible.is_empty() {
        // No matches — nothing to print, exit cleanly.
        std::process::exit(0);
    }

    let width = if args.wide {
        None
    } else {
        terminal_width().or(Some(80))
    };

    let opts = RenderOptions {
        ascii: args.ascii,
        width,
    };
    let output = render(&proc_map, &children, &visible, args.root_pid, &opts);
    print!("{}", output);
}
