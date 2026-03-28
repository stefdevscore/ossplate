# Phase 1 Contract

Phase 1 establishes a single canonical CLI surface in the Rust core. The JavaScript and Python packages are wrappers that delegate to that binary rather than implementing separate behavior.

## Canonical Commands

- `version`
  returns JSON with the tool identity and package version
- `validate [--path <dir>] [--json]`
  returns validation results for owned metadata surfaces
- `sync [--path <dir>] [--check]`
  rewrites or checks owned metadata surfaces against the canonical project identity
- `create <target>`
  copies the curated scaffold payload into a clean target directory, applies optional identity overrides, and synchronizes the owned metadata surfaces there
- `init [--path <dir>]`
  hydrates an existing directory with the expected scaffold layout, applies optional identity overrides, and then synchronizes owned metadata in place

Both commands are available from source checkouts and installed wrapper artifacts that include the staged scaffold payload.

## Expected Output Shapes

```json
{"tool":"ossplate","version":"0.1.0"}
{"ok":true,"issues":[]}
```

## Wrapper Model

- Rust is the source of truth for command parsing and output.
- JavaScript and Python are adapter layers that resolve a packaged binary and forward arguments unchanged.
- Wrappers preserve stdout, stderr, and exit status from the core binary.

## Local Development Override

Both wrappers support `OSSPLATE_BINARY` to bypass packaged binary lookup during local development and parity testing.

`create` and `init` also honor `OSSPLATE_TEMPLATE_ROOT` when the template source needs to be overridden explicitly.

## Packaged Binary Layout

Wrappers expect binaries under:

```text
bin/<target>/<executable>
```

Supported target identifiers are aligned across wrappers:

- `darwin-arm64`
- `darwin-x64`
- `linux-x64`
- `win32-x64`
