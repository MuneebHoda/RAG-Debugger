# ADR 0001: Rust Workspace Monorepo

## Status

Accepted.

## Context

RAG Debugger needs shared contracts, a local collector, backend services, future workers, and high-performance systems components. Splitting repositories now would slow iteration and make contract drift likely.

## Decision

Use a Rust workspace monorepo with apps and crates in one repository.

## Consequences

- Shared domain types live in `crates/core`.
- Product apps live in `apps`.
- Service and worker boundaries can be extracted later when scale requires it.

