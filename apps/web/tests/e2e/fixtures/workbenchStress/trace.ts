import type { RetrievalQueryResponse } from "../../../../src/lib/api/retrieval";
import type { Trace, TraceSummary } from "../../../../src/lib/api/traces";

import {
  stressChunk,
  stressDocumentRecord,
  stressSourceRecord,
} from "./corpus";
import { stressIds, stressValues, unbrokenToken } from "./shared";

export const stressDiagnosis = {
  outcome: "failing",
  summary: `Candidate evidence failed deterministic answerability checks. ${unbrokenToken}`,
  primary_issue: {
    code: "answerability_gap",
    severity: "critical",
    title: "Candidates do not directly support an answer",
    summary: "Retrieved candidates lack sufficient body-text support.",
    evidence_refs: ["E1"],
  },
  failures: [
    {
      code: "answerability_gap",
      severity: "critical",
      title: "Candidates do not directly support an answer",
      summary: "Retrieved candidates lack sufficient body-text support.",
      evidence_refs: ["E1"],
    },
    {
      code: "duplicate_evidence",
      severity: "warning",
      title: "Duplicate evidence reduced result diversity",
      summary: "The same normalized evidence occupied multiple ranks.",
      evidence_refs: ["E1"],
    },
    {
      code: "low_score_margin",
      severity: "warning",
      title: "Top evidence is not clearly separated",
      summary: "The leading scores are nearly tied.",
      evidence_refs: ["E1"],
    },
  ],
  score_explanations: [
    {
      evidence_ref: "E1",
      chunk_id: stressIds.chunk,
      rank: 1,
      final_score: 0.817_234,
      score_delta_from_previous: null,
      score_delta_to_next: 0.000_013,
      dominant_signal: "semantic",
      score_breakdown: {
        semantic: 1.342_815,
        lexical: 1.309_992,
        phrase: 0.197_551,
        section: 0.188_441,
        path: 0.129_887,
        metadata: 0.099_113,
      },
      normalized_score_breakdown: {
        semantic: 1,
        lexical: 0.975_556,
        phrase: 0.147_115,
        section: 0.140_329,
        path: 0.096_726,
        metadata: 0.073_808,
      },
      summary: `Semantic scoring led a dense six-signal breakdown. ${unbrokenToken}`,
    },
  ],
  recommendations: [
    {
      code: "require_supported_body_evidence",
      priority: "critical",
      area: "corpus_coverage",
      title: "Require direct body-text support",
      rationale: "Candidates were retrieved, but none passed answerability.",
      action: "Add the missing source or retrieve a chunk with direct support.",
      failure_codes: ["answerability_gap"],
      evidence_refs: ["E1"],
    },
    {
      code: "remove_duplicate_chunks",
      priority: "high",
      area: "chunking",
      title: "Remove duplicate chunks",
      rationale: "Duplicate evidence narrowed the candidate set.",
      action: "Deduplicate chunks and rerun the comparison.",
      failure_codes: ["duplicate_evidence"],
      evidence_refs: ["E1"],
    },
  ],
} satisfies NonNullable<RetrievalQueryResponse["diagnosis"]>;

export const stressRetrieval = {
  run: {
    id: stressIds.run,
    query: stressValues.query,
    top_k: 25,
    retrieval_mode: "hybrid",
    latency_ms: 12_345,
    created_at: "2026-07-04T08:00:00Z",
  },
  answer: {
    status: "insufficient_evidence",
    text: "Candidates were retrieved, but none directly support an answer.",
    citations: [],
  },
  hits: [
    {
      rank: 1,
      score: 0.817_234,
      chunk: stressChunk,
      document: stressDocumentRecord,
      source: stressSourceRecord,
      matched_terms: [
        { term: "recovery", count: 12 },
        { term: "evidence", count: 9 },
        { term: "ranking", count: 7 },
      ],
      score_breakdown: stressDiagnosis.score_explanations[0].score_breakdown,
      normalized_score_breakdown:
        stressDiagnosis.score_explanations[0].normalized_score_breakdown,
      snippet: stressValues.snippet,
      citation: {
        label: "[1]",
        chunk_id: stressIds.chunk,
        document_id: stressIds.document,
        document_path: stressValues.documentPath,
        chunk_ordinal: 27,
        section_title: stressChunk.section_title,
        checksum_prefix: unbrokenToken.slice(0, 12),
        snippet: stressValues.snippet,
      },
      quality_flags: [
        "duplicate",
        "too_short",
        "weak_evidence",
        "semantic_match",
        "section_only_match",
      ],
      evidence_strength: "weak",
      duplicate_count: 8,
      answer_support: {
        status: "unsupported",
        reason: "semantic_only_match",
        matched_body_term_count: 1,
        query_term_count: 14,
        body_term_coverage: 0.071_429,
      },
    },
  ],
  embedding_status: {
    readiness: "partial",
    required: true,
    model: {
      provider: "local",
      model_name: `local-hash-v1-${unbrokenToken}`,
      dimension: 384,
    },
    total_chunks: 100_000,
    indexed_chunks: 99_997,
    missing_chunks: 2,
    stale_chunks: 1,
  },
  diagnosis: stressDiagnosis,
} satisfies RetrievalQueryResponse;

export const stressTrace = {
  id: stressIds.trace,
  project_id: stressIds.project,
  input: stressValues.query,
  output: null,
  started_at: "2026-07-04T08:00:00Z",
  completed_at: "2026-07-04T08:00:12Z",
  failure_labels: [
    "missing_document",
    "bad_chunking",
    "bad_embedding",
    "bad_ranking",
    "weak_evidence",
    "duplicate_evidence",
    "heading_only_evidence",
  ],
  source_run_id: stressIds.run,
  summary: stressDiagnosis.summary,
  status: "failed",
  evidence_strength: "weak",
  spans: [
    {
      id: "018f7a2a-6e2e-7000-a000-000000004811",
      kind: "retrieval",
      title: `Dense retrieval ranking ${unbrokenToken}`,
      description: stressDiagnosis.summary,
      started_at: "2026-07-04T08:00:00Z",
      completed_at: "2026-07-04T08:00:12Z",
      latency_ms: 12_345,
      status: "failed",
      detail: {
        type: "retrieval",
        hit_count: 25,
        top_score: 0.817_234,
        embedding_readiness: "partial",
      },
    },
  ],
  retrieval: stressRetrieval,
  reruns: [],
  diagnosis: stressDiagnosis,
} satisfies Trace;

export const stressTraceSummary = {
  id: stressIds.trace,
  query: stressValues.query,
  retrieval_mode: "hybrid",
  latency_ms: 12_345,
  evidence_strength: "weak",
  failure_labels: stressTrace.failure_labels,
  span_count: stressTrace.spans.length,
  rerun_count: 0,
  created_at: stressTrace.started_at,
} satisfies TraceSummary;
