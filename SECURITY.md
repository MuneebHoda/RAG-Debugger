# Security Policy

CorpusLab is a public pre-release project. Security reports are handled through
GitHub Security Advisories so sensitive details do not enter the public issue
tracker.

## Supported Versions

| Version | Security support |
| --- | --- |
| `main` | Supported on a best-effort basis |
| Latest published pre-release | Supported on a best-effort basis |
| Older tags, branches, and snapshots | No guaranteed security fixes |

Pre-release compatibility and support may change as the product evolves. When a
fix is available, maintainers may ask reporters to verify it against `main` or a
new pre-release.

## Report A Vulnerability

Do not open a public GitHub issue for a suspected vulnerability. Submit a
[private vulnerability report](https://github.com/MuneebHoda/RAG-Debugger/security/advisories/new)
instead.

Include only the information needed to assess the report:

- affected version, tag, or commit;
- minimal reproduction steps;
- expected security boundary and observed behavior;
- likely impact and affected components;
- suggested remediation, if known; and
- sanitized logs, screenshots, or other evidence.

Never include passwords, API keys, session cookies, access tokens, database
credentials, secret hashes, authorization headers, customer corpus content,
document chunks, queries, prompts, traces, reports, or other sensitive data.
Use synthetic data and redact environment-specific identifiers. If sensitive
material is necessary to establish impact, first describe its type and wait for
maintainer guidance before transmitting it.

## Response And Disclosure

Reports are acknowledged and assessed on a best-effort basis. This project does
not promise a response deadline, remediation service level, or bug bounty.
Maintainers will attempt to confirm scope, coordinate a fix, and agree on a
reasonable disclosure timeline with the reporter.

Please keep vulnerability details private until a fix or mitigation is
available and coordinated disclosure is complete. Maintainers may publish a
GitHub Security Advisory, release notes, upgrade guidance, and credit when the
reporter consents. If a report is not actionable or falls outside the supported
versions, maintainers will explain that decision when practical.

## Local-First And Credential Boundaries

CorpusLab is designed to keep corpus data, chunks, embeddings, queries, traces,
and reports within the configured local or private storage boundary. A report
that suggests this boundary was crossed should identify the affected component
without attaching the underlying private content.

Credentials must never be committed, logged, pasted into issues, or included in
advisory attachments. Revoke or rotate any credential that may have been
exposed, then report the exposure using redacted identifiers only.

## Containment And Remediation

When a credible exposure is identified, take proportionate steps without
assuming hosted incident-response infrastructure exists:

- stop access to or further distribution of affected releases, artifacts, or
  links where practical;
- revoke affected credentials and sessions before continuing investigation;
- preserve sanitized evidence needed to understand the event without copying
  private corpus, query, trace, credential, or customer content;
- assess the exposed data classes, affected versions, duration, and impacted
  users;
- coordinate remediation and disclosure privately through the advisory;
- make required user, contractual, legal, or regulatory notifications through
  the appropriate channels; and
- publish a visible corrected release and mitigation guidance without moving
  tags or silently rewriting published history.

Containment actions, evidence handling, remediation ownership, and disclosure
decisions should be recorded in the private advisory using redacted details.
