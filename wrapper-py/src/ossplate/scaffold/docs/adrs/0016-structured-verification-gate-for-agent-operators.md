# ADR 0016: Structured Verification Gate For Agent Operators

## ADR-0016-01 Status

Accepted

## ADR-0016-02 Context

Even after `ossplate` gained machine-readable validation, sync, inspect, bootstrap, and publish-plan surfaces, the final verification gate still lived only in `scripts/verify.sh`. That left the most important enforcement boundary, the full repo gate, exposed only through shell output. Agent operators could not rely on a supported machine-readable summary of which phase failed, which phases were skipped, or what stdout and stderr belonged to each verification step.

## ADR-0016-03 Decision

- `verify --json` is a first-class CLI surface for the full repo verification gate.
- It returns structured per-step results including step name, success state, exit code, stdout, stderr, skip state, and optional skip reason.
- The verification gate remains portable across supported environments and cannot depend on macOS-only shell assumptions.
- `scripts/verify.sh` remains the shell-oriented operator and CI entrypoint, but the underlying verification contract is also available as structured JSON.

## ADR-0016-04 Consequences

- Agent operators can close the full repo loop without parsing shell output heuristically.
- Documentation must treat `verify --json` and `verify.sh` as equivalent verification surfaces with different consumers.
- Verification step ordering, skip behavior, and cross-platform execution become part of the supported operator contract, not just script implementation detail.
