# Releases

Use this guide when cutting or recovering an `ossplate` release.

## Current Registry Model

- crates.io publishes `ossplate`
- npm publishes `ossplate` plus runtime packages
- PyPI publishes `ossplate`
- the CLI name is `ossplate`

Current npm runtime package names for this repository are:

- `ossplate-darwin-arm64`
- `ossplate-darwin-x64`
- `ossplate-linux-x64`
- `ossplate-windows-x64`

Run the full gate before releasing:

```bash
./scripts/verify.sh
cargo run --manifest-path core-rs/Cargo.toml -- publish --plan --json
cargo run --manifest-path core-rs/Cargo.toml -- verify --json
```
