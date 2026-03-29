# Python Package Size Report

This report tracks Python wheel size and target packaging tradeoffs. It is reference material, not canonical operator guidance.

## PY-SIZE-01 Current Shape

- each wheel bundles one native executable for its target
- wheel filenames are target-specific
- wheel size is expected to be driven mainly by that one executable
- scaffold payload size should stay secondary and curated

## PY-SIZE-02 What To Watch

- accidental multi-target binary inclusion
- scaffold growth that adds maintainer-only or generated content
- size regressions that break the current target-specific budget assumptions

For active wheel and release behavior, use [Releases](./releases.md). For enforced artifact checks, use [Testing](./testing.md).
