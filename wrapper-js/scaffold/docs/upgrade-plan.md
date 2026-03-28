# Ossplate Upgrade Plan

## Goal

Turn `ossplate` from a minimal multi-registry demo into a production-ready scaffold tool with:

- one canonical core program
- real wrapper parity across Rust, TypeScript/JavaScript, and Python
- robust CI and publish workflows with explicit auth fallbacks
- layered testing guidance and enforcement
- a structure that can scale toward a hexagonal architecture shell

The product behavior is now real in the core maintenance and scaffold paths. Remaining work is about hardening, ergonomics, and release scale.

## Operating Principles

- Keep one source of truth for CLI behavior and output contracts.
- Prefer thin wrappers over duplicated implementations.
- Fail early when template placeholders were not replaced.
- Treat OIDC as the default publish path, with explicit token fallbacks.
- Require install/build/test/package checks before any publish step.
- Keep the starter small, but not fake in the critical paths.

## Phase 0: Stabilize the Template Baseline

Purpose: make the current scaffold safe to evolve and hard to misuse.

### P0

- Add a release-readiness validator that fails on placeholder metadata.
- Validate package names, crate name, binary names, repository URLs, author fields, and obvious placeholder strings.
- Add a root documentation index under `docs/` so implementation docs have a stable home.

### P1

- Normalize naming conventions across Cargo, npm, and PyPI packages.
- Define the canonical command name and the expected wrapper naming pattern.
- Document the template customization surface:
  project name, package ids, repo URL, author, license, binary name, description.

### P2

- Remove or tighten weak placeholder copy in README files.
- Add a checklist for creating a new project from the template.

### Exit Criteria

- A new maintainer can identify every required rename/customization step.
- CI can fail automatically if placeholder identity leaks into a release path.

## Phase 1: Establish the Canonical Core and Wrapper Parity

Purpose: stop treating each ecosystem as a separate product stub.

### P0

- Make the Rust binary the canonical executable implementation.
- Define a small, stable placeholder command contract:
  `--help`, `version`, `health`, and one example command returning structured JSON.
- Convert the JavaScript package into a real wrapper around the packaged binary.
- Convert the Python package into a real wrapper around the packaged binary.
- Ensure wrappers preserve exit codes, stdout, and stderr from the core binary.

### P1

- Move `wrapper-js` source to TypeScript and publish built JavaScript artifacts.
- Add binary resolution logic for supported platforms in both wrappers.
- Add wrapper tests that assert parity against the core binary output contract.

### P2

- Add a small shared contract document covering commands, arguments, output, and error behavior.
- Add support for environment-based binary overrides for local development and debugging.

### Exit Criteria

- Rust, JS, and Python packages all exercise the same underlying behavior.
- Wrapper packages no longer maintain separate hand-written CLI logic.

## Phase 2: Production-Grade Quality Gates

Purpose: make the scaffold trustworthy before publish automation runs.

### P0

- Expand CI to enforce core quality gates in all three environments.
- Add Rust checks:
  `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`.
- Add JS/TS checks:
  install, typecheck, test, package dry-run.
- Add Python checks:
  environment setup, tests, wheel/sdist build, install-from-built-artifact smoke test.
- Add a cross-package parity job that verifies wrappers match core contract behavior.

### P1

- Add artifact-level smoke tests for packaged npm and wheel outputs.
- Add a matrix strategy where it materially improves confidence for supported platforms.
- Make CI logs and job names intentionally clear for template adopters.

### P2

- Add optional local aggregate commands or scripts for running all checks consistently.
- Add a contributor quickstart for running the same quality gates outside CI.

### Exit Criteria

- A release candidate must pass formatting, linting, unit tests, packaging, and parity checks.
- Published artifacts are verified before publish jobs run.

## Phase 3: Robust Publishing with OIDC First and Concrete Fallbacks

Purpose: keep publishing reliable even when registry auth or registry behavior is imperfect.

### P0

- Keep publish workflows rerun-safe with published-version detection before attempting release.
- Use OIDC or trusted publishing as the default path for npm, PyPI, and crates.io where supported.
- Add explicit secret-based fallbacks:
  `NPM_TOKEN`, PyPI API token, and `CARGO_REGISTRY_TOKEN`.
