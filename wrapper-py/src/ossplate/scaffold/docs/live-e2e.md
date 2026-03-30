# Live E2E

Use this flow to exercise the published `ossplate` packages the way a real user installs them.

This is separate from source-repo verification. It checks the live registry artifacts:

- Cargo: `cargo install ossplate`
- npm: `npm install ossplate`
- PyPI: `pip install ossplate`

## E2E-01 Default Flow

Run the full live install matrix:

```bash
./scripts/live-e2e.sh
```

That script runs three isolated install paths in temporary directories:

1. Cargo install into a temporary `--root`
2. npm install into a temporary project with local `node_modules/.bin/ossplate`
3. Python install into a temporary virtual environment

Each run also writes a timestamped capture log under `.live-e2e/` in the repo root. Override the location with `OSSPLATE_LIVE_E2E_CAPTURE_DIR=/path/to/logs` if you want the captures somewhere else.

For npm debugging against local artifacts instead of the public registry, override the package specs:

```bash
OSSPLATE_LIVE_E2E_NPM_RUNTIME_SPEC=/path/to/ossplate-linux-x64-<version>.tgz \
OSSPLATE_LIVE_E2E_NPM_PACKAGE_SPEC=/path/to/ossplate-<version>.tgz \
./scripts/live-e2e.sh npm
```

Each installed CLI must pass the same checks:

- `ossplate version`
- `ossplate create <tmp>`
- `ossplate validate --path <tmp> --json`
- `ossplate inspect --path <tmp> --json`
- `ossplate sync --path <tmp> --check --json`
- `ossplate init --path <tmp>`
- `ossplate validate --path <tmp> --json`
- `ossplate sync --path <tmp> --check --json`

## E2E-02 Single-Ecosystem Runs

Run only one installer path when debugging:

```bash
./scripts/live-e2e.sh cargo
./scripts/live-e2e.sh npm
./scripts/live-e2e.sh python
```

## E2E-03 What This Covers

- the published package name is installable
- the installed command is on the expected path
- the installed CLI can create from its shipped scaffold payload
- the installed CLI can validate and sync-check created and initialized projects

## E2E-04 What It Does Not Cover Yet

- containerized Linux matrix verification
- browser or multi-service integration scenarios

Local operator runs are still host-narrow, but published releases now add native-runner cross-platform live install coverage through `.github/workflows/live-e2e-published.yml` on:

- `ubuntu-latest`
- `macos-14`
- `macos-15-intel`
- `windows-latest`

Remaining gaps are containerized Linux variants, PowerShell-native Windows shell coverage, and any multi-service integration scenarios beyond CLI install/use.
