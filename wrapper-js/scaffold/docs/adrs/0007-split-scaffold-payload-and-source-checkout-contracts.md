# ADR 0007: Split Scaffold Payload And Source-Checkout Contracts

## ADR-0007-01 Status

Accepted

## ADR-0007-02 Context

One mixed scaffold manifest had started serving too many consumers at once: packaged scaffold contents, full source-checkout validation, and exclusion policy. Small structural refactors therefore forced unrelated packaging, validation, and test consumers to move in lockstep.

## ADR-0007-03 Decision

- `scaffold-payload.json` owns packaged scaffold contents and exclusions.
- `source-checkout.json` owns the full source-checkout contract used by create/init/publish validation.
- Consumers should read only the contract they actually need.

## ADR-0007-04 Consequences

- File moves in the source-checkout contract no longer force unrelated payload consumers to share one schema.
- Packaging and source-validation drift fail in narrower places.
- New scaffold-boundary changes must update the correct contract file explicitly.
