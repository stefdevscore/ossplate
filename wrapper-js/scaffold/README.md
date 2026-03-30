<!-- ossplate:readme-identity:start -->
# Ossplate

Scaffold and maintain one Rust-core CLI that ships cleanly through Cargo, npm, and PyPI.
<!-- ossplate:readme-identity:end -->

<p align="center">
  <img src="https://raw.githubusercontent.com/stefdevscore/ossplate/main/assets/illustrations/chestplate.svg" alt="Ossplate armor" width="360">
</p>

`ossplate` helps maintainers and agents start and keep a single CLI aligned across Rust, npm, and PyPI.

It gives you a working baseline with:

- one real core CLI
- thin JavaScript and Python wrappers
- release-ready workflows for Cargo, npm, and PyPI
- a scaffold you can create, adopt, and keep in sync
- machine-checkable validation, planning, repair, inspection, and verification commands for agent loops

## What It Does

Use `ossplate` when you want a single command-line tool to exist cleanly in multiple ecosystems without maintaining three separate implementations.

It can:

- create a new scaffolded project
- initialize an existing directory with the expected structure
- validate project identity and metadata
- synchronize the files it owns
- inspect effective repo contracts
- plan publish behavior and local preflight state without mutation
- run the full repo gate as a structured JSON contract

## Best Fit

`ossplate` is optimized for projects with this structure:

- one Rust-core CLI
- thin JavaScript and Python wrappers
- multi-registry distribution through Cargo, npm, and PyPI
- deterministic ownership, validation, and sync contracts

When an AI agent is driving repo setup or maintenance, the high-signal loop is:

- `cargo run --manifest-path core-rs/Cargo.toml -- create <target>` to produce a coherent Rust-core baseline
- `cargo run --manifest-path core-rs/Cargo.toml -- validate --json` to inspect repo health in machine-readable form
- `cargo run --manifest-path core-rs/Cargo.toml -- inspect --json` to read the effective config, owned files, and derived runtime contract
- `cargo run --manifest-path core-rs/Cargo.toml -- sync --check --json` or `-- sync --plan --json` to inspect bounded drift
- `cargo run --manifest-path core-rs/Cargo.toml -- sync --json` to apply bounded repairs with a structured result
- `cargo run --manifest-path core-rs/Cargo.toml -- publish --plan --json` to inspect local publish preflight without side effects
- `cargo run --manifest-path core-rs/Cargo.toml -- verify --json` to run the full repo gate with per-step structured results

## Quick Start

```bash
cargo run --manifest-path core-rs/Cargo.toml -- create ../my-new-project \
  --name "My Project" \
  --repository "https://github.com/acme/my-project" \
  --author-name "Acme" \
  --author-email "oss@acme.dev" \
  --rust-crate "my-project" \
  --npm-package "@acme/my-project" \
  --python-package "my-project-py" \
  --command "my-project"
```

Then check that everything is aligned:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- validate
cargo run --manifest-path core-rs/Cargo.toml -- sync --check --json
```

For a generated-repo-safe operations guide, see [docs/agent-operations.md](https://github.com/stefdevscore/ossplate/blob/main/docs/agent-operations.md).

## Core Commands

```bash
cargo run --manifest-path core-rs/Cargo.toml -- version
cargo run --manifest-path core-rs/Cargo.toml -- create <target>
cargo run --manifest-path core-rs/Cargo.toml -- init --path <dir>
cargo run --manifest-path core-rs/Cargo.toml -- validate --json
cargo run --manifest-path core-rs/Cargo.toml -- inspect --json
cargo run --manifest-path core-rs/Cargo.toml -- sync --plan --json
cargo run --manifest-path core-rs/Cargo.toml -- sync --json
cargo run --manifest-path core-rs/Cargo.toml -- publish --plan --json
cargo run --manifest-path core-rs/Cargo.toml -- verify --json
```

The same command surface is available through the Rust binary and the packaged JS/Python wrappers.

## Why It’s Useful

- You keep one source of truth for CLI behavior.
- You avoid drift between Rust, npm, and PyPI releases.
- You get a real scaffold instead of a fake demo project.
- You can publish with modern registry workflows instead of assembling release plumbing from scratch.
- You give AI agents a deterministic contract for bootstrap, validation, and repair.

## Learn More

- [Documentation](https://github.com/stefdevscore/ossplate/blob/main/docs/README.md)
- [Agent Operations](https://github.com/stefdevscore/ossplate/blob/main/docs/agent-operations.md)
- [Adoption Guide](https://github.com/stefdevscore/ossplate/blob/main/docs/customizing-the-template.md)
- [Testing Guide](https://github.com/stefdevscore/ossplate/blob/main/docs/testing.md)
- [Release Guide](https://github.com/stefdevscore/ossplate/blob/main/docs/releases.md)
- [Architecture](https://github.com/stefdevscore/ossplate/blob/main/docs/architecture.md)

## License

Licensed under the [Unlicense](LICENSE).