- Make auth mode selection visible in workflow logs.
- Preserve non-destructive handling of already-published versions.

### P1

- Document exact registry setup steps for both preferred and fallback auth modes.
- Add validation that publish jobs fail clearly when neither OIDC nor token auth is configured.
- Tighten registry-specific behavior:
  npm public access, PyPI trusted publishing expectations, crates.io version detection and retry behavior.

### P2

- Add optional manual dispatch inputs for dry-run or targeted registry publish flows if they simplify maintenance.
- Add guidance for organizations that intentionally disable OIDC and standardize on tokens.

### Exit Criteria

- Publish workflows are understandable, rerunnable, and resilient.
- Auth failures are explicit and actionable rather than implicit or flaky.

## Phase 4: Layered Testing Guidance and Scaffolding

Purpose: teach adopters how to scale verification without overcomplicating the starter.

### P0

- Add docs for the default testing pyramid:
  smoke, unit, integration/parity, and release verification.
- Define minimum required tests for any project generated from this template.
- Document how wrapper parity tests fit into the scaffold.

### P1

- Add optional docs for live end-to-end browser testing with Playwright when a generated project includes a web UI.
- Clarify that Playwright is not a default requirement for non-UI projects.
- Add example CI placement for smoke vs slower e2e suites.

### P2

- Add template examples of failure triage:
  unit regression, packaging regression, publish regression, wrapper parity regression.

### Exit Criteria

- The testing strategy is clear enough that teams do not improvise incompatible structures from scratch.
- Optional e2e guidance exists without forcing UI assumptions onto every project.

## Phase 5: Architecture Shell and Scaling Docs

Purpose: leave room for serious product evolution without bloating the first scaffold cut.

### P1

- Add architecture docs for a hexagonal baseline:
  domain, application, adapters, and delivery surfaces.
- Define where product logic belongs and where it should not live.
- Document wrappers as adapters rather than alternate implementations.

### P2

- Add an example directory shell that future projects can expand into as complexity grows.
- Link to the stronger internal reference material that informed this structure.
- Add ADR guidance if the template grows beyond a small starter.

### Exit Criteria

- The template can scale into a maintainable project layout without major restructuring.
- The initial starter remains small and understandable.

## Recommended Execution Order

1. Phase 0
2. Phase 1
3. Phase 2
4. Phase 3
5. Phase 4
6. Phase 5

Rationale:

- Phase 0 prevents accidental misuse while the template is still changing.
- Phase 1 fixes the most important structural flaw: duplicated CLI stubs.
- Phase 2 makes the scaffold trustworthy.
- Phase 3 hardens release automation after the package surfaces are stable.
- Phase 4 and Phase 5 improve scale and adoption quality without blocking the core scaffold.

## Suggested Near-Term Milestones

### Milestone A

Complete Phases 0 and 1.

Outcome:
`ossplate` becomes a real scaffold with one canonical core and thin wrappers.

### Milestone B

Complete Phase 2 and the P0 items in Phase 3.

Outcome:
the scaffold becomes release-grade with meaningful quality gates and robust publish behavior.

### Milestone C

Complete Phases 4 and 5.

Outcome:
the scaffold becomes easier to adopt, extend, and scale across future projects.

## Current State

Completed:

- release-readiness checks are wired into CI
- JS and Python are thin wrappers over the Rust core
- the Rust core now provides `version`, `validate`, `sync`, `create`, and `init`
- `ossplate.toml` is the canonical identity source for owned metadata surfaces
- installed npm and Python artifacts now carry the staged scaffold payload for `create` and `init`
- artifact smoke tests prove installed distributions can run `version`, `create`, and `validate`
- scaffold payload staging is now driven by an explicit curated manifest instead of a broad repo snapshot
- Python packaging stages distribution assets through its build hook
- CI now enforces Rust formatting and clippy alongside the existing test/package checks

## Next Steps

- expand `sync` ownership carefully beyond the root README identity block and into selected workflow identity fields where safe
- improve `validate` and `sync --check` output further if diff-style rendering becomes necessary
- keep the root verification workflow aligned with CI as new checks are added
- harden publish workflows with the OIDC-first fallback strategy from Phase 3
