# ADR 0008: Stage Neutral Runtime Artifacts For Wrapper Packaging

## ADR-0008-01 Status

Accepted

## ADR-0008-02 Context

Python wheel packaging previously depended on the JavaScript runtime package tree to locate the built native executable. That created an unnecessary wrapper-to-wrapper dependency and made packaging layout changes higher risk than they needed to be.

## ADR-0008-03 Decision

- Runtime binaries are staged first into a neutral generated artifact location.
- JavaScript runtime packages and Python wheel builds both consume that neutral staged artifact.
- Wrapper packaging must not depend on another wrapper's package tree layout.

## ADR-0008-04 Consequences

- Cross-wrapper coupling is lower.
- The staging path remains shared, but the consumers are siblings rather than parent/child.
- Future packaging changes can happen with less wrapper-specific coordination.
