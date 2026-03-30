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

Learn more:

- [Main documentation](https://github.com/stefdevscore/ossplate/blob/main/docs/README.md)
- [Agent Operations](https://github.com/stefdevscore/ossplate/blob/main/docs/agent-operations.md)
- [Testing guide](https://github.com/stefdevscore/ossplate/blob/main/docs/testing.md)
- [Architecture](https://github.com/stefdevscore/ossplate/blob/main/docs/architecture.md)
