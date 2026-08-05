# Releasing CorpusLab

CorpusLab is a public pre-release project. Releases are deliberate, traceable
checkpoints rather than claims that hosted deployment infrastructure exists.
This guide complements [CONTRIBUTING.md](../CONTRIBUTING.md) and the
[engineering quality policy](engineering-quality.md).

## Version Policy

Use semantic pre-release versions established by the repository:

- `v0.x.0` for a coherent product milestone;
- `v0.x.y` for compatible fixes within that milestone; and
- `v0.x.0-rc.n` for release candidates.

Tags use a `v` prefix. Never reuse, move, or silently replace a published tag.
Only `main` and the latest published pre-release receive best-effort security
support under [SECURITY.md](../SECURITY.md).

## Release Preconditions

Before tagging, assign one maintainer as the release and rollback owner in the
milestone or release issue. That owner verifies:

1. The milestone's intended issues are closed, deferred explicitly, or linked
   from release notes.
2. The release commit is on `main`, required pull requests are merged, and
   required CI checks are green.
3. `CHANGELOG.md` describes user-visible, API, storage, security, privacy, and
   operational changes without exposing sensitive data.
4. Privacy and security changes completed the repository review checklist;
   unresolved advisories, dependency exceptions, and CodeQL findings have an
   explicit disposition.
5. Dependency changes passed Dependency Review and Cargo Deny. New providers or
   data movement have the required documentation and ADR.
6. `/api/v1` compatibility and persisted-data compatibility were reviewed.
   Applied SQLx migrations are additive or have documented forward-compatible
   behavior and rollback constraints.
7. Setup, upgrade, and verification documentation matches the release.

## Release Verification

From a clean checkout of the exact release commit, run:

```sh
just ci-check
git diff --check
```

Confirm the GitHub `Coverage`, Dependency Review, Cargo Deny, CodeQL, Rust, Web,
Database migrations, and Documentation checks completed for that commit.
Coverage percentages remain informational, but report generation and uploads
must be healthy.

Review the generated handbook, confirm no coverage, Playwright, screenshot,
database, uploaded-document, credential, or local environment artifacts are
tracked, and verify the release commit and working tree are clean.

## Tag And Publish

Prepare release notes in a reviewed file or GitHub draft. Include the version,
release commit, major changes, compatibility or migration notes, known issues,
verification performed, and the named rollback owner.

Create and push an annotated tag:

```sh
git switch main
git pull --ff-only origin main
git tag -a vX.Y.Z -m "CorpusLab vX.Y.Z"
git push origin vX.Y.Z
```

Create a GitHub Release from that existing tag, mark it as a pre-release while
CorpusLab is pre-launch, and use the reviewed release notes. Verify that the
release page points to the intended commit and that any checksums or attached
artifacts match their documented source. Do not publish local databases,
customer data, environment files, or credentials.

## Post-Release Checks

After publishing:

1. Open the release and tag from a signed-out browser session.
2. Confirm source archives resolve to the intended commit.
3. Follow the documented local setup against a clean database.
4. Run migrations, start the API and web app, verify readiness, authenticate,
   and exercise the guided demo's ingest-to-report path.
5. Recheck the security advisory and dependency status for the released commit.
6. Record results and any follow-up issue links on the release or milestone.

These are local product smoke checks; they do not imply a hosted deployment.

## Rollback And Correction

The named release owner decides whether to correct forward or roll back the
application change. Roll back when a release exposes sensitive data, breaks
authentication or workspace isolation, corrupts persisted data, prevents clean
startup or migration, or causes a severe retrieval-quality regression without
a safe mitigation.

- Revert application commits through a reviewed pull request and publish a new
  patch or release-candidate version. Do not force-push `main` or move a tag.
- Never edit, delete, or reorder an applied database migration. Add a new
  forward-fix migration that restores a safe schema or data state and document
  whether application rollback must wait for it.
- If credentials or release automation were compromised, revoke and rotate
  them, preserve sanitized audit evidence, and use private vulnerability
  reporting to coordinate remediation.
- If a published release is incorrect or unsafe, mark it clearly on GitHub,
  explain the affected versions and mitigation, and publish a new version.
  Remove unsafe downloadable artifacts when necessary, but retain an explicit
  public notice; never silently rewrite tags, release notes, or published
  history.
- Use a GitHub Security Advisory when coordinated disclosure or a CVE is
  appropriate, and link the corrected release after publication.

Close the milestone only after release verification and correction ownership
are recorded.
