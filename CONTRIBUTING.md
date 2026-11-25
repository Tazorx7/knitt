# Contributing to knitt

## Workflow
- Features go on a `feat/*` branch and fixes on a `fix/*` branch.
- `main` and `master` are reserved for the blue-green flow.
- Submit pull requests to `main`. Branch protection requires reviews and passing CI.
- Run `cargo clippy --all-targets -- -D warnings` and `cd ui && npm run build` before pushing.
- Implement from scratch. Do not copy assets, logos or trademarks.

## Code style
- Rust: follow clippy recommendations
- TypeScript/React: use existing patterns and types
