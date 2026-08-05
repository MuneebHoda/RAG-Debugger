import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  mkdir,
  mkdtemp,
  rename,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  applyCandidate,
  readStableCandidateSnapshot,
} from "../../../scripts/autonomy/candidate.mjs";
import {
  buildGitHubApiUrl,
  GitHubClient,
} from "../../../scripts/autonomy/github.mjs";

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

test("stable candidate reads reject symlink replacement", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "autonomy-symlink-race-"));
  const candidate = path.join(root, "candidate.txt");
  const target = path.join(root, "target.txt");
  await writeFile(candidate, "reviewed bytes\n");
  await writeFile(target, "replacement bytes\n");
  await assert.rejects(
    () =>
      readStableCandidateSnapshot({
        root,
        relativePath: "candidate.txt",
        errorMessage: "candidate changed",
        maximumBytes: 1024,
        afterRead: async () => {
          await rm(candidate);
          await symlink(target, candidate);
        },
      }),
    /candidate changed/u,
  );
});

test("stable candidate reads reject regular-file replacement", async () => {
  const root = await mkdtemp(
    path.join(os.tmpdir(), "autonomy-replacement-race-"),
  );
  const candidate = path.join(root, "candidate.txt");
  const replacement = path.join(root, "replacement.txt");
  await writeFile(candidate, "reviewed bytes\n");
  await writeFile(replacement, "replacement bytes\n");
  await assert.rejects(
    () =>
      readStableCandidateSnapshot({
        root,
        relativePath: "candidate.txt",
        errorMessage: "candidate changed",
        maximumBytes: 1024,
        afterRead: () => rename(replacement, candidate),
      }),
    /candidate changed/u,
  );
});

test("stable candidate reads reject in-place content mutation", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "autonomy-content-race-"));
  const candidate = path.join(root, "candidate.txt");
  await writeFile(candidate, "reviewed bytes\n");
  await assert.rejects(
    () =>
      readStableCandidateSnapshot({
        root,
        relativePath: "candidate.txt",
        errorMessage: "candidate changed",
        maximumBytes: 1024,
        afterRead: () => writeFile(candidate, "mutated bytes!\n"),
      }),
    /candidate changed/u,
  );
});

test("candidate application rejects content and hash mismatch", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "autonomy-hash-root-"));
  const artifact = await mkdtemp(
    path.join(os.tmpdir(), "autonomy-hash-artifact-"),
  );
  await mkdir(path.join(artifact, "files"));
  await writeFile(
    path.join(artifact, "manifest.json"),
    JSON.stringify({
      version: 1,
      entries: [
        {
          path: "payload.txt",
          operation: "upsert",
          mode: "100644",
          bytes: 5,
          sha256: sha256("different\n"),
        },
      ],
    }),
  );
  await writeFile(path.join(artifact, "files", "payload.txt"), "safe\n");
  await assert.rejects(
    () => applyCandidate({ repositoryRoot: root, artifactDirectory: artifact }),
    /failed integrity/u,
  );
});

test("GitHub destinations reject origin, credential, scheme, and fragment bypasses", () => {
  for (const endpoint of [
    "https://evil.example/repos/x",
    "http://api.github.com/repos/x",
    "//evil.example/repos/x",
    "https://user:pass@api.github.com/repos/x",
    "/repos/x#https://evil.example",
    "\\\\evil.example\\repos\\x",
    "/repos/MuneebHoda/RAG-Debugger/%2e%2e/issues",
    "/repos/MuneebHoda/RAG-Debugger/%2e%2e%2fissues",
    "/repos/MuneebHoda/RAG-Debugger/%ZZ/issues",
  ]) {
    assert.throws(() => buildGitHubApiUrl(endpoint), /unsafe|canonical/u);
  }
  const encoded = buildGitHubApiUrl(
    "/repos/MuneebHoda/RAG-Debugger/issues/%2F%2Fevil.example",
  );
  assert.equal(encoded.protocol, "https:");
  assert.equal(encoded.origin, "https://api.github.com");
  assert.equal(encoded.username, "");
  assert.equal(encoded.password, "");
  const query = buildGitHubApiUrl(
    "/search/issues?q=https%3A%2F%2Fevil.example%2Fcollect",
  );
  assert.equal(query.origin, "https://api.github.com");
  assert.equal(query.searchParams.get("q"), "https://evil.example/collect");
  assert.equal(
    buildGitHubApiUrl("/repos/MuneebHoda/RAG-Debugger/labels/agent%2Fgenerated")
      .pathname,
    "/repos/MuneebHoda/RAG-Debugger/labels/agent%2Fgenerated",
  );
});

test("GitHub client fixes the HTTPS origin and disables redirects", async () => {
  const calls = [];
  const client = new GitHubClient({
    repository: "MuneebHoda/RAG-Debugger",
    token: "synthetic-test-token",
    transport: async (url, options) => {
      calls.push({ url, options });
      return {
        ok: true,
        status: 200,
        json: async () => ({ number: 27 }),
      };
    },
  });
  await client.getIssue(27);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].url.origin, "https://api.github.com");
  assert.equal(calls[0].url.protocol, "https:");
  assert.equal(calls[0].options.redirect, "error");
  assert.equal(calls[0].url.username, "");
  assert.equal(calls[0].url.password, "");
});

test("GitHub client rejects untyped issue and commit path segments", async () => {
  let calls = 0;
  const client = new GitHubClient({
    repository: "MuneebHoda/RAG-Debugger",
    token: "synthetic-test-token",
    transport: async () => {
      calls += 1;
      return { ok: true, status: 200, json: async () => ({}) };
    },
  });
  assert.throws(() => client.getIssue("27/../outside"), /positive integer/u);
  assert.throws(() => client.getTimeline(0), /positive integer/u);
  assert.throws(
    () => client.getGitCommit("a".repeat(39) + "/"),
    /40 lowercase hexadecimal/u,
  );
  assert.equal(calls, 0);
  await client.getGitCommit("a".repeat(40));
  assert.equal(calls, 1);
});

test("redirect responses fail without a second outbound request", async () => {
  let calls = 0;
  const client = new GitHubClient({
    repository: "MuneebHoda/RAG-Debugger",
    token: "synthetic-test-token",
    transport: async (_url, options) => {
      calls += 1;
      assert.equal(options.redirect, "error");
      return {
        ok: false,
        status: 302,
        json: async () => ({ message: "redirect rejected" }),
      };
    },
  });
  await assert.rejects(() => client.getIssue(27), /failed with 302/u);
  assert.equal(calls, 1);
});
