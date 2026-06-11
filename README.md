# pstree-rs

A `pstree` implementation for macOS, written in Rust.

Uses `sysctl(KERN_PROC_ALL)` directly — no forking, no `ps` parsing.

## Usage

```
pstree-rs [-w] [--ascii] [-p pid] [-u user] [-l depth] [pid]

  pid        root pid to start from (default: 1)
  -p pid     show only branches containing pid
  -u user    show only branches containing processes owned by user
  -l depth   limit tree depth
  -w         wide output, no truncation
  --ascii    use ASCII tree characters
```

## Install via Nix

```bash
nix profile install github:yourusername/pstree-rs
```

Or run directly:

```bash
nix run github:yourusername/pstree-rs
```

## Build from source

```bash
cargo build --release
```

In a Nix devShell:

```bash
nix develop
cargo build --release
```
