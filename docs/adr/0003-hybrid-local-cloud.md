# ADR 0003: Hybrid Local/Cloud Product Model

## Status

Accepted.

## Context

Startup teams need collaboration, but RAG traces and documents may contain sensitive data. Raw document upload by default would weaken trust.

## Decision

Use a local-first collector with cloud collaboration. Raw documents remain local unless a project explicitly enables approved snippet sync.

## Consequences

- Privacy mode is part of core domain modeling.
- Cloud APIs must support redacted traces and metrics.
- Hosted team workflows can grow without forcing raw document custody.

