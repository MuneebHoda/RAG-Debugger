import {
  Activity,
  BadgeCheck,
  BarChart3,
  Binary,
  Boxes,
  Braces,
  BrainCircuit,
  ClipboardCheck,
  DatabaseZap,
  FileSearch,
  Fingerprint,
  Gauge,
  GitBranch,
  KeyRound,
  LockKeyhole,
  Network,
  ScanSearch,
  ShieldCheck,
  Sparkles,
  Users,
  Workflow,
  Zap,
  type LucideIcon,
} from "lucide-react";

import type { PricingTier } from "../../components/ui/PricingCard";

export type PlatformFeature = {
  icon: LucideIcon;
  title: string;
  description: string;
  image?: string;
};

export const productImages = {
  dashboard: "/product/corpuslab-dashboard.png",
  sources: "/product/corpuslab-sources.png",
  retrieval: "/product/corpuslab-retrieval.png",
  evals: "/product/corpuslab-evals.png",
  reports: "/product/corpuslab-reports.png",
};

export const themeImages = {
  hero: "/product/corpuslab-hero-theme.png",
  evidenceMap: "/product/corpuslab-evidence-map.png",
  qualityLayer: "/product/corpuslab-quality-layer.png",
};

export const primaryFeatures: PlatformFeature[] = [
  {
    icon: FileSearch,
    title: "Corpus ingestion",
    description:
      "Ingest PDFs, policies, product docs, contracts, support KBs, research papers, wikis, code docs, and resumes into one explainable corpus layer.",
    image: productImages.sources,
  },
  {
    icon: ScanSearch,
    title: "Extraction quality",
    description:
      "Profile documents, flag extraction warnings, detect weak text density, and surface corpus gaps before they damage answers.",
  },
  {
    icon: Boxes,
    title: "Structured chunking",
    description:
      "Preserve sections, paragraphs, and bullet groups while detecting duplicates, heading-only chunks, and evidence-ready spans.",
  },
  {
    icon: BrainCircuit,
    title: "Embedding readiness",
    description:
      "Track indexed, missing, and stale chunks across embedding models so semantic retrieval quality is visible before launch.",
  },
  {
    icon: Workflow,
    title: "Retrieval comparison",
    description:
      "Compare lexical, vector, and hybrid retrieval with normalized score bars, matched terms, section boosts, and path signals.",
    image: productImages.retrieval,
  },
  {
    icon: BadgeCheck,
    title: "Evidence summaries",
    description:
      "Generate cited evidence summaries with checksum prefixes, section labels, quality flags, and duplicate suppression.",
  },
  {
    icon: ClipboardCheck,
    title: "Retrieval evals",
    description:
      "Create eval cases, compare modes, measure recall@k and precision@k, and track quality regressions over time.",
    image: productImages.evals,
  },
  {
    icon: BarChart3,
    title: "Executive reports",
    description:
      "Produce failed-query diagnosis, weak-evidence warnings, cited evidence, corpus health, and stakeholder-ready reports.",
    image: productImages.reports,
  },
];

export const platformFeatures: PlatformFeature[] = [
  {
    icon: Activity,
    title: "Trace observability",
    description:
      "Inspect retrieval, reranking, generation, prompt, latency, and failure spans across production RAG flows.",
  },
  {
    icon: GitBranch,
    title: "Versioned configs",
    description:
      "Track prompt, chunking, retrieval, embedding, reranking, and model configuration changes across releases.",
  },
  {
    icon: Users,
    title: "Team workspaces",
    description:
      "Collaborate with shared projects, roles, comments, annotations, reports, and audit-ready review history.",
  },
  {
    icon: KeyRound,
    title: "API keys and SDKs",
    description:
      "Connect CI evals, production traces, webhooks, local collectors, and internal tools through a clean API surface.",
  },
  {
    icon: ShieldCheck,
    title: "Privacy controls",
    description:
      "Control redaction, retention, private deployment, local collection, and data boundaries for sensitive corpora.",
  },
  {
    icon: LockKeyhole,
    title: "Enterprise security",
    description:
      "Use SSO/SAML, SCIM, RBAC, audit logs, support controls, and deployment options designed for enterprise review.",
  },
  {
    icon: Gauge,
    title: "Quality gates",
    description:
      "Run evals in CI, block weak releases, compare retrieval modes, and turn production failures into test cases.",
  },
  {
    icon: DatabaseZap,
    title: "Index operations",
    description:
      "Monitor chunk indexing, embeddings, vector search, stale corpora, throughput, and large-corpus readiness.",
  },
  {
    icon: Binary,
    title: "GPU and HPC workers",
    description:
      "Run high-throughput embedding, indexing, reranking, and benchmark jobs for corpora that outgrow one machine.",
  },
  {
    icon: Network,
    title: "Integrations",
    description:
      "Connect LangChain, LlamaIndex, OpenTelemetry, model gateways, data warehouses, document stores, and ticketing systems.",
  },
  {
    icon: Braces,
    title: "Developer tooling",
    description:
      "Use typed contracts, API references, SDK workflows, reproducible reports, and deterministic local debugging loops.",
  },
  {
    icon: Fingerprint,
    title: "Evidence lineage",
    description:
      "Trace every cited answer back to document, source, chunk, checksum, retrieval run, eval case, and report.",
  },
];

