#!/usr/bin/env bash
set -euo pipefail

repository_root="${1:?repository root is required}"
cd "$repository_root"

before_diff="$(git diff --binary HEAD | git hash-object --stdin)"

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace

(
  cd apps/web
  npm run format:check
  npm run typecheck
  npm run lint
  npm run test:unit -- --run
  npm run build
  npm run size:check
  npx playwright test
  npm run docs:pdf
)

docker compose up -d postgres
postgres_ready=false
for _attempt in $(seq 1 30); do
  if docker compose exec -T postgres pg_isready -U postgres -d rag_debugger >/dev/null 2>&1; then
    postgres_ready=true
    break
  fi
  sleep 2
done
if [ "$postgres_ready" != "true" ]; then
  echo "PostgreSQL did not become ready within 60 seconds." >&2
  docker compose logs postgres >&2 || true
  exit 1
fi
DATABASE_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost:5432/rag_debugger}" sqlx migrate run
DATABASE_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost:5432/rag_debugger}" \
  cargo test -p rag-debugger-storage --test evidence_repository_contract \
  postgres_evidence_repository_is_deterministic_and_bounded -- --ignored
DATABASE_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost:5432/rag_debugger}" \
  cargo test -p rag-debugger-storage --test eval_workspace_contract \
  postgres_eval_repository_enforces_workspace_ownership -- --ignored
DATABASE_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost:5432/rag_debugger}" \
  cargo test -p rag-debugger-storage --test runtime_workspace_contract \
  postgres_runtime_repository_enforces_workspace_ownership -- --ignored
DATABASE_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost:5432/rag_debugger}" \
  cargo test -p rag-debugger-storage --test workspace_migration \
  workspace_ownership_migration_backfills_singletons_and_quarantines_ambiguity -- --ignored
DATABASE_URL="${DATABASE_URL:-postgres://postgres:postgres@localhost:5432/rag_debugger}" \
  cargo test -p rag-debugger-storage postgres_evidence_query_plans_are_index_compatible -- --ignored

cargo deny --workspace --all-features --locked check advisories bans licenses sources
git diff --check

after_diff="$(git diff --binary HEAD | git hash-object --stdin)"
test "$before_diff" = "$after_diff" || {
  echo "Quality commands changed tracked candidate content." >&2
  exit 1
}
