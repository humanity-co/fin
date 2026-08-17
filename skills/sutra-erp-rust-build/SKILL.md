---
name: sutra-erp-rust-build
description: How to build/check the SutraERP Rust project in the sandbox — works around disk space constraints on /home.
---

# SutraERP Rust Build

## Problem
The sandbox `/home` partition is only 300MB, which is too small for Rust's `target/` and `~/.cargo/` directories.

## Solution
1. Rust toolchain is installed in `/tmp/cargo` and `/tmp/rustup`.
2. `~/.cargo` is symlinked to `/tmp/cargo`.
3. Target directory is set to `/tmp/sutra-target` via `.cargo/config.toml`.

## Build/Check Commands
```bash
source /tmp/cargo/env
# Ensure symlink exists
rm -rf ~/.cargo && ln -sf /tmp/cargo ~/.cargo
cd /home/team/shared/sutra-erp
rm -rf /tmp/sutra-target
cargo check
```

If rustup's default toolchain is broken, use the direct toolchain paths AND
export the toolchain bin dir on PATH so cargo can find rustc:
```bash
export PATH=/tmp/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH
cd /home/team/shared/sutra-erp
cargo check
```
(Using the direct cargo binary without PATH yields
`could not execute process 'rustc -vV' ... No such file or directory`.)

## Known Issues
- `rustup default stable` fails due to /home disk space during component installation.
- If `~/.cargo` exists and fills up, delete it and recreate the symlink.
- After `cargo check`, clean up with `rm -rf /tmp/sutra-target`.
- Cold builds on a fresh sandbox fail with `cannot find -lgcc` if the GCC dev
  runtime is missing (`libgcc.a` absent from /usr/lib/gcc/x86_64-linux-gnu/13/).
  Fix: `apt-get update && apt-get install -y libgcc-13-dev` (network to
  archive.ubuntu.com is open; crates.io also works). Then rerun cargo check.

