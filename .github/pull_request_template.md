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
