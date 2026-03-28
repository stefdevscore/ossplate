# Contributing

Use the root verification flow before committing changes:

```bash
./scripts/verify.sh
```

Optional local hook setup:

```bash
npm install
```

That enables the Husky `pre-push` hook, which runs `./scripts/verify.sh`.

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

Release bump rules:

- `feat:` triggers a minor bump
- `fix:`, `docs:`, `refactor:`, `test:`, `chore:`, `build:`, `ci:` and similar commits trigger a patch bump
- `!` in a conventional commit header or `BREAKING CHANGE` in the body triggers a major bump
- `[major]`, `[minor]`, and `[patch]` override the inferred bump level

Read this next:

- [`docs/architecture.md`](./docs/architecture.md)
- [`docs/testing.md`](./docs/testing.md)
- [`docs/releases.md`](./docs/releases.md)
- [`docs/adrs/0002-sync-owns-bounded-identity.md`](./docs/adrs/0002-sync-owns-bounded-identity.md)

Current ownership model:

- `ossplate.toml` is the canonical identity source
- `sync` owns Rust/npm/Python metadata surfaces
- `sync` owns the root `README.md` identity block between `ossplate:readme-identity` markers
- `sync` owns the top-level workflow display names between `ossplate:workflow-name` markers
- workflow logic, auth, triggers, and job structure remain manual
