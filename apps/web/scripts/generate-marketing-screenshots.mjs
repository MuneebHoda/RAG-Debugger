/* global console */

import { mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const productDir = resolve(scriptDir, "../public/product");

await mkdir(productDir, { recursive: true });

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 960 } });

const screens = [
  {
    file: "corpuslab-dashboard.png",
    eyebrow: "Corpus command center",
    title: "Evidence health across every RAG workspace",
    statA: ["12.8k", "documents indexed"],
    statB: ["94%", "eval pass rate"],
    statC: ["99.1%", "embedding readiness"],
    panels: [
      [
        "Corpus health",
        "Extraction warnings, duplicate chunks, stale indexes, and weak evidence are visible before release.",
      ],
      [
        "Retrieval quality",
        "Lexical, vector, and hybrid ranking are scored side by side with cited evidence.",
      ],
      [
        "Reports",
        "Stakeholders get export-ready failed-query diagnosis with evidence lineage.",
      ],
    ],
  },
  {
    file: "corpuslab-sources.png",
    eyebrow: "Sources",
    title: "Profiles, warnings, and chunk quality for every document",
    statA: ["8", "document profiles"],
    statB: ["416", "quality flags"],
    statC: ["0", "stored binaries"],
    panels: [
      [
        "policy-handbook.pdf",
        "policy_or_legal · 126 chunks · 4 extraction warnings",
      ],
      ["support-kb.html", "support_kb · 88 chunks · 12 duplicate chunks"],
      [
        "architecture-spec.md",
        "technical_docs · 64 chunks · strong evidence candidate",
      ],
    ],
  },
  {
    file: "corpuslab-retrieval.png",
    eyebrow: "Retrieval diagnosis",
    title: "Evidence summaries with grouped hits and score bars",
    statA: ["Hybrid", "active mode"],
    statB: ["12 ms", "query latency"],
    statC: ["5", "cited chunks"],
    panels: [
      [
        "Evidence summary",
        "The refund workflow is supported by policy section 4.2 and support article 117, with one weak citation suppressed.",
      ],
      [
        "Rank 1 · policy-handbook.pdf",
        "semantic 92 · lexical 74 · section 41 · path 28",
      ],
      [
        "Rank 2 · support-kb.html",
        "semantic 88 · lexical 66 · citation strength strong",
      ],
    ],
  },
  {
    file: "corpuslab-evals.png",
    eyebrow: "Evals",
    title: "Regression tracking for retrieval quality",
    statA: ["87%", "recall@5"],
    statB: ["72%", "precision@5"],
    statC: ["31/36", "cases passed"],
    panels: [
      ["Payment policy coverage", "pass · top hit rank 1 · hybrid wins"],
      ["GPU worker limits", "pass · vector finds semantic match"],
      ["Stale document query", "fail · missing source evidence"],
    ],
  },
  {
    file: "corpuslab-reports.png",
    eyebrow: "Reports",
    title: "Audit-ready diagnosis for failed and high-risk queries",
    statA: ["14", "evidence issues"],
    statB: ["6", "owners assigned"],
    statC: ["PDF", "export ready"],
    panels: [
      [
        "Failed-query diagnosis",
        "Query failed because the top evidence was heading-only and the expected policy source was not indexed.",
      ],
      [
        "Evidence lineage",
        "source · document · chunk · checksum · retrieval run · eval case · report",
      ],
      [
        "Business review",
        "Share cited evidence, quality warnings, and next actions with product and compliance teams.",
      ],
    ],
  },
];

for (const screen of screens) {
  await page.setContent(renderScreen(screen), { waitUntil: "networkidle" });
  await page.locator("#screen").screenshot({
    path: resolve(productDir, screen.file),
  });
  console.log(`Generated ${screen.file}`);
}

await browser.close();

