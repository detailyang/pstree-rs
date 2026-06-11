# pstree-rs

[![CI](https://github.com/detailyang/pstree-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/detailyang/pstree-rs/actions/workflows/ci.yml)

A `pstree` implementation for macOS, written in Rust.

- Uses `sysctl(KERN_PROC_ALL)` directly — no forking, no `ps` subprocess.
- Full command line via `KERN_PROCARGS2` (argv joined), falls back to `p_comm` for privileged processes.
- Single dependency: [`libc`](https://crates.io/crates/libc).
- Installable via [Nix flake](#install-via-nix).

## Example

```
$ pstree-rs -p $$
1 launchd
└─ 2544 tmux new-session -s main
   └─ 24398 -fish
      └─ 2526 pi
         └─ 32279 bash -c pstree-rs -p $$
            └─ 32280 pstree-rs -p 32279
```

```
$ pstree-rs --ascii -l 2
1 launchd
|-- 90 logd
|-- 92 UserEventAgent
|-- 94 fseventsd
|-- 98 systemstats
|   \-- 409 systemstats
|-- 102 configd
|   \-- 3851 eapolclient
...
```

## Usage

```
Usage: pstree-rs [-w] [--ascii] [-p pid] [-u user] [-l depth] [pid]

Options:
  pid        root pid to start from (default: 1)
  -p pid     show only branches containing pid
  -u user    show only branches containing processes owned by user
  -l depth   limit tree depth
  -w         wide output, no truncation (default: on)
  --ascii    use ASCII tree characters
  -h         show this help
```

## Install via Nix

```bash
nix profile install github:detailyang/pstree-rs
```

Run without installing:

```bash
nix run github:detailyang/pstree-rs
```

## Build from source

Requires Rust stable (1.65+).

```bash
cargo build --release
./target/release/pstree-rs
```

In a Nix dev shell:

```bash
nix develop
cargo build --release
```

## Platform

macOS only (Darwin). Uses `sysctl(KERN_PROC_ALL)` for process listing,
`KERN_PROCARGS2` for full command lines, and `ioctl(TIOCGWINSZ)` for terminal width.

System processes owned by root will show truncated names (16 chars) due to
macOS permission restrictions on `KERN_PROCARGS2`. Run with `sudo` to see full
command lines for all processes.

## License

MIT
