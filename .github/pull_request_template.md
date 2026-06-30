## Summary

- 

## Screenshots / Demo

- UI changes: add screenshots or a short recording.
- Non-UI changes: write `N/A`.

## Tests Run

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cd apps/web && npm run typecheck`
- [ ] `cd apps/web && npm run lint`
- [ ] `cd apps/web && npm test -- --run`
- [ ] `cd apps/web && npm run build`
- [ ] `cd apps/web && npm run format:check`
- [ ] `cd apps/web && npx playwright test`
- [ ] `just docs-pdf`
- [ ] `docker compose up -d postgres && sqlx migrate run`

## Migrations / Compatibility

- Database migrations:
- API compatibility:
- Data backfill or rollback notes:

## Privacy / Security

- [ ] No raw documents, chunks, queries, prompts, answers, or vectors are newly sent outside the documented storage boundary, or the movement is documented and approved
- [ ] No passwords, tokens, cookies, API keys, authorization headers, secret hashes, or database/provider credentials are logged
- [ ] API-key, session, role, retention, and deletion behavior is unchanged or documented and tested
- [ ] Trace, report, eval, and corpus export/sharing behavior is unchanged or explicitly redacted and user initiated
- [ ] `docs/privacy-security.md`, the privacy checklist, logging policy, and any required ADR are updated or marked `N/A` with a reason

## Docs Freshness

- [ ] I checked whether `README.md` or `docs/development.md` needs an update
- [ ] I checked whether `docs/architecture.md` needs an update
- [ ] I checked whether the relevant feature guide needs an update
- [ ] I checked whether `docs/testing.md` needs an update
- [ ] I checked whether `docs/technical-handbook.md` and PDF need an update
- [ ] I checked whether an ADR is required
- [ ] I updated `CHANGELOG.md` or wrote `N/A` with a reason

## Risks And Rollback

- Main risk:
- Rollback plan:
