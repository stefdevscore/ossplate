# Agents

`ossplate` works well when an AI agent is the primary operator because the repo contract is explicit, bounded, and machine-checkable.

## AGENT-01 Best Agent Loop

Use this sequence:

1. `ossplate create <target>` to bootstrap a new pattern-1 repo.
2. `ossplate validate --json` to inspect consistency without guessing.
3. `ossplate sync --check` to confirm owned metadata is aligned.
4. `ossplate sync` only when a bounded repair is needed.

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
- `ossplate sync --check --json`
- `ossplate sync --plan --json`
- `ossplate sync --json`
- `./scripts/verify.sh`
- `node ./scripts/release-check.mjs release-state`
- `node ./scripts/release-check.mjs publish-readiness publish`

Use `validate --json` for structured issue detection. Use `sync --check --json` or `sync --plan --json` before mutation. Use `sync --json` when the bounded repair is intended. Use `verify.sh` when the agent needs the full repo gate.

## AGENT-04 Safe Mutation Model

Prefer this mutation order:

1. update canonical identity in `ossplate.toml`
2. run `ossplate sync`
3. run `ossplate validate --json`
4. run `ossplate sync --check`

Do not treat generated scaffold mirrors as editable source. Edit the root source checkout and regenerate packaging assets through the existing scripts.

## AGENT-05 What Not To Assume

Agents should not assume:

- JS or Python own product behavior
- scaffold mirrors are source of truth
- release state can be inferred from one manifest alone
- package metadata can be hand-edited safely without revalidation

The supported contract is already narrower and more reliable than that.
