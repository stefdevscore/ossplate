# ADR 0015: Agent-First Machine-Readable Repo Contract

## ADR-0015-01 Status

Accepted

## ADR-0015-02 Context

`ossplate` had already made validation and bounded metadata repair deterministic, but agent operators still had to infer too much of the repo contract from raw files and human-oriented command output. That was workable for humans, but it forced agents to duplicate repo logic when inspecting ownership, planning sync, reading bootstrap results, or preflighting publish behavior.

## ADR-0015-03 Decision

- The primary operator contract for repo state inspection and bounded repair planning is machine-readable JSON.
- `validate --json`, `sync --check --json`, `sync --plan --json`, `sync --json`, `inspect --json`, `create --json`, `init --json`, and `publish --plan --json` are first-class supported agent surfaces.
- These commands must describe the effective repo contract directly rather than forcing operators to infer it from manifests, workflow files, or helper scripts.
- Generated repos get a generated-repo-safe agent operations guide, while repo-local `ossplate` operator guidance stays separate.

## ADR-0015-04 Consequences

- Agents can inspect, plan, and apply bounded repo changes without parsing human shell output.
- Repo-local `ossplate` positioning and generated-project operations guidance stay intentionally separate.
- Documentation and tests must keep the JSON command loop current as the repo contract evolves.
