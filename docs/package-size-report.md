# Package Size Report

This note covers the current size pressure in `ossplate`, why it exists, and the most realistic ways to reduce it without changing the product shape.

## Current State

Local measurement on the current `dev` branch:

- `wrapper-js` `npm pack --dry-run`: `20.8 MB` unpacked, `6.0 MB` tarball, `49` files
- `wrapper-js/bin`: `6.6 MB`
- `wrapper-js/scaffold`: `13 MB`
- `wrapper-py/src/ossplate/bin`: `6.6 MB`
- `wrapper-py/src/ossplate/scaffold`: `13 MB`

Current local binary payload:

- `darwin-arm64`: real binary at about `6.6 MB`
- `darwin-x64`: stub placeholder
- `linux-x64`: stub placeholder
- `win32-x64`: stub placeholder

The key point is that package size is not being driven by docs or TypeScript/Python wrapper code. It is being driven by:

- bundled native binaries
- scaffold payload duplication
- nested copies of wrapper/runtime assets inside the scaffold

Published and `main` branch numbers can be higher than this local `dev` snapshot when more real target binaries are staged.

## What The Ecosystem Supports

For npm, the supported primitives for platform-specific installs are package-level `os`/`cpu` constraints and dependency indirection. That means the clean size-reduction path is to move per-platform binaries into separate install-time packages and let the top-level package resolve the right one instead of shipping all targets in one tarball.

For Python, the standard model is platform-specific wheels for platform-dependent artifacts. `ossplate` is already moving in the correct direction there: one wheel per target plus an sdist is the normal packaging answer when a distribution bundles a native executable.

Relevant references:

- npm package constraints: [package.json `os` / `cpu`](https://docs.npmjs.com/cli/v11/configuring-npm/package-json)
- npm install behavior and dependency packaging: [npm install docs](https://docs.npmjs.com/cli/v11/commands/npm-install)
- Python binary packaging guidance: [Packaging binary extensions](https://packaging.python.org/en/latest/guides/packaging-binary-extensions/)
- Python distribution formats: [Package Formats](https://packaging.python.org/en/latest/discussions/package-formats/)

## Best Reduction Options

### 1. Split npm binaries into platform packages

This is the highest-value reduction.

Target shape:

- top-level `ossplate` npm package becomes a thin JS wrapper plus scaffold metadata
- each real binary moves to a platform package such as:
  - `ossplate-darwin-arm64`
  - `ossplate-darwin-x64`
  - `ossplate-linux-x64`
  - `ossplate-windows-x64`
- platform packages declare matching `os` / `cpu`
- top-level `ossplate` depends on them as `optionalDependencies`
- runtime resolution prefers the installed matching platform package

Expected impact:

- users download one target binary instead of all targets
- the top-level npm tarball shrinks materially
- the pattern matches how other native npm CLIs typically avoid multi-platform bloat

Tradeoff:

- release flow becomes a multi-package npm publish instead of a single package publish

### 2. Remove nested runtime binaries from the scaffold payload completely

This is the next biggest reduction after platform packages.

Today the scaffold still carries wrapper package trees that can themselves contain runtime-facing binary locations. Even when those are placeholders or small on the local branch, they will grow again as more targets become real.

Preferred rule:

- scaffold payload should contain source, manifests, docs, workflow templates, and lightweight launchers
- scaffold payload should not contain bundled runtime binaries
- created projects can restage or rebuild their own wrapper binaries as part of their own release process

Expected impact:

- smaller npm and Python artifacts
- less duplication between “installed product” and “generated project”

Tradeoff:

- `create` / `init` stays intact, but generated projects would no longer inherit pre-bundled wrapper binaries directly from the published artifact

### 3. Keep Python on platform wheels and avoid backsliding

Python is already on the right architectural path.

The main requirement here is to keep wheel contents disciplined:

- exactly one binary per wheel
- no nested wrapper binaries in scaffold payload
- one sdist only for source distribution

Expected impact:

- prevents the Python package from regressing back toward a universal multi-platform wheel

Tradeoff:

- release remains a wheel matrix, which is operationally more complex than one generic wheel

### 4. Make scaffold payload curation stricter

The scaffold is intentionally useful, but it is still large enough that every included file should be justified.

Preferred tightening:

- keep a strict manifest of what belongs in the scaffold
- exclude anything that exists only to support publishing installed binaries
- treat scaffold size as a tracked budget, not an accidental byproduct

Expected impact:

- moderate size reduction
- lower long-term drift risk

Tradeoff:

- more maintenance discipline around the scaffold manifest

## Recommended Next Sequence

If the goal is to reduce adoption friction without destabilizing the product, the best sequence is:

1. split npm runtime binaries into platform packages
2. remove nested runtime binaries from the scaffold payload
3. keep Python on target-specific wheels and add size assertions so it does not regress
4. add artifact size reporting to CI so future increases are obvious

## Suggested Acceptance Targets

Reasonable short-term targets:

- top-level npm package ships only one real binary per install path, not all targets
- scaffold payload contains no bundled runtime binaries
- Python wheels contain exactly one binary and stay platform-specific
- CI surfaces unpacked/tarball size for npm and wheel sizes for Python

That will not make `ossplate` tiny, because the product really does bundle native executables, but it should remove the worst avoidable duplication and make the size story easier to justify.