function renderScreen(screen) {
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <style>
      * { box-sizing: border-box; }
      body {
        margin: 0;
        min-height: 100vh;
        display: grid;
        place-items: center;
        background: #101820;
        color: #101820;
        font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      }
      #screen {
        width: 1360px;
        height: 860px;
        overflow: hidden;
        border: 1px solid rgba(255,255,255,0.2);
        border-radius: 30px;
        background: linear-gradient(135deg, #f9fbf3 0%, #eef6f7 48%, #dcebea 100%);
        box-shadow: 0 40px 110px rgba(0,0,0,0.38);
        padding: 34px;
      }
      .chrome {
        display: grid;
        grid-template-columns: 238px 1fr;
        gap: 22px;
        height: 100%;
      }
      .sidebar,
      .main,
      .panel,
      .stat {
        border: 1px solid rgba(16,24,32,0.12);
        border-radius: 18px;
        background: rgba(255,255,255,0.82);
        box-shadow: 0 16px 42px rgba(16,24,32,0.1);
      }
      .sidebar {
        display: grid;
        align-content: space-between;
        padding: 22px;
      }
      .logo {
        display: flex;
        align-items: center;
        gap: 10px;
        font-weight: 950;
        font-size: 24px;
      }
      .mark {
        display: grid;
        place-items: center;
        width: 40px;
        height: 40px;
        border-radius: 12px;
        background: #101820;
        color: #d5ff5f;
      }
      .nav {
        display: grid;
        gap: 10px;
      }
      .nav span {
        display: block;
        border-radius: 12px;
        padding: 12px 13px;
        color: #4d5f6e;
        font-weight: 850;
      }
      .nav span:first-child,
      .nav span:nth-child(3) {
        background: #101820;
        color: #ffffff;
      }
      .privacy {
        border-radius: 14px;
        background: rgba(12,135,146,0.1);
        color: #0b5f66;
        font-size: 13px;
        font-weight: 850;
        padding: 14px;
      }
      .main {
        display: grid;
        grid-template-rows: auto auto 1fr;
        gap: 18px;
        padding: 26px;
      }
      .top {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 18px;
      }
      .top small,
      .eyebrow {
        color: #0b5f66;
        font-size: 12px;
        font-weight: 950;
        letter-spacing: 0.08em;
        text-transform: uppercase;
      }
      h1 {
        max-width: 860px;
        margin: 8px 0 0;
        color: #101820;
        font-size: 58px;
        line-height: 1;
      }
      .status {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        border-radius: 999px;
        background: #101820;
        color: #ffffff;
        font-weight: 900;
        padding: 9px 13px;
      }
      .dot {
        width: 9px;
        height: 9px;
        border-radius: 999px;
        background: #d5ff5f;
      }
      .stats {
        display: grid;
        grid-template-columns: repeat(3, minmax(0, 1fr));
        gap: 12px;
      }
      .stat {
        padding: 18px;
      }
      .stat strong {
        display: block;
        color: #101820;
        font-size: 36px;
      }
      .stat span {
        color: #617080;
        font-size: 13px;
        font-weight: 850;
      }
      .grid {
        display: grid;
        grid-template-columns: 1.1fr 0.9fr;
        gap: 14px;
        min-height: 0;
      }
      .panel {
        padding: 20px;
      }
      .panel h2 {
        margin: 0 0 12px;
        color: #101820;
        font-size: 22px;
      }
      .panel p {
        margin: 0;
        color: #4d5f6e;
        font-size: 17px;
        line-height: 1.55;
      }
      .panelList {
        display: grid;
        gap: 12px;
      }
      .score {
        display: grid;
        gap: 8px;
      }
      .bar {
        height: 10px;
        overflow: hidden;
        border-radius: 999px;
        background: #d8dee8;
      }
      .bar i {
        display: block;
        height: 100%;
        width: 78%;
        border-radius: inherit;
        background: linear-gradient(90deg, #0c8792, #d5ff5f);
      }
      .tagRow {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
        margin-top: 16px;
      }
      .tag {
        border-radius: 999px;
        background: rgba(12,135,146,0.12);
        color: #0b5f66;
        font-size: 12px;
        font-weight: 900;
        padding: 7px 9px;
      }
    </style>
  </head>
  <body>
    <div id="screen">
      <div class="chrome">
        <aside class="sidebar">
          <div class="logo"><span class="mark">CL</span>CorpusLab</div>
          <nav class="nav">
            <span>Overview</span>
            <span>Sources</span>
            <span>Retrieval</span>
            <span>Evals</span>
            <span>Reports</span>
            <span>Settings</span>
          </nav>
          <div class="privacy">Private corpus controls active</div>
        </aside>
        <main class="main">
          <div class="top">
            <span class="eyebrow">${escapeHtml(screen.eyebrow)}</span>
            <span class="status"><span class="dot"></span>workspace healthy</span>
          </div>
          <header>
            <h1>${escapeHtml(screen.title)}</h1>
          </header>
          <section class="stats">
            ${renderStat(screen.statA)}
            ${renderStat(screen.statB)}
            ${renderStat(screen.statC)}
          </section>
          <section class="grid">
            <div class="panel panelList">
              ${screen.panels.map((panel) => renderPanel(panel)).join("")}
            </div>
            <div class="panel">
              <h2>Quality breakdown</h2>
              <div class="score">
                ${["semantic", "lexical", "section", "citation", "coverage", "freshness"].map((name, index) => renderBar(name, 92 - index * 8)).join("")}
              </div>
              <div class="tagRow">
                <span class="tag">hybrid ranking</span>
                <span class="tag">citations</span>
                <span class="tag">eval gates</span>
                <span class="tag">audit lineage</span>
              </div>
            </div>
          </section>
        </main>
      </div>
    </div>
  </body>
</html>`;
}

function renderStat([value, label]) {
  return `<article class="stat"><strong>${escapeHtml(value)}</strong><span>${escapeHtml(label)}</span></article>`;
}

function renderPanel([title, body]) {
  return `<article class="panel"><h2>${escapeHtml(title)}</h2><p>${escapeHtml(body)}</p></article>`;
}

function renderBar(name, width) {
  return `<div><small>${escapeHtml(name)} · ${width}</small><div class="bar"><i style="width:${width}%"></i></div></div>`;
}

function escapeHtml(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
