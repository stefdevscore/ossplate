# ADR 0002: Sync Owns Bounded Identity Surfaces

## ADR-0002-01 Status

Accepted

## ADR-0002-02 Context

`ossplate sync` must keep shared project identity aligned without becoming a destructive repo rewriter.

## ADR-0002-03 Decision

`sync` owns only bounded, identity-bearing surfaces:

- Rust, npm, and Python package metadata
- wrapper package README content
- the root README identity block between `ossplate:readme-identity` markers
- workflow display names between `ossplate:workflow-name` markers

`sync` does not own:

- workflow logic
- auth setup
- arbitrary prose outside markers
- docs that are not explicitly bounded

## ADR-0002-04 Consequences

- Drift checks stay surgical.
- Contributors can change most prose and workflow logic safely.
- Any ownership expansion must be explicit and bounded first.
