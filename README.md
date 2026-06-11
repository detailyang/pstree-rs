# pstree-rs

[![CI](https://github.com/detailyang/pstree-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/detailyang/pstree-rs/actions/workflows/ci.yml)

A `pstree` implementation for macOS, written in Rust.

- Uses `sysctl(KERN_PROC_ALL)` directly — no forking, no `ps` subprocess.
- Single dependency: [`libc`](https://crates.io/crates/libc).
- Installable via [Nix flake](#install-via-nix).

## Example

```
$ pstree-rs -p $$
1 launchd
└─ 2544 tmux
   └─ 24398 fish
      └─ 2526 node
         └─ 24756 bash
            └─ 24757 pstree-rs
```

```
$ pstree-rs -l 2
1 launchd
├─ 90 logd
├─ 92 UserEventAgent
├─ 94 fseventsd
├─ 98 systemstats
│  └─ 409 systemstats
├─ 102 configd
│  └─ 3851 eapolclient
...
```

## Usage

```
pstree-rs [-w] [--ascii] [-p pid] [-u user] [-l depth] [pid]

  pid        root pid to start from (default: 1)
  -p pid     show only branches containing pid
  -u user    show only branches containing processes owned by user
  -l depth   limit tree depth
  -w         wide output, no truncation
  --ascii    use ASCII tree characters instead of UTF-8
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

macOS only (Darwin). Uses `sysctl(KERN_PROC_ALL)` and `ioctl(TIOCGWINSZ)`.

## License

MIT
