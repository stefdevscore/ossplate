# Package Size Report

This report summarizes package-size tradeoffs across Cargo, npm, and PyPI. It is reference material, not canonical product policy.

## SIZE-01 Current Size Model

- the top-level npm package is thin
- npm runtime binaries live in platform packages
- the published Windows npm runtime package is `ossplate-windows-x64`
- Python wheels bundle exactly one target binary each
- scaffold payload size is secondary to binary size

Internal target identifiers still use values such as `win32-x64`, but published package names use the public runtime package names.

## SIZE-02 What Matters

- avoid shipping nested runtime binaries in the scaffold
- keep the scaffold curated through `scaffold-payload.json`
- keep top-level npm and Python artifacts focused on one install path per target

For current release behavior, use [Releases](./releases.md). For current verification rules, use [Testing](./testing.md).
