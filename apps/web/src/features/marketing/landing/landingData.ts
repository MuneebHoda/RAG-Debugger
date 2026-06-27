import {
  BarChart3,
  Blocks,
  BrainCircuit,
  FileSearch,
  GitCompareArrows,
  ScanSearch,
  Share2,
  ShieldCheck,
  type LucideIcon,
} from "lucide-react";

import { productImages, themeImages } from "../marketingData";

export type FailureStage = {
  id: "extract" | "chunk" | "embed" | "retrieve" | "evaluate";
  label: string;
  title: string;
  description: string;
  diagnosis: string;
  action: string;
  image: string;
  icon: LucideIcon;
};

export type DemoQuery = {
  id: "policy" | "support" | "platform";
  label: string;
  question: string;
  evidence: string;
  citation: string;
};

export type DemoMode = {
  id: "lexical" | "vector" | "hybrid";
  label: string;
  score: number;
  latency: number;
  diagnosis: string;
  breakdown: Array<{ label: string; value: number }>;
};

export type ProductTourItem = {
  id: "dashboard" | "corpus" | "retrieval" | "quality" | "reports";
  label: string;
  title: string;
  description: string;
  image: string;
  alt: string;
};

export type CapabilityGroup = {
  label: string;
  title: string;
  description: string;
  signals: string[];
  icon: LucideIcon;
};

export const outcomes = [
  ["98%", "evidence coverage visible"],
  ["3 modes", "lexical, vector, hybrid"],
  ["100k", "team units included"],
  ["0", "unexplained citations"],
] as const;

export const failureStages: FailureStage[] = [
  {
    id: "extract",
    label: "Extract",
    title: "Bad text creates invisible evidence.",
    description:
      "CorpusLab profiles extraction density, document structure, and warnings before malformed content reaches retrieval.",
    diagnosis: "A policy table was flattened and lost its exception labels.",
    action: "Inspect extraction warnings and reprocess the document.",
    image: themeImages.evidenceMap,
    icon: FileSearch,
  },
  {
    id: "chunk",
    label: "Chunk",
    title: "Context can disappear at the boundary.",
    description:
      "Structured chunking preserves headings, paragraphs, and evidence groups while flagging duplicates and weak fragments.",
    diagnosis:
      "The answer and its qualifying clause landed in separate chunks.",
    action: "Review chunk boundaries and evidence-quality signals.",
    image: productImages.sources,
    icon: Blocks,
  },
  {
    id: "embed",
    label: "Embed",
    title: "An incomplete index behaves like missing knowledge.",
    description:
      "Embedding readiness makes missing, stale, and model-mismatched chunks visible before semantic search is trusted.",
    diagnosis:
      "Twelve updated chunks were absent from the active embedding index.",
    action: "Refresh the index and verify the configured model snapshot.",
    image: themeImages.qualityLayer,
    icon: BrainCircuit,
  },
  {
    id: "retrieve",
    label: "Retrieve",
    title: "Relevant evidence can exist and still rank too low.",
    description:
      "Compare lexical, vector, and hybrid ranking with normalized score contributions, matched terms, and citation strength.",
    diagnosis:
      "The correct policy ranked fourth behind three heading-only matches.",
    action: "Compare modes and inspect why each chunk received its rank.",
    image: productImages.retrieval,
    icon: ScanSearch,
  },
  {
    id: "evaluate",
    label: "Evaluate",
    title: "Quality needs a release decision, not a hunch.",
    description:
      "Deterministic datasets, experiments, and CI gates measure retrieval changes before they reach production.",
    diagnosis:
      "A chunking change reduced recall@5 across critical support questions.",
    action: "Open failed cases and block the release until evidence recovers.",
    image: productImages.evals,
    icon: BarChart3,
  },
];

export const demoQueries: DemoQuery[] = [
  {
    id: "policy",
    label: "Policy exception",
    question: "When can an enterprise customer receive a refund exception?",
    evidence:
      "Enterprise refund exceptions require director approval and a documented service-impact event.",
    citation: "policy-handbook.pdf · section 4.2 · chunk 18",
  },
  {
    id: "support",
    label: "Support escalation",
    question: "Which incidents require an immediate support escalation?",
    evidence:
      "Security exposure, payment failure, and multi-region service loss trigger immediate escalation.",
    citation: "support-operations.md · Escalation · chunk 7",
  },
  {
    id: "platform",
    label: "GPU indexing",
    question: "How do GPU workers improve corpus indexing?",
    evidence:
      "GPU workers batch embedding and reranking operations while preserving the same quality-gate snapshot.",
    citation: "platform-guide.md · Index workers · chunk 12",
  },
];

