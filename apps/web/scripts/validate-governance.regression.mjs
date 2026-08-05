import assert from "node:assert/strict";
import test from "node:test";

import {
  PRIVATE_ADVISORY_URL,
  extractMarkdownLinkDestinations,
  validateCanonicalAdvisoryLinks,
} from "./validate-governance.mjs";

function reportingLink(destination) {
  return `Report privately through [GitHub private vulnerability reporting](${destination}).`;
}

test("accepts the exact private advisory link destination", () => {
  const markdown = reportingLink(PRIVATE_ADVISORY_URL);

  assert.deepEqual(extractMarkdownLinkDestinations(markdown), [
    PRIVATE_ADVISORY_URL,
  ]);
  assert.doesNotThrow(() =>
    validateCanonicalAdvisoryLinks("valid.md", markdown),
  );
});

test("rejects an evil host embedding the canonical URL", () => {
  const destination = `https://evil.example/redirect/${PRIVATE_ADVISORY_URL}`;

  assert.throws(
    () =>
      validateCanonicalAdvisoryLinks(
        "evil-host.md",
        reportingLink(destination),
      ),
    /non-canonical vulnerability-reporting URL/,
  );
});

test("rejects the canonical host with an incorrect path", () => {
  for (const destination of [
    `${PRIVATE_ADVISORY_URL}/archive`,
    `https://github.com/redirect/${PRIVATE_ADVISORY_URL}`,
  ]) {
    assert.throws(
      () =>
        validateCanonicalAdvisoryLinks(
          "wrong-path.md",
          reportingLink(destination),
        ),
      /non-canonical vulnerability-reporting URL/,
    );
  }
});

test("rejects query-string and fragment variants", () => {
  for (const suffix of ["?next=public", "#public"]) {
    assert.throws(
      () =>
        validateCanonicalAdvisoryLinks(
          "variant.md",
          reportingLink(`${PRIVATE_ADVISORY_URL}${suffix}`),
        ),
      /non-canonical vulnerability-reporting URL/,
    );
  }
});

test("rejects a plain-text URL that is not a Markdown destination", () => {
  assert.throws(
    () =>
      validateCanonicalAdvisoryLinks(
        "plain-text.md",
        `Report privately at ${PRIVATE_ADVISORY_URL}.`,
      ),
    /must link to the private advisory URL/,
  );
});

test("rejects a non-canonical reference beside the canonical link", () => {
  const markdown = [
    reportingLink(PRIVATE_ADVISORY_URL),
    reportingLink(`${PRIVATE_ADVISORY_URL}?duplicate=true`),
  ].join("\n");

  assert.throws(
    () => validateCanonicalAdvisoryLinks("mixed.md", markdown),
    /non-canonical vulnerability-reporting URL/,
  );
});
