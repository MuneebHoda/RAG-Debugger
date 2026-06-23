# Development Guide

## Prerequisites

- Rust via `rustup`
- Node.js 24 or newer
- Postgres for persistent features
- `just` is optional but recommended

## Setup

```sh
cp .env.example .env
cd apps/web && npm install
```

## Run

API:

```sh
cargo run -p rag-debugger-api
```

Web:

```sh
cd apps/web && npm run dev
```

## Commands

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/web && npm run typecheck && npm run lint && npm test && npm run build
```

With `just`:

```sh
just check
just api
just web
```

## Database Flow

Persistent features should add SQLx migrations before repository implementations:

```sh
sqlx migrate add <change_name>
sqlx migrate run
```

The scaffold does not require a live database yet. `/readyz` is wired so database readiness can be added without changing callers.

## Adding a Feature

1. Add or update domain types in `crates/core`.
2. Add behavior interfaces in `crates/rag` or repository traits in `crates/storage`.
3. Implement API handlers under `apps/api`.
4. Add UI routes/components under `apps/web`.
5. Add tests at the lowest useful layer.
6. Update docs or ADRs when the architecture changes.

