# Hexagonal Shell

This document defines the scalable architecture shell `ossplate` should grow into.

The goal is not to turn `ossplate` into a framework exercise. The goal is to create a structure that:

- keeps one product architecture centered in Rust
- keeps the Rust core as the behavioral source of truth
- preserves thin wrappers
- makes ownership boundaries explicit
- gives the codebase room to grow without collapsing into command sprawl
- stays enforceable through tests and lightweight checks

## Why A Shell

`ossplate` does not need a full backend-style DDD/CQRS stack.

It does need a shell that makes it obvious:

- where new behavior belongs
- what each slice is allowed to know
- where infrastructure concerns should stop
- how to add new features without mixing product logic, packaging logic, and release logic together

In this document, “hexagonal shell” means:

- clear architecture slices
- inward dependency direction
- ports only where external churn is real
- mechanical checks at boundaries

For `ossplate`, that shell exists once at the product level:

- Rust carries the real behavioral shell
- JavaScript and Python remain adapter packages with local structure
- wrappers are not peer product cores unless they later earn enough behavior to justify that move

## The Proposed Slices

The scalable shell for `ossplate` has five slices.

### 1. Core Execution

This slice owns product behavior.

It includes:

- command parsing
- command dispatch
- use-case orchestration for `validate`, `sync`, `create`, `init`, and `publish`
- domain rules around owned metadata and scaffold operations

It does not own:

- npm package installation behavior
- Python wheel packaging behavior
- GitHub workflow implementation details
- arbitrary wrapper-specific branching

If a new feature changes what `ossplate` means or does, it belongs here first.

### 2. Scaffold Synthesis

This slice owns the generated-project projection.

It includes:

- scaffold file selection
- scaffold-manifest ownership
- projection rules from source repo to distributed scaffold
- parity assertions between source-of-truth files and shipped scaffold files

It does not own:

- project-specific runtime behavior
- wrapper launch behavior
- release sequencing

The key idea is that the scaffold is a controlled projection, not a blind source copy.

### 3. Package Metadata Sync

This slice owns bounded identity-bearing surfaces.

It includes:

- `ossplate.toml` as the shared source of truth
- Cargo, npm, and Python metadata synchronization
- wrapper README identity synchronization
- bounded workflow display-name synchronization
- drift detection and sync-check behavior

It does not own:

- workflow job logic
- registry auth setup
- arbitrary prose outside bounded markers
- product logic outside identity-bearing surfaces

This slice should stay narrow and explicit. If a surface is not bounded, sync should not rewrite it.

### 4. Registry Release Orchestration

This slice owns release and publish behavior as product architecture, not “just CI glue”.

It includes:

- release-state assertions
- publish-readiness assertions
- local operator publish behavior
- registry sequencing rules
- rerun safety
- post-publish reconciliation such as npm lockfile repair

It does not own:

- core CLI semantics unrelated to publishing
- wrapper runtime resolution
- generic CI concerns with no release impact

This is the slice most likely to accumulate accidental complexity, so it should be treated as a first-class architectural boundary.

### 5. Verification

This slice owns the checks that keep the other slices honest.

It includes:

- source verification
- unit and integration tests
- package artifact assertions
- installed-artifact smoke checks
- live registry E2E
- future architecture checks for dependency and ownership rules

It does not own:

- product behavior
- release mutation logic
- wrapper-specific features

Verification exists to enforce architecture, not merely to provide generic QA coverage.

## How This Applies Per Package

The shell is shared at the product level, but it does not apply equally inside every package.

### Rust: Full Product Shell

Rust is the behavioral core and should carry the real internal architecture.

That means Rust owns:

- command behavior
- orchestration
- invariants
- owned metadata rules
- release and publish policy

If `ossplate` grows richer internals, that growth should happen here first.

### TypeScript: Adapter-Local Shell

The TypeScript package should stay adapter-shaped.

It may own:

- runtime package lookup
- packaged binary resolution
- environment setup
- delegation into the shipped Rust binary
- package-local tests for install and runtime handoff

It should not grow its own peer application/domain/infrastructure stack unless it starts carrying real product behavior.

### Python: Adapter-Local Shell

The Python package should follow the same rule.

It may own:

- packaged binary lookup
- target-specific wheel handoff
- environment setup
- delegation into the shipped Rust binary
- package-local tests for install and runtime handoff

It should not become a second product core.

## Dependency Direction

The shell should follow this dependency direction:

- wrappers and delivery edges -> core execution
- scaffold synthesis -> core execution rules where needed, but not wrapper behavior
- package metadata sync -> core execution and owned metadata rules
- release orchestration -> core execution plus external registry/process boundaries
- verification -> every slice, but only as observer/enforcer

More concretely:

- wrappers may depend on packaged runtime lookup and environment setup, then delegate inward
- core execution must not depend on wrapper-specific product behavior
- release orchestration may call external tools and registries, but should keep sequencing and recovery policy inside the core-owned surface
- verification may inspect all slices, but should not become a hidden implementation layer

The critical interpretation is:

