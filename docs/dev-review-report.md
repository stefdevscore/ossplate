# Dev Branch Review

Date: 2026-03-29

This report reviews the current `dev` branch after merging the latest stable release and publish-pipeline work from `main`.

## Overall Assessment

`dev` is in a strong state.

- the branch is clean and synced with `origin/dev`
- the automated release and publish flow has been stabilized and merged into `dev`
- the split npm runtime package model and the platform-specific Python wheel model are both present
- the current risks are mostly documentation drift and maintainability follow-up, not release-blocking implementation gaps

## What Looks Solid

### Release And Publish Flow

- release orchestration now separates CI validation, release-state checks, publish readiness, and downstream registry publish monitoring
- npm top-level publish correctly depends on runtime package availability
- JS lockfile handling now distinguishes between:
  - source-repo CI lockfile validity
  - release-time placeholder lockfile shape
  - post-publish lockfile refresh back onto `main`
- PyPI publishes platform-specific wheels instead of a universal multi-binary wheel

### Packaging Shape

- the top-level npm package is thin again
- runtime binaries are split into per-platform npm packages
- scaffold payloads no longer carry nested runtime binaries
- Python wheels bundle only one target binary each

### Product Shape

- Rust remains the real product surface
- JS and Python wrappers remain thin
- scaffold ownership and sync boundaries are explicit and documented

## Findings

### P1: Release docs still name the old Windows npm runtime package

[`docs/releases.md`](./releases.md) still tells operators to use `ossplate-win32-x64` in the “Use the runtime package that matches the machine” section.

That is stale. The published npm runtime package name is now `ossplate-windows-x64`, while `win32-x64` remains only the internal target identifier.

Impact:

- operator confusion during local packaging checks
- easy copy/paste failure during Windows npm verification

### P1: Testing guide command order is stale after the release hardening work

[`docs/testing.md`](./testing.md) still lists an outdated “Underlying command order”.

The current root verification path includes:

- `scripts/assert-js-lockfile-state.mjs`
- `scripts/assert-publish-readiness.mjs publish`

but that list does not show both steps in the actual order.

Impact:

- docs understate the current gate
- contributors reading the testing guide do not see the real JS lockfile contract checks

### P2: Package-size reports still contain stale Windows package naming

The size-analysis docs still reference `ossplate-win32-x64` in places:

- [`docs/package-size-report.md`](./package-size-report.md)
- [`docs/javascript-package-size-report.md`](./javascript-package-size-report.md)

Those reports should now distinguish clearly between:

- runtime target: `win32-x64`
- published npm package: `ossplate-windows-x64`

Impact:

- minor docs inconsistency
- lowers confidence in the historical package-size writeups

## Recommended Next Steps

### Immediate Docs Cleanup

1. fix the Windows runtime package name in `docs/releases.md`
2. update `docs/testing.md` so the documented root verification order matches the current scripts
3. refresh the package-size reports so they use the current runtime package naming model

### Product/Docs Improvements Worth Doing Next

1. tighten the public docs set further so the reports and architecture docs are easier to navigate
2. review whether `wrapper-js/platform-packages/ossplate-win32-x64/README.md` should stay target-named internally or be clarified to reflect the published package identity
3. consider adding one short “current status” page that links product docs, release docs, and review reports together

## Conclusion

No new release-blocking implementation issues were discovered in this review.

The branch’s main remaining debt is documentation alignment after the recent release/publish hardening work. The next pass should focus on cleaning that up and then moving into feature work from a stable baseline.
