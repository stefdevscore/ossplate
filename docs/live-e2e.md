# Live E2E

Use this flow to exercise the published `ossplate` packages the way a real user installs them.

This is separate from source-repo verification. It checks the live registry artifacts:

- Cargo: `cargo install ossplate`
- npm: `npm install ossplate`
- PyPI: `pip install ossplate`

## Default Flow

Run the full live install matrix:

```bash
./scripts/live-e2e.sh
```

That script runs three isolated install paths in temporary directories:

1. Cargo install into a temporary `--root`
2. npm install into a temporary project with local `node_modules/.bin/ossplate`
3. Python install into a temporary virtual environment

Each installed CLI must pass the same checks:

- `ossplate version`
- `ossplate create <tmp>`
- `ossplate validate --path <tmp> --json`
- `ossplate sync --path <tmp> --check`
- `ossplate init --path <tmp>`
- `ossplate validate --path <tmp> --json`
- `ossplate sync --path <tmp> --check`

## Single-Ecosystem Runs

Run only one installer path when debugging:

```bash
./scripts/live-e2e.sh cargo
./scripts/live-e2e.sh npm
./scripts/live-e2e.sh python
```

## What This Covers

- the published package name is installable
- the installed command is on the expected path
- the installed CLI can create from its shipped scaffold payload
- the installed CLI can validate and sync-check created and initialized projects

## What It Does Not Cover Yet

- cross-platform live install coverage
- containerized Linux matrix verification
- Windows PowerShell-native install flow
- browser or multi-service integration scenarios

Those are the next logical expansion points once the local repeatable flow is stable.
