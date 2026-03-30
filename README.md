<!-- ossplate:readme-identity:start -->
# Ossplate

Scaffold and maintain one Rust-core CLI that ships cleanly through Cargo, npm, and PyPI.
<!-- ossplate:readme-identity:end -->

<p align="center">
  <img src="https://raw.githubusercontent.com/stefdevscore/ossplate/main/assets/illustrations/chestplate.svg" alt="Ossplate armor" width="360">
</p>

`ossplate` helps agents and maintainers start and keep a single CLI aligned across Rust, npm, and PyPI.

It gives you a working baseline with:

- one real core CLI
- thin JavaScript and Python wrappers
- release-ready workflows for Cargo, npm, and PyPI
- a scaffold you can create, adopt, and keep in sync
- machine-checkable validation and sync commands for agent loops

## What It Does

Use `ossplate` when you want a single command-line tool to exist cleanly in multiple ecosystems without maintaining three separate implementations.

It can:

- create a new scaffolded project
- initialize an existing directory with the expected structure
- validate project identity and metadata
- synchronize the files it owns

## Agentic AI Fit

`ossplate` is especially useful when an AI agent is driving repo setup or maintenance and needs a narrow, deterministic contract instead of vague prose.

The core loop for agents is:

- `ossplate create <target>` to produce a coherent pattern-1 baseline
- `ossplate validate --json` to inspect repo health in machine-readable form
- `ossplate sync --check` to confirm owned metadata is aligned without mutating state
- `ossplate sync` to repair bounded identity drift when needed

That lets an agent bootstrap, inspect, and repair the repo without inventing its own policy layer.

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
cargo run --manifest-path core-rs/Cargo.toml -- sync --check
```

For an agent-facing walkthrough, see [docs/agents.md](https://github.com/stefdevscore/ossplate/blob/main/docs/agents.md).

## Core Commands

```bash
ossplate version
ossplate create <target>
ossplate init --path <dir>
ossplate validate
ossplate sync --check
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
- [Agent Guide](https://github.com/stefdevscore/ossplate/blob/main/docs/agents.md)
- [Adoption Guide](https://github.com/stefdevscore/ossplate/blob/main/docs/customizing-the-template.md)
- [Testing Guide](https://github.com/stefdevscore/ossplate/blob/main/docs/testing.md)
- [Release Guide](https://github.com/stefdevscore/ossplate/blob/main/docs/releases.md)
- [Architecture](https://github.com/stefdevscore/ossplate/blob/main/docs/architecture.md)

## License

Licensed under the [Unlicense](LICENSE).
