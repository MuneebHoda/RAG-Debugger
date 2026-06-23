# ADR 0004: React TypeScript Web UI

## Status

Accepted.

## Context

The first product surface is a debugger workbench for traces, sources, evals, and failure diagnosis. It needs fast iteration and strong type checks.

## Decision

Use React, Vite, and TypeScript with strict compiler settings.

## Consequences

- UI development stays quick while the Rust core matures.
- API boundaries should be generated or shared once contracts stabilize.
- Browser tests can verify core workflows before release.

