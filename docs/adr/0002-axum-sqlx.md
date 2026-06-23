# ADR 0002: Axum and SQLx for the API

## Status

Accepted.

## Context

The API needs typed async HTTP handlers, structured errors, Postgres readiness, and a Rust ecosystem that can grow into local collectors and workers.

## Decision

Use Axum for HTTP and SQLx for Postgres access.

## Consequences

- API routes are explicit and easy to test with Tower.
- SQLx keeps database access typed without hiding SQL.
- Database migrations will be added before the first persistent feature.

