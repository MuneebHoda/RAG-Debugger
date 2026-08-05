# ADR 0005: CodeRabbit Pull Request Review Provider

## Status

Accepted.

## Context

CorpusLab uses pull requests and automated checks to protect code quality. Static analysis and tests catch deterministic failures, but they do not provide a focused review of each change's correctness, maintainability, or security implications.

An automated review provider needs access to pull request diffs, repository context, review metadata, and check results. This is an external processing boundary for engineering data, but it must not become part of the CorpusLab runtime or receive customer corpus data.

## Decision

Install the CodeRabbit GitHub App only for `MuneebHoda/RAG-Debugger` and keep its behavior versioned in the root `.coderabbit.yaml`.

CodeRabbit uses the `chill` profile and reviews ready pull requests automatically. Its summary and review status are advisory: it does not submit blocking request-changes reviews, modify code, generate tests or docstrings, fix CI, resolve conflicts, prompt other AI agents, or reply to chat automatically.

The provider may process repository source, pull request diffs, review conversations, commit metadata, and CI status required to produce a review. It is not connected to CorpusLab APIs, databases, workspaces, traces, documents, chunks, embeddings, queries, credentials, or deployed runtime environments. Repository secrets must never be placed in pull request content or logs.

## Consequences

- Pull requests receive an additional nonblocking, high-signal review and summary.
- CodeRabbit has external access to engineering data in this repository under the GitHub App permissions approved during installation.
- Runtime privacy, local-first corpus handling, branch protection, and existing CI gates remain unchanged.
- Configuration changes are reviewed and versioned like other engineering policy changes.
- Rollback removes the repository from the CodeRabbit GitHub App installation and reverts `.coderabbit.yaml`; no application or database rollback is required.
