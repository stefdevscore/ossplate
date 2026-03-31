# Ossplate

<p align="center">
  <img src="https://raw.githubusercontent.com/stefdevscore/ossplate/main/assets/illustrations/chestplate.svg" alt="Ossplate armor" width="320">
</p>

`ossplate` helps you start and maintain a project that ships the same CLI through Rust, npm, and PyPI.

Use it to:

- create a new scaffolded project
- initialize an existing directory
- validate project identity and metadata
- keep owned files in sync
- inspect the effective repo contract
- run the full repo gate in structured JSON

This package is the installed JavaScript delivery adapter for the same `ossplate` CLI described in the main docs. It forwards to the bundled native binary for your current platform and exposes the same subcommands as the Rust core.

Common commands:

```bash
ossplate version
ossplate create <target>
ossplate init --path <dir>
ossplate validate --json
ossplate inspect --json
ossplate sync --check --json
ossplate verify --json
```

Typical workflow:

```bash
ossplate create ../my-new-project \
  --name "My Project" \
  --repository "https://github.com/acme/my-project" \
  --author-name "Acme OSS" \
  --author-email "oss@acme.dev" \
  --rust-crate "my-project-core" \
  --npm-package "@acme/my-project" \
  --python-package "my-project-py" \
  --command "my-project"

ossplate validate --path ../my-new-project --json
ossplate inspect --path ../my-new-project --json
ossplate sync --path ../my-new-project --check --json
```

If you are working from a source checkout instead of an installed npm package, use the same subcommands through:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- <subcommand> ...
```

Learn more:

- [Main documentation](https://github.com/stefdevscore/ossplate/blob/main/docs/README.md)
- [Agent Operations](https://github.com/stefdevscore/ossplate/blob/main/docs/agent-operations.md)
- [Testing guide](https://github.com/stefdevscore/ossplate/blob/main/docs/testing.md)
- [Architecture](https://github.com/stefdevscore/ossplate/blob/main/docs/architecture.md)
