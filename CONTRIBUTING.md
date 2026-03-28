# Contributing

Use the root verification flow before committing changes:

```bash
./scripts/verify.sh
```

That script mirrors the local release-confidence gate:

- Rust format, clippy, and tests
- `ossplate validate` and `ossplate sync --check`
- JavaScript wrapper tests and package dry-run
- Python wrapper tests

Useful operator commands:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- validate
cargo run --manifest-path core-rs/Cargo.toml -- sync --check
cargo run --manifest-path core-rs/Cargo.toml -- sync
```

Current ownership model:

- `ossplate.toml` is the canonical identity source
- `sync` owns Rust/npm/Python metadata surfaces
- `sync` owns the root `README.md` identity block between `ossplate:readme-identity` markers
- `sync` owns the top-level workflow display names between `ossplate:workflow-name` markers
- workflow logic, auth, triggers, and job structure remain manual
