# Python Package Size Report

This report focuses on the PyPI side of `ossplate`: what currently drives wheel size, what is already correct about the Python packaging model, and which reductions are still worth pursuing.

## Current Python Distribution Shape

`ossplate` on PyPI currently ships:

- one platform-specific wheel per supported target
- one source distribution
- one native `ossplate` executable inside each wheel
- the curated scaffold payload used by `create` and `init`

That is the correct high-level model for Python because the package bundles a native executable. A single `py3-none-any` wheel would be the wrong shape here.

## Current Size Drivers

Local measurement on the current `dev` branch:

- local host wheel: `ossplate-0.1.11-py3-none-macosx_15_0_arm64.whl`
- compressed wheel size: about `2.0 MB`
- local unpacked runtime payload for `darwin-arm64`: about `6.6 MB`
- local scaffold payload under `wrapper-py/src/ossplate/scaffold`: about `146 KB`

The important point is that Python size is no longer being driven by duplicated multi-platform binaries or by the scaffold payload. The wheel size is now dominated by the single bundled executable for the current target.

## What Is Already Good

The Python packaging story is already on the right structural path:

- wheels are target-specific
- each wheel carries exactly one binary
- the scaffold payload is small relative to the binary
- nested wrapper runtime binaries are excluded from the scaffold

That means Python does not need the kind of package split that JavaScript needed.

## Best Remaining Improvements

### 1. Add explicit size assertions to tests and CI

This is the best immediate parity improvement.

The wheel build tests should fail if a wheel unexpectedly grows past a reasonable target-specific budget. That keeps size from regressing silently as binaries or scaffold payloads change.

Recommended policy:

- assert compressed wheel size stays within a target-specific ceiling
- assert unpacked wheel contents stay within a target-specific ceiling
- keep the ceilings loose enough to avoid churn from normal binary variation

### 2. Keep scaffold growth disciplined

The scaffold is already small relative to the runtime binary, but it should stay curated.

That means:

- keep using `scaffold-manifest.json` as the source of truth
- avoid adding generated artifacts or runtime payloads back into the scaffold
- treat scaffold growth as a deliberate product decision

### 3. Make size visible in release confidence checks

CI does not need a separate reporting system to start being useful here. The existing wheel smoke test can report measured sizes in assertion failures and logs so regressions are easy to spot.

### 4. Revisit only if binary size becomes the real product problem

If the native executable itself grows materially, there are only a few real options:

- optimize the Rust binary size
- change what features compile into release artifacts
- redesign distribution so the Python package downloads or discovers the core separately

That would be a much larger product decision. It is not justified by the current package shape.

## Recommended Python Sequence

The surgical Python sequence is:

1. add wheel size assertions to the existing artifact test
2. document the current size model and expected budgets
3. keep watching published wheel sizes per target after releases

That gives `ossplate` parity with the JavaScript size work without destabilizing the Python install story.
