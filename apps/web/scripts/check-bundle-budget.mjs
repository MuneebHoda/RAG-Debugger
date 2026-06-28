/* global console, process */

import { readdir, readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { gzipSync } from "node:zlib";

const assetsDirectory = resolve(import.meta.dirname, "../dist/assets");
const budgets = {
  javascript: 180 * 1024,
  css: 20 * 1024,
};

const files = await readdir(assetsDirectory);
const totals = { javascript: 0, css: 0 };

for (const file of files) {
  const kind = file.endsWith(".js")
    ? "javascript"
    : file.endsWith(".css")
      ? "css"
      : null;
  if (!kind) continue;
  totals[kind] += gzipSync(
    await readFile(resolve(assetsDirectory, file)),
  ).byteLength;
}

let failed = false;
for (const kind of Object.keys(budgets)) {
  const total = totals[kind];
  const budget = budgets[kind];
  console.log(
    `${kind}: ${formatBytes(total)} gzip / ${formatBytes(budget)} budget`,
  );
  if (total > budget) failed = true;
}

if (failed) {
  console.error(
    "Bundle budget exceeded. Route-split or remove unnecessary code.",
  );
  process.exitCode = 1;
}

function formatBytes(bytes) {
  return `${(bytes / 1024).toFixed(1)} KB`;
}
