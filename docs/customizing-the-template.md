# Adoption Guide

Use this guide after creating or cloning a project managed by `ossplate`.

The adoption rule is simple:

- `ossplate.toml` is the shared identity source of truth
- `sync` rewrites owned surfaces back into alignment
- `validate` checks that the owned surfaces still match
- `upgrade` is the structural path for supported scaffold-version transitions

## ADOPT-01 Canonical Source Of Truth

`ossplate.toml` currently owns:

- project name
- project description
- repository URL
- license
- author name and email
- Rust crate name
- npm package name
- Python package name
- CLI command name
- projected package discoverability metadata for Cargo, npm, and PyPI manifests

## ADOPT-02 Required Identity Changes

Replace these inherited defaults before reuse:

| Surface | Current scaffold value | Where it lives |
| --- | --- | --- |
| Rust crate name | `ossplate` | `core-rs/Cargo.toml` |
| npm package name | `ossplate` | `wrapper-js/package.json` |
| PyPI package name | `ossplate` | `wrapper-py/pyproject.toml` |
| CLI command | `ossplate` | `ossplate.toml`, `wrapper-js/package.json`, `wrapper-py/pyproject.toml` |
| Repository URL | `https://github.com/stefdevscore/ossplate` | Rust, npm, and Python metadata |
| Author/email | `Stef <stefdevscore@github.com>` / `stefdevscore@github.com` | Rust, npm, and Python metadata |
| Package-facing branding | `ossplate` identity in wrapper docs | `wrapper-js/README.md`, `wrapper-py/README.md` |

Scoped npm packages are supported. If `npm_package` is scoped, the generated runtime package names follow the same configured package identity plus the target suffix.

## ADOPT-03 What `validate` And `sync` Own

Owned today:

- Cargo, npm, and Python metadata fields
- wrapper package README identity
- the root README identity block
- workflow display names between `ossplate:workflow-name` markers

Not owned today:

- workflow logic
- publish auth logic
- arbitrary docs prose
- content outside bounded markers

Implementation note:

- runtime package folder paths under `wrapper-js/platform-packages/` are scaffold internals
- published runtime package names are derived from the configured npm package identity, not from a hard-coded `ossplate-*` public naming rule

## ADOPT-04 First Adoption Pass

1. Update `ossplate.toml` directly or use `create` / `init` with identity flags.
2. Run `cargo run --manifest-path core-rs/Cargo.toml -- inspect --json` to confirm the effective identity and owned contract.
3. If the repo is an older supported descendant, run `cargo run --manifest-path core-rs/Cargo.toml -- upgrade --plan --json`.
4. Run `cargo run --manifest-path core-rs/Cargo.toml -- sync --plan --json` or `-- sync --json`.
5. Run `cargo run --manifest-path core-rs/Cargo.toml -- validate --json`.
6. Run the verification flow from [Testing](./testing.md).
7. Only then expand product code or publish configuration.

## ADOPT-05 Create A New Project

Use:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- create ../my-new-project \
  --name "My Project" \
  --repository "https://github.com/acme/my-project" \
  --author-name "Acme OSS" \
  --author-email "oss@acme.dev" \
  --rust-crate "my-project-core" \
  --npm-package "@acme/my-project" \
  --python-package "my-project-py" \
  --command "my-project"
```

`create`:

- copies the curated scaffold payload into the target directory
- applies identity overrides to `ossplate.toml`
- runs `sync` on the new target
- can return the effective generated identity through `--json`
- rejects non-empty targets
- rejects targets inside the source template tree

## ADOPT-06 Adopt An Existing Directory

Use:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- init \
  --path ../existing-project \
  --name "Existing Project" \
  --command "existing-project"
```

`init`:

- ensures the expected scaffold layout exists
- copies any missing scaffold files
- applies requested identity overrides
- runs `sync` so owned metadata matches `ossplate.toml`
- can return the effective initialized identity through `--json`

`init` is intentionally narrow. It hydrates repos that already match the expected scaffold shape. It is not the general-purpose path for cross-generation scaffold migration.

## ADOPT-07 Upgrade Supported Descendants

Use:

```bash
cargo run --manifest-path core-rs/Cargo.toml -- upgrade --plan --json
cargo run --manifest-path core-rs/Cargo.toml -- upgrade --json
```

`upgrade`:

- detects scaffold compatibility and current scaffold version
- resolves a chained `x -> x+1 -> ... -> latest` path only across explicitly supported versioned transitions
- upgrades unversioned descendants only when they exactly match a known historical scaffold fingerprint
- updates managed scaffold surfaces to the current scaffold version
- returns structured step plans and changed-file output in `--json` mode

If a repo is older than the supported upgrade window or does not match a recognized descendant shape, prefer recreate over heuristic mutation.

## ADOPT-08 Identity Flags

- `--name`
- `--description`
- `--repository`
- `--license`
- `--author-name`
- `--author-email`
- `--rust-crate`
- `--npm-package`
- `--python-package`
- `--command`

## ADOPT-09 Related Decisions

- [ADR 0002: Sync Owns Bounded Identity Surfaces](./adrs/0002-sync-owns-bounded-identity.md)
- [ADR 0003: Ship A Curated Scaffold Payload](./adrs/0003-curated-scaffold-payload.md)
- [ADR 0015: Agent-First Machine-Readable Repo Contract](./adrs/0015-agent-first-machine-readable-repo-contract.md)
