/* global console */

import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "../../..");
const markdownPath = resolve(repoRoot, "docs/technical-handbook.md");
const pdfPath = resolve(repoRoot, "docs/technical-handbook.pdf");

const markdown = await readFile(markdownPath, "utf8");
const html = renderMarkdown(markdown);

const browser = await chromium.launch();
const page = await browser.newPage();
await page.setContent(html, { waitUntil: "networkidle" });
await page.pdf({
  path: pdfPath,
  format: "Letter",
  printBackground: true,
  margin: {
    top: "0.65in",
    right: "0.7in",
    bottom: "0.65in",
    left: "0.7in",
  },
});
await browser.close();

console.log(`Generated ${pdfPath}`);

function renderMarkdown(source) {
  const lines = source.split("\n");
  const body = [];
  let inCode = false;
  let inList = false;
  let code = [];

  const closeList = () => {
    if (inList) {
      body.push("</ul>");
      inList = false;
    }
  };

  const closeCode = () => {
    if (inCode) {
      body.push(`<pre><code>${escapeHtml(code.join("\n"))}</code></pre>`);
      code = [];
      inCode = false;
    }
  };

  for (const line of lines) {
    if (line.startsWith("```")) {
      if (inCode) {
        closeCode();
      } else {
        closeList();
        inCode = true;
      }
      continue;
    }

    if (inCode) {
      code.push(line);
      continue;
    }

    if (line.startsWith("# ")) {
      closeList();
      body.push(`<h1>${inline(line.slice(2))}</h1>`);
      continue;
    }
    if (line.startsWith("## ")) {
      closeList();
      body.push(`<h2>${inline(line.slice(3))}</h2>`);
      continue;
    }
    if (line.startsWith("### ")) {
      closeList();
      body.push(`<h3>${inline(line.slice(4))}</h3>`);
      continue;
    }
    if (line.startsWith("- ")) {
      if (!inList) {
        body.push("<ul>");
        inList = true;
      }
      body.push(`<li>${inline(line.slice(2))}</li>`);
      continue;
    }
    if (line.trim() === "") {
      closeList();
      continue;
    }

    closeList();
    body.push(`<p>${inline(line)}</p>`);
  }

  closeCode();
  closeList();

  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>CorpusLab Technical Handbook</title>
  <style>
    body {
      color: #17202a;
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      font-size: 11pt;
      line-height: 1.55;
    }
    h1 { font-size: 25pt; margin: 0 0 18px; }
    h2 { border-top: 1px solid #d8dee8; font-size: 16pt; margin: 24px 0 10px; padding-top: 14px; }
    h3 { font-size: 12pt; margin: 16px 0 8px; }
    p { margin: 0 0 9px; }
    ul { margin: 0 0 10px 20px; padding: 0; }
    li { margin: 0 0 4px; }
    code { background: #eef3f6; border-radius: 4px; font-size: 9.5pt; padding: 1px 4px; }
    pre { background: #17202a; border-radius: 8px; color: #f6f8fb; overflow: hidden; padding: 12px; }
    pre code { background: transparent; color: inherit; padding: 0; }
  </style>
</head>
<body>${body.join("\n")}</body>
</html>`;
}

function inline(value) {
  return escapeHtml(value).replace(/`([^`]+)`/g, "<code>$1</code>");
}

function escapeHtml(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
