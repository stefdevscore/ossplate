# JavaScript Package Size Report

This report focuses only on the npm distribution strategy for `ossplate`: what ships today, why it is large, and the best reduction paths that still fit the current wrapper-based architecture.

## Current JS Distribution Shape

The npm package currently ships:

- the JavaScript launcher in `bin/ossplate.js`
- compiled JS in `dist/`
- packaged runtime binaries under `bin/<target>/...`
- a full scaffold payload under `scaffold/`

Current package behavior from `wrapper-js/src/index.ts`:

- resolve the current platform target at runtime
- execute a bundled `ossplate` binary from `bin/<target>/...`
- set `OSSPLATE_TEMPLATE_ROOT` to `scaffold/`

Current package configuration from `wrapper-js/package.json`:

- the package publishes `bin`, `dist`, `README.md`, and `scaffold`
- packaging is done through `prepack`
- there is only one npm package: `ossplate`

## Current Size Drivers

Local measurement on the current `dev` branch:

- `npm pack --dry-run`: `20.8 MB` unpacked, `6.0 MB` tarball, `49` files
- `wrapper-js/bin`: `6.6 MB`
- `wrapper-js/scaffold`: `13 MB`

The local target payload right now is mostly one real binary:

- `bin/darwin-arm64/ossplate`: about `6.6 MB`
- the other target entries are still stubs on this branch

The important point is that the current size is not caused by TypeScript or docs. It comes from two structural choices:

1. the package bundles native runtime binaries directly
2. the package also bundles a scaffold that itself contains nested wrapper/runtime structure

As more real platform binaries replace stubs, the npm package size will grow again unless the distribution shape changes.

## What npm Supports Well

npm already has a standard model for platform-targeted packages:

- `os`
- `cpu`
- `optionalDependencies`

That means the most natural reduction path is not “compress harder”; it is “publish fewer binaries per install.”

Relevant references:

- [package.json fields](https://docs.npmjs.com/cli/v11/configuring-npm/package-json)
- [npm install behavior](https://docs.npmjs.com/cli/v11/commands/npm-install)

## Best Reduction Options

### 1. Split runtime binaries into platform packages

This is the highest-value JS change.

Target shape:

- top-level `ossplate` stays the public npm package
- runtime binaries move into separate packages such as:
  - `@stefdevscore/ossplate-darwin-arm64`
  - `@stefdevscore/ossplate-darwin-x64`
  - `@stefdevscore/ossplate-linux-x64`
  - `@stefdevscore/ossplate-win32-x64`
- each platform package declares matching `os` and `cpu`
- top-level `ossplate` lists them as `optionalDependencies`
- the JS resolver loads the installed matching platform package instead of a local `bin/<target>` file inside the top-level tarball

Expected impact:

- users only download one platform binary through npm
- the top-level npm tarball becomes much smaller
- the architecture stays thin-wrapper-based

Tradeoff:

- release complexity increases because npm becomes a multi-package publish flow

### 2. Remove nested runtime binaries from the scaffold payload

This is the next best JS reduction even if platform packages are deferred.

Today the scaffold payload still includes wrapper trees that can contain runtime binary directories. That duplicates install-time concerns inside a create/init payload.

Preferred rule:

- scaffold includes source, manifests, docs, workflows, and lightweight entrypoints
- scaffold does not include pre-bundled runtime binaries
- generated projects stage their own wrapper binaries as part of their own release process

Expected impact:

- smaller npm package immediately
- less duplication between “installed tool” and “generated project”

Tradeoff:

- generated projects do not inherit pre-staged wrapper binaries from the npm package

### 3. Make the scaffold manifest stricter for JS packaging

Even after runtime binaries are handled, the scaffold should stay intentional.

Recommended tightening:

- keep only files needed to create or initialize a project
- exclude any file that exists only to support publishing already-built npm artifacts
- treat scaffold growth as a product decision, not a default copy-through

Expected impact:

- moderate size reduction
- lower long-term drift risk

Tradeoff:

- requires more discipline in the scaffold staging script and manifest

### 4. Add package-size assertions to CI

Once the distribution strategy is improved, lock it in.

Recommended checks:

- capture `npm pack --dry-run` unpacked size and package size
- assert no nested scaffold runtime binaries are present
- assert expected top-level runtime footprint for the current strategy

Expected impact:

- prevents regressions from quietly shipping large payload increases

Tradeoff:

- one more CI maintenance surface

## Recommended JS Sequence

If the goal is to reduce adoption friction with the highest impact first, the best JS-only sequence is:

1. remove nested runtime binaries from the scaffold payload
2. split top-level npm runtime binaries into platform packages
3. update the JS resolver to load the installed platform package
4. add CI assertions for tarball contents and size budget

That order gives a meaningful size reduction early while keeping the larger multi-package npm refactor contained.

## Concrete Next-Step Plan Shape

The next executable JS slice should do all of the following:

- introduce platform npm packages for runtime binaries
- keep `ossplate` as the only user-facing npm install command
- update `wrapper-js/src/index.ts` to resolve binaries from installed platform packages first
- remove bundled runtime binaries from the scaffold payload
- keep `create`, `init`, `validate`, and `sync` behavior unchanged
- update JS packaging tests to validate:
  - only one platform runtime is installed per environment
  - scaffold payload contains no nested runtime binaries
  - packaged `create` and `validate` still work end to end