export const demoModes: DemoMode[] = [
  {
    id: "lexical",
    label: "Lexical",
    score: 72,
    latency: 8,
    diagnosis:
      "Fast exact-term match, but one semantic qualifier ranks below the cutoff.",
    breakdown: [
      { label: "Term match", value: 92 },
      { label: "Phrase", value: 68 },
      { label: "Meaning", value: 34 },
    ],
  },
  {
    id: "vector",
    label: "Vector",
    score: 86,
    latency: 14,
    diagnosis:
      "Strong semantic match with less support from exact policy language.",
    breakdown: [
      { label: "Term match", value: 51 },
      { label: "Phrase", value: 61 },
      { label: "Meaning", value: 94 },
    ],
  },
  {
    id: "hybrid",
    label: "Hybrid",
    score: 96,
    latency: 12,
    diagnosis:
      "Best balance: exact policy terms and semantic context agree on the top evidence.",
    breakdown: [
      { label: "Term match", value: 91 },
      { label: "Phrase", value: 82 },
      { label: "Meaning", value: 96 },
    ],
  },
];

export const capabilityGroups: CapabilityGroup[] = [
  {
    label: "Build",
    title: "Create evidence-ready corpora.",
    description:
      "Turn heterogeneous documents into inspectable text, structured chunks, and versioned indexes.",
    signals: ["Document profiles", "Extraction findings", "Chunk quality"],
    icon: Blocks,
  },
  {
    label: "Test",
    title: "Interrogate retrieval before users do.",
    description:
      "Ask real questions, compare ranking modes, and see the exact evidence behind every result.",
    signals: ["Mode comparison", "Score lineage", "Cited evidence"],
    icon: GitCompareArrows,
  },
  {
    label: "Debug",
    title: "Explain the complete run.",
    description:
      "Follow query, retrieval, evidence, and evaluation spans with deterministic failure labels.",
    signals: ["Run timeline", "Failure diagnosis", "Rerun comparison"],
    icon: ScanSearch,
  },
  {
    label: "Measure",
    title: "Turn important questions into gates.",
    description:
      "Protect recall, precision, citation coverage, and latency with datasets and CI experiments.",
    signals: ["Eval datasets", "Regression gates", "Release reports"],
    icon: BarChart3,
  },
  {
    label: "Share",
    title: "Give every reviewer the evidence.",
    description:
      "Package technical diagnoses for product, compliance, support, and engineering decisions.",
    signals: ["Audit history", "Evidence reports", "Workspace access"],
    icon: Share2,
  },
];

export const productTour: ProductTourItem[] = [
  {
    id: "dashboard",
    label: "Home",
    title: "Know what needs attention now.",
    description:
      "Mission Control connects corpus health, embedding readiness, saved runs, and quality gates to one recommended action.",
    image: productImages.dashboard,
    alt: "CorpusLab Mission Control dashboard with corpus health and quality actions",
  },
  {
    id: "corpus",
    label: "Corpus",
    title: "Inspect the material behind retrieval.",
    description:
      "Document profiles, extraction findings, chunk structure, and evidence signals stay visible and actionable.",
    image: productImages.sources,
    alt: "CorpusLab Corpus document library with source and chunk quality details",
  },
  {
    id: "retrieval",
    label: "Retrieval",
    title: "See why evidence ranked.",
    description:
      "Evidence summaries, citations, matched terms, and normalized scores make every result explainable.",
    image: productImages.retrieval,
    alt: "CorpusLab retrieval results with evidence summary and score bars",
  },
  {
    id: "quality",
    label: "Quality",
    title: "Measure changes before release.",
    description:
      "Compare retrieval modes, diagnose failed cases, and enforce deterministic release gates.",
    image: productImages.evals,
    alt: "CorpusLab Quality experiment with recall precision and failed cases",
  },
  {
    id: "reports",
    label: "Reports",
    title: "Share a diagnosis people can act on.",
    description:
      "Bring weak runs, corpus findings, CI failures, and evidence lineage into one review-ready surface.",
    image: productImages.reports,
    alt: "CorpusLab Reports view with failed gate and evidence diagnosis",
  },
];

export const enterpriseSignals = [
  { icon: ShieldCheck, label: "Private deployment" },
  { icon: BrainCircuit, label: "Local collectors" },
  { icon: GitCompareArrows, label: "CI quality gates" },
  { icon: Share2, label: "Workspace governance" },
] as const;
