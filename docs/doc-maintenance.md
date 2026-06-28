# Documentation Maintenance

Documentation changes ship in the same pull request as the behavior or boundary they describe.

## When Documentation Must Change

Review documentation when changing:

- Routes or API response fields.
- Environment variables.
- Migrations or storage behavior.
- Setup and quality commands.
- Architecture or ownership boundaries.
- Authentication, sessions, or API keys.
- Privacy, logging, sync, or export behavior.
- Workbench workflows.
- Eval Lab metrics, gates, or failure labels.
- CI workflow behavior.

## Required Documents By Change Type

### API Change

- `docs/architecture.md`.
- The relevant feature guide, such as `docs/trace-debugger.md` or `docs/eval-lab.md`.
- `docs/technical-handbook.md` and its generated PDF.
- `CHANGELOG.md`.

### Storage Or Migration Change

- Migration file when the schema changes.
- `docs/development.md`.
- `docs/architecture.md`.
- Relevant feature and testing guides.
- ADR when the storage design changes.
- `CHANGELOG.md`.

### Frontend Workflow Change

- Relevant feature guide.
- `docs/frontend-architecture.md` when ownership changes.
- Screenshots or a recording in the PR when behavior is visible.
- `docs/testing.md` when the test flow changes.
- `CHANGELOG.md`.

### Quality Or CI Change

- `README.md`.
- `docs/development.md`.
- `docs/engineering-quality.md`.
- `.github/pull_request_template.md` when contributor expectations change.
- `CHANGELOG.md`.

## ADR Triggers

Add an ADR for:

- A new storage backend.
- A new worker or deployment model.
- Hosted or cloud synchronization behavior.
- Privacy-boundary changes.
- External model or provider integration.
- Breaking API version changes.
- Auth provider replacement.
- Billing or tenant-model decisions.

## Changelog Rules

- Record user-visible behavior, compatibility changes, architecture boundaries, and quality-gate changes under `Unreleased`.
- Do not record formatting-only edits or typo fixes unless they correct unsafe instructions.
- Move entries into a versioned section when creating the release tag.
- Use `N/A` with a reason in the PR when no changelog entry is appropriate.

## Versioned Generated Documents

The Markdown source is authoritative. Regenerate `docs/technical-handbook.pdf` whenever `docs/technical-handbook.md` changes:

```sh
just docs-pdf
```

Other generated PDFs, screenshots, local uploads, test output, and runtime data are not committed unless a documentation or marketing task explicitly curates them.
