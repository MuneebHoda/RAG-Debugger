import type { DebugReport } from "../../../../src/lib/api/reports";

import { stressIds, stressValues, unbrokenToken } from "./shared";
import { stressDiagnosis } from "./trace";

export const stressReport = {
  id: stressIds.report,
  workspace_id: "018f7a2a-6e2e-7000-a000-000000000903",
  project_id: stressIds.project,
  title: stressValues.reportTitle,
  subject: "",
  source: { type: "trace", trace_id: stressIds.trace },
  privacy_mode: "metadata_only",
  executive_summary: stressDiagnosis.summary,
  context: {
    retrieval_mode: "hybrid",
    top_k: "25",
    embedding_model: `local-hash-v1-${unbrokenToken}`,
  },
  findings: stressDiagnosis.failures.map((failure) => ({
    code: failure.code,
    severity: failure.severity,
    title: failure.title,
    summary: failure.summary,
    failure_labels: [failure.code],
    evidence_refs: failure.evidence_refs,
  })),
  recommendations: stressDiagnosis.recommendations.map((recommendation) => ({
    code: recommendation.code,
    priority: recommendation.priority,
    area: recommendation.area,
    title: recommendation.title,
    rationale: recommendation.rationale,
    action: recommendation.action,
    finding_codes: recommendation.failure_codes,
    evidence_refs: recommendation.evidence_refs,
  })),
  evidence: [
    {
      label: "E1",
      role: "retrieved",
      source_id: stressIds.source,
      document_id: stressIds.document,
      chunk_id: stressIds.chunk,
      rank: 1,
      document_path: null,
      section_title: null,
      checksum_prefix: unbrokenToken,
      citation_label: null,
      snippet: null,
      evidence_strength: "weak",
      chunk_quality_flags: ["duplicate", "too_long", "low_text_density"],
      retrieval_quality_flags: [
        "semantic_match",
        "section_only_match",
        "weak_evidence",
      ],
    },
  ],
  diagnosis: stressDiagnosis,
  created_at: "2026-07-04T08:00:13Z",
} satisfies DebugReport;
