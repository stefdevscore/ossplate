# Documentation

This docs area explains how to use `ossplate` as a real tool for validating, synchronizing, and now creating a multi-registry scaffold.

## Start Here

- [Customizing The Template](./customizing-the-template.md)
- [Architecture](./architecture.md)
- [Testing Guide](./testing.md)
- [Release Guide](./releases.md)
- [Phase 1 Contract](./phase-1-contract.md)
- [Upgrade Plan](./upgrade-plan.md)
- `ossplate validate` checks owned metadata drift
- `ossplate sync --check` verifies the repo is already synchronized
- `ossplate create <target>` scaffolds a clean target directory and can apply identity overrides from flags
- `ossplate init --path <dir>` hydrates an existing directory in place and can apply identity overrides from flags

## What These Docs Cover

- the canonical config and command surface
- the required rename and customization surface before first release
- the layered testing and packaging workflow
- the release operator flow and rerun-safe publish expectations
- the phased plan for turning this tool into a broader scaffold product
