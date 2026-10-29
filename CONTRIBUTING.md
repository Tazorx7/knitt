# Contributing to knitt

## Workflow
- All general features must go on their own branches (`feat/*`, `fix/*`).
- `main` and `master` are reserved for the blue-green flow.
- Submit pull requests to `main`. Branch protection requires reviews and passing CI.
- Run `cargo clippy --all-targets -- -D warnings` and `cd ui && npm run build` before pushing.
- Follow clean-room implementation - no copied assets, logos, or trademarks.

## Code Style
- Rust: follow clippy recommendations
- TypeScript/React: use existing patterns and types
