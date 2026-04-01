# Agents

`ossplate` works well when an AI agent is the primary operator because the repo contract is explicit, bounded, and machine-checkable.

## AGENT-01 Best Agent Loop

Use this sequence:

1. `ossplate create <target>` to bootstrap a new pattern-1 repo.
2. `ossplate validate --json` to inspect consistency without guessing.
3. `ossplate inspect --json` to read the effective config, owned files, and runtime contract.
4. `ossplate upgrade --plan --json` when the repo may be on an older supported scaffold version and a chained upgrade path may exist.
5. `ossplate sync --check --json` or `ossplate sync --plan --json` to inspect bounded drift.
6. `ossplate sync --json` only when a bounded repair is needed.
7. `ossplate verify --json` to close the full repo gate in machine-readable form.

This keeps the agent inside the supported ownership boundary instead of rewriting random files heuristically.

## AGENT-02 Pattern-1 Fit

`ossplate` is strongest for repos that want:

- one real Rust core
- thin JS and Python delivery wrappers
- one CLI name and version across Cargo, npm, and PyPI
- strong release and verification discipline

That makes it a good bootstrap for agent-operated infrastructure CLIs, code tooling, and local developer utilities.

## AGENT-03 High-Signal Commands

These commands are the most useful for agents:

- `ossplate validate --json`
- `ossplate upgrade --plan --json`
- `ossplate upgrade --json`
- `ossplate sync --check --json`
- `ossplate sync --plan --json`
- `ossplate sync --json`
- `ossplate verify --json`
- `./scripts/verify.sh`
- `node ./scripts/release-check.mjs release-state`
- `node ./scripts/release-check.mjs publish-readiness publish`

Use `validate --json` for structured issue detection. Use `upgrade --plan --json` before attempting structural scaffold migration, and rely on its `upgradePath` or `blockingReason` fields instead of guessing historical compatibility. Use `sync --check --json` or `sync --plan --json` before bounded metadata mutation. Use `sync --json` when the bounded repair is intended. Use `verify --json` for a structured full-gate result, or `verify.sh` when a human shell-oriented gate is sufficient.

## AGENT-04 Safe Mutation Model

Prefer this mutation order:

1. update canonical identity in `ossplate.toml`
2. run `ossplate inspect --json`
3. if compatibility indicates an older supported descendant, run `ossplate upgrade --plan --json`
4. run `ossplate sync --plan --json`
5. run `ossplate sync --json`
6. run `ossplate validate --json`
7. run `ossplate verify --json`

Do not treat generated wrapper scaffold payloads as editable source. Edit the root source checkout and regenerate packaging assets through the existing scripts.

## AGENT-05 What Not To Assume

Agents should not assume:

- JS or Python own product behavior
- generated wrapper scaffolds are source of truth
- release state can be inferred from one manifest alone
- package metadata can be hand-edited safely without revalidation
- `init` is a broad migration tool across scaffold generations
- unversioned descendants are safe to upgrade without an exact fingerprint match

The supported contract is already narrower and more reliable than that.
