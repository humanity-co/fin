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

If rustup's default toolchain is broken, use the direct toolchain paths:
```bash
/tmp/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo check
```

## Known Issues
- `rustup default stable` fails due to /home disk space during component installation.
- If `~/.cargo` exists and fills up, delete it and recreate the symlink.
- After `cargo check`, clean up with `rm -rf /tmp/sutra-target`.
