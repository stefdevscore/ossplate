# ADR 0005: Verification Enforces Source And Installed Contracts

## ADR-0005-01 Status

Accepted

## ADR-0005-02 Context

`ossplate` ships through source workflows and through installed Cargo, npm, and PyPI artifacts. Source tests alone cannot prove that the wrappers remain thin adapters, that packaged scaffold content is correct, or that published artifacts still execute the expected CLI path.

## ADR-0005-03 Decision

- Verification is a first-class architecture slice.
- The verification path must cover source checks, artifact assertions, and installed-artifact smoke flows.
- CI on `dev` and `main` is part of the release confidence model, not just hygiene.

## ADR-0005-04 Consequences

- `scripts/verify.sh`, package assertions, and live E2E are architectural enforcement, not optional extras.
- Docs must keep the verification order and installed-E2E scope current.
- Wrapper regressions can be caught as delivery failures even when the Rust core still passes unit tests.
