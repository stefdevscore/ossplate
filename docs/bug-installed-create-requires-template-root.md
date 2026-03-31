# Bug: Installed `ossplate create` Requires External Template Root

## Summary

Creating a new project from an installed `ossplate` binary is not self-contained.

After `cargo install ossplate`, running `ossplate create <target>` on a machine without a nearby `ossplate` source checkout fails unless `OSSPLATE_TEMPLATE_ROOT` is pointed at a scaffold source tree containing `ossplate.toml`.

That is product friction and breaks the expected installed-first onboarding path.

## Date

- Verified: 2026-03-30

## Reproduction

1. Install the latest published CLI:

```bash
cargo install ossplate
```

2. Attempt to create a new project from an arbitrary directory that is not inside an `ossplate` source checkout:

```bash
ossplate create /tmp/example-project \
  --name "Example Project" \
  --repository "https://github.com/acme/example-project" \
  --author-name "Acme" \
  --author-email "oss@acme.dev" \
  --rust-crate "example-project" \
  --npm-package "example-project" \
  --python-package "example-project" \
  --command "example-project"
```

3. Observe the failure:

```text
ossplate: failed to locate template root containing ossplate.toml
```

## Actual Behavior

- The installed binary attempts to discover a template root on the local filesystem.
- If no source checkout is nearby, `create` fails.
- The current workaround is to set `OSSPLATE_TEMPLATE_ROOT` to a real scaffold source checkout.

## Expected Behavior

- `cargo install ossplate` should be sufficient for first-time project creation.
- `ossplate create ...` should work from any directory without requiring:
  - a local source checkout
  - an environment-variable override
  - repo-internal knowledge about scaffold payload layout

## Impact

- First-time dogfooding is not frictionless.
- The CLI is not actually standalone for the most important greenfield flow.
- Installed-user docs currently imply a smoother path than the product delivers.

## Root Cause

`create` and `init` currently depend on template-root discovery logic that looks for `ossplate.toml`:

- near the current executable
- in the current directory ancestry
- or via `OSSPLATE_TEMPLATE_ROOT`

The published binary does not appear to embed the scaffold payload needed for standalone scaffolding.

## Temporary Workaround

Use the installed binary with an explicit template root:

```bash
OSSPLATE_TEMPLATE_ROOT=/path/to/ossplate-source-checkout \
  ossplate create /tmp/example-project \
  --name "Example Project" \
  --repository "https://github.com/acme/example-project" \
  --author-name "Acme" \
  --author-email "oss@acme.dev" \
  --rust-crate "example-project" \
  --npm-package "example-project" \
  --python-package "example-project" \
  --command "example-project"
```

## Desired Fix Direction

- Ship the scaffold payload with the installed binary, or otherwise make scaffold source resolution self-contained for installed usage.
- Preserve the existing source-checkout workflow for local development.
- Keep `OSSPLATE_TEMPLATE_ROOT` only as an override, not a requirement for normal use.

## Notes

- `ossplate init` on an empty directory is not a substitute for this flow. It behaves like adoption/hydration and does not recreate the full scaffold payload from nothing.
- This was observed while resetting `ossblade` from scratch to dogfood greenfield project creation with the published CLI.
