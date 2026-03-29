# ADR 0001: Rust Core, Thin Wrappers

## ADR-0001-01 Status

Accepted

## ADR-0001-02 Context

`ossplate` has to ship one command surface across Cargo, npm, and PyPI. Maintaining separate implementations would create drift quickly and make release verification harder.

## ADR-0001-03 Decision

- Rust is the only product implementation.
- JavaScript and Python packages are adapters that resolve the packaged binary and forward arguments.
- Wrapper packages preserve stdout, stderr, and exit status from the Rust binary.

## ADR-0001-04 Consequences

- New product behavior goes into `core-rs`.
- JS and Python should stay small and operationally focused.
- Parity testing can compare wrapper execution with direct Rust execution.