export const pricingTiers: PricingTier[] = [
  {
    name: "Developer",
    price: "$0/mo",
    description:
      "For local debugging, learning, and validating a first production corpus.",
    cta: "Start free",
    href: "/signup",
    usage: "Includes a local workspace and starter hosted sync.",
    items: [
      "One workspace",
      "Local corpus workbench",
      "Structured chunking and retrieval comparison",
      "Starter eval and report allowance",
      "Community support",
    ],
  },
  {
    name: "Team",
    price: "$299/mo",
    description:
      "For RAG teams sharing projects, reports, evals, and production traces.",
    cta: "Start team plan",
    href: "/signup",
    featured: true,
    usage:
      "Includes 5 seats and 100k platform units. Extra seats $39/user/mo. Overage $8 per 100k units.",
    items: [
      "Shared projects and team workspaces",
      "Trace observability and report sharing",
      "Eval runs and regression tracking",
      "Team comments and annotations",
      "Email support",
    ],
  },
  {
    name: "Scale",
    price: "$999/mo",
    description:
      "For production RAG organizations with CI gates, APIs, and larger corpora.",
    cta: "Scale CorpusLab",
    href: "/signup",
    usage:
      "Includes 15 seats and 500k platform units. Extra seats $49/user/mo. Volume overage discounts apply.",
    items: [
      "CI evals, API keys, and webhooks",
      "Longer retention and higher throughput",
      "Advanced report workflows",
      "Priority support",
      "Workspace governance",
    ],
  },
  {
    name: "Enterprise",
    price: "Custom",
    description:
      "For regulated, private, high-scale, or GPU-accelerated deployments.",
    cta: "Talk to sales",
    href: "/signup",
    usage:
      "Annual contract with custom usage, retention, and deployment terms.",
    items: [
      "SSO/SAML, SCIM, RBAC, and audit logs",
      "Private deployment and local collector",
      "Custom retention and security review",
      "Dedicated support",
      "GPU/HPC worker capacity",
    ],
  },
];

export const workflowSteps = [
  "Ingest every source",
  "Profile extraction quality",
  "Chunk with evidence signals",
  "Index embeddings",
  "Compare retrieval modes",
  "Run evals",
  "Generate reports",
  "Monitor production traces",
];

export const forbiddenPublicCopy = [
  "coming soon",
  "future",
  "planned",
  "roadmap",
];

export const publicCopyCorpus = [
  ...primaryFeatures.map((item) => `${item.title} ${item.description}`),
  ...platformFeatures.map((item) => `${item.title} ${item.description}`),
  ...pricingTiers.map(
    (item) =>
      `${item.name} ${item.price} ${item.description} ${item.usage} ${item.items.join(" ")}`,
  ),
  workflowSteps.join(" "),
  "CorpusLab helps RAG teams turn messy corpora into explainable, testable, production-grade retrieval systems.",
  "Debug the whole RAG pipeline, not just vector search.",
  "Measure quality with evals instead of vibes.",
  "Produce reports engineers and business teams can both understand.",
  "Support privacy-first local and enterprise hosted deployment.",
  "Subscription plus usage pricing keeps entry simple and scaling fair.",
  "Platform units cover traces, eval scores, report generation, chunk indexing, and embedding work.",
];

export const heroStats = [
  ["98%", "evidence coverage visible"],
  ["3 modes", "lexical, vector, hybrid"],
  ["100k", "Team units included"],
  ["0 guesswork", "every citation explained"],
];

export const trustSignals = [
  { icon: ShieldCheck, label: "Private corpora" },
  { icon: Sparkles, label: "Cited evidence" },
  { icon: Zap, label: "HPC indexing" },
];
