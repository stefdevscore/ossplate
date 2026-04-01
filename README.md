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
- a scaffold you can create, upgrade, adopt, and keep in sync
- machine-checkable validation, planning, repair, inspection, and verification commands for agent loops

## Installed Usage

```bash
ossplate version
ossplate create ../my-new-project \
  --name "My Project" \
  --repository "https://github.com/acme/my-project" \
  --author-name "Acme" \
  --author-email "oss@acme.dev" \
  --rust-crate "my-project" \
  --npm-package "@acme/my-project" \
  --python-package "my-project-py" \
  --command "my-project"
```

## Source Checkout Usage

- `cargo run --manifest-path core-rs/Cargo.toml -- create <target>`
- `cargo run --manifest-path core-rs/Cargo.toml -- validate --json`
- `cargo run --manifest-path core-rs/Cargo.toml -- inspect --json`
- `cargo run --manifest-path core-rs/Cargo.toml -- upgrade --plan --json`
- `cargo run --manifest-path core-rs/Cargo.toml -- sync --check --json`
- `cargo run --manifest-path core-rs/Cargo.toml -- publish --plan --json`
- `cargo run --manifest-path core-rs/Cargo.toml -- verify --json`

## Learn More

- [Documentation](https://github.com/stefdevscore/ossplate/blob/main/docs/README.md)
- [Agent Operations](https://github.com/stefdevscore/ossplate/blob/main/docs/agent-operations.md)
- [Adoption Guide](https://github.com/stefdevscore/ossplate/blob/main/docs/customizing-the-template.md)
- [Testing Guide](https://github.com/stefdevscore/ossplate/blob/main/docs/testing.md)
- [Release Guide](https://github.com/stefdevscore/ossplate/blob/main/docs/releases.md)
- [Architecture](https://github.com/stefdevscore/ossplate/blob/main/docs/architecture.md)

## License

Licensed under the [Unlicense](LICENSE).
