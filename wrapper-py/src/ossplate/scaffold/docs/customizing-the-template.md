# Adoption Guide

Use this guide after creating or cloning a project managed by `ossplate`.

The adoption rule is simple:

- `ossplate.toml` is the shared identity source of truth
- `sync` rewrites owned surfaces back into alignment
- `validate` checks that the owned surfaces still match

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

## ADOPT-04 First Adoption Pass

1. Update `ossplate.toml` directly or use `create` / `init` with identity flags.
2. Run `cargo run --manifest-path core-rs/Cargo.toml -- sync`.
3. Run `cargo run --manifest-path core-rs/Cargo.toml -- validate`.
4. Run the verification flow from [Testing](./testing.md).
5. Only then expand product code or publish configuration.

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

## ADOPT-07 Identity Flags

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

## ADOPT-08 Related Decisions

- [ADR 0002: Sync Owns Bounded Identity Surfaces](./adrs/0002-sync-owns-bounded-identity.md)
- [ADR 0003: Ship A Curated Scaffold Payload](./adrs/0003-curated-scaffold-payload.md)
