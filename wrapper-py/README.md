# Ossplate

<p align="center">
  <img src="https://raw.githubusercontent.com/stefdevscore/ossplate/dev/assets/illustrations/armour05platemail.png" alt="Ossplate armor" width="320">
</p>

`ossplate` helps you start and maintain a project that ships the same CLI through Rust, npm, and PyPI.

Use it to:

- create a new scaffolded project
- initialize an existing directory
- validate project identity and metadata
- keep owned files in sync

Common commands:

```bash
ossplate version
ossplate create <target>
ossplate init --path <dir>
ossplate validate
ossplate sync --check
```

Learn more:

- [Main documentation](../docs/README.md)
- [Testing guide](../docs/testing.md)
- [Architecture](../docs/architecture.md)
