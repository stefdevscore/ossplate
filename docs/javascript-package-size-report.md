# JavaScript Package Size Report

This report tracks npm package-size tradeoffs. It is reference material, not the canonical release contract.

## JS-SIZE-01 Current Shape

- `ossplate` is the thin top-level npm package
- runtime binaries live in platform packages
- published runtime package names are:
  - `ossplate-darwin-arm64`
  - `ossplate-darwin-x64`
- `ossplate-linux-x64`
- `ossplate-windows-x64`

## JS-SIZE-02 What The Report Is For

Use this document when evaluating:

- top-level npm package bloat
- runtime package footprint
- whether scaffold payload changes are leaking install-time concerns into shipped npm artifacts

For the active release contract, use [Releases](./releases.md). For required artifact checks, use [Testing](./testing.md).
