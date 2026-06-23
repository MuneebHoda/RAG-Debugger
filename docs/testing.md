# Testing Guide

## Rust

Run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Expected coverage in the scaffold:

- API health/readiness smoke tests.
- Chunker behavior tests.
- Domain serialization tests as contracts become public.

## Web

Run:

```sh
cd apps/web
npm run typecheck
npm run lint
npm test
npm run build
```

Expected coverage in the scaffold:

- App shell render test.
- Playwright configuration for future workflow tests.

## Documentation Check

When changing commands, paths, or architecture, update:

- `README.md`
- `docs/development.md`
- Relevant ADRs in `docs/adr`