- one shell for the product
- small adapter-local structure in JS/Python
- not three equal hexagonal architectures

## Where Ports And Adapters Actually Make Sense

`ossplate` should not introduce ports and adapters everywhere.

They are worth using where infrastructure churn or replaceability is real:

- process execution for local operator publish and external tool invocation
- registry existence checks and publish boundaries
- scaffold source vs shipped scaffold projection boundaries
- wrapper-to-core runtime handoff boundaries

Inside the JS and Python wrappers, keep these boundaries minimal and packaging-focused.

They are not worth using just to imitate backend architecture patterns:

- simple metadata transformations
- straightforward file rewriting with stable local rules
- command-local logic that has no alternate infrastructure implementation

The rule is simple:

- add a port when the boundary is external, volatile, or high-risk
- keep direct function boundaries when the behavior is stable and local

## Ownership Constraints

These constraints should stay explicit as the codebase grows.

### Core Execution May

- define command behavior
- define invariants
- define use-case flow
- define required inputs and outputs

### Core Execution May Not

- depend on wrapper package internals as behavior
- hide release policy inside CI-only shell scripts
- let generated scaffold artifacts become the real source of truth

### Wrappers May

- resolve the correct packaged binary
- set environment context
- forward execution

### Wrappers May Not

- implement separate product rules
- fork command semantics
- accumulate release policy

### Scaffold Synthesis May

- project curated files
- exclude maintainer-only files
- assert parity for shipped files

### Scaffold Synthesis May Not

- silently drift from the source repo
- ship internal-only harnesses by accident
- become a second implementation path

### Release Orchestration May

- encode sequencing
- encode rerun safety
- encode publish preflight and reconciliation

### Release Orchestration May Not

- spread release-critical policy across unrelated scripts and wrappers
- rely on undocumented ordering assumptions
- bypass product-owned assertions

## How This Scales As You Build

The shell is meant to scale in stages.

### Stage 1: Current Small Product

At the current size:

- keep Rust as the only behavioral core
- keep wrappers thin
- keep sync ownership narrow
- keep scaffold projection explicit
- keep release logic concentrated and tested

Do not add new layers unless they remove a concrete source of drift or confusion.

### Stage 2: Growth In Commands And Rules

When command behavior grows:

- split code by use case or ownership boundary, not by generic technical category
- keep related release behavior together
- keep scaffold logic together
- keep metadata sync logic together

Prefer vertical grouping by outcome over horizontal grouping by primitive.

### Stage 3: Growth In External Boundaries

When more registries, package surfaces, or delivery edges appear:

- introduce ports at those churn-heavy boundaries
- keep policy in the core-owned slice
- keep adapters at the edge

This is where the shell becomes more explicitly hexagonal, but only because the system earned it.

### Stage 4: Growth In Team Size And Change Volume

When multiple people are changing the system concurrently:

- convert architecture rules into checks where possible
- codify what each slice may own
- add narrow assertions before adding broad abstractions

The first scaling move should usually be stronger boundaries, not more framework.

## How To Add New Features Safely

When adding a feature, ask these questions in order:

1. Does this change product behavior?
   If yes, start in core execution.
2. Does this change what gets projected into shipped scaffolds?
   If yes, add it through scaffold synthesis and parity checks.
3. Does this change identity-bearing metadata?
   If yes, bound it explicitly before letting sync own it.
4. Does this change release sequencing or registry behavior?
   If yes, place it in release orchestration and add rerun/failure coverage.
5. Does this add a new external boundary?
   If yes, consider whether a port is justified.

This should keep growth intentional instead of ad hoc.

## Verification Map

The shell should be enforced through layered verification:

- smoke:
  basic command and sync behavior
- unit/integration:
  command parsing, local rules, wrapper parity
- packaging:
  tarball and wheel content, runtime isolation, scaffold content
- installed-artifact smoke:
  packed or locally installed wrappers invoking the shipped CLI
- live E2E:
  actual registry-installed cargo/npm/python flows
- architecture checks:
  future ownership and dependency assertions

The important rule is that each test layer should map to a real architectural risk.

## What To Avoid

Avoid these failure modes while building the shell:

- copying full backend CQRS or repository/service patterns into a CLI that has not earned them
- creating a giant shared internal utility layer with vague ownership
- letting wrappers become alternate product implementations
- treating release logic as incidental shell glue instead of a bounded architecture slice
- documenting architecture without enforcing it anywhere

## Near-Term Implementation Moves

The next concrete steps after this document should be:

1. Update the main architecture page to point to this shell as the forward-looking model.
2. Promote the proposed slices and constraints into canonical docs navigation.
3. Add lightweight architecture checks where the rules are already stable.
4. Keep new feature work aligned to one of the five slices.
5. Add ADRs only when a boundary becomes important enough that changing it would be costly.

## Bottom Line

The scalable version of `ossplate` is not “more layers”.

It is:

- one behavioral core
- thin wrappers
- explicit scaffold projection
- narrow sync ownership
- release orchestration treated as architecture
- verification that enforces the boundaries

That is the hexagonal shell `ossplate` should implement and grow into.
