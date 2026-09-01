use std::collections::BTreeMap;

use rag_debugger_core::{
    ProductConfig, RetrievalEvalCaseProvenance, RetrievalEvalChunkSetProvenance,
    RetrievalEvalChunkingProvenance, RetrievalEvalCompatibility,
    RetrievalEvalCompatibilityClassification, RetrievalEvalCompatibilityReason,
    RetrievalEvalCompatibilityReasonCode, RetrievalEvalCorpusProvenance, RetrievalEvalDataset,
    RetrievalEvalDatasetProvenance, RetrievalEvalDocumentProvenance,
    RetrievalEvalEmbeddingProvenance, RetrievalEvalExperiment, RetrievalEvalExperimentProvenance,
    RetrievalEvalFilterProvenance, RetrievalEvalProvenanceFieldClass,
    RetrievalEvalProvenanceIdentity, RetrievalEvalProvenanceInformation,
    RetrievalEvalRetrievalProvenance, RetrievalEvalScoringProvenance,
    RetrievalEvalSourceChunkingProvenance, RetrievalMode, SearchableChunk, SourceSummary,
    WorkspaceId, RETRIEVAL_EVAL_PROVENANCE_SCHEMA_VERSION,
};
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProvenanceError {
    #[error("provenance serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub struct ExperimentCorpusSnapshot<'a> {
    pub sources: &'a [SourceSummary],
    pub candidates: &'a [SearchableChunk],
}

pub fn build_experiment_provenance(
    workspace_id: WorkspaceId,
    dataset: &RetrievalEvalDataset,
    modes: &[RetrievalMode],
    top_k: u32,
    product: &ProductConfig,
    corpus_snapshot: ExperimentCorpusSnapshot<'_>,
    informational: RetrievalEvalProvenanceInformation,
) -> Result<RetrievalEvalExperimentProvenance, ProvenanceError> {
    let ExperimentCorpusSnapshot {
        sources,
        candidates,
    } = corpus_snapshot;
    let mut project_ids = sources
        .iter()
        .map(|summary| summary.source.project_id)
        .collect::<Vec<_>>();
    project_ids.sort_by_key(|id| id.0);
    project_ids.dedup();

    let mut source_ids = sources
        .iter()
        .map(|summary| summary.source.id)
        .collect::<Vec<_>>();
    source_ids.sort_by_key(|id| id.0);
    source_ids.dedup();

    let mut documents = sources
        .iter()
        .flat_map(|summary| &summary.documents)
        .map(|summary| RetrievalEvalDocumentProvenance {
            document_id: summary.document.id,
            source_id: summary.document.source_id,
            checksum: summary.document.checksum.clone(),
            path_fingerprint: digest(summary.document.path.as_bytes()),
        })
        .collect::<Vec<_>>();
    documents.sort_by_key(|document| document.document_id.0);
    documents.dedup_by_key(|document| document.document_id);

    let mut chunking_sources = sources
        .iter()
        .map(|summary| {
            let mut config = summary.source.chunking;
            config.strategy = config.strategy.normalized();
            RetrievalEvalSourceChunkingProvenance {
                source_id: summary.source.id,
                config,
            }
        })
        .collect::<Vec<_>>();
    chunking_sources.sort_by_key(|source| source.source_id.0);
    chunking_sources.dedup_by_key(|source| source.source_id);

    let mut normalized_modes = modes.to_vec();
    normalized_modes.sort_by_key(|mode| mode_code(*mode));
    normalized_modes.dedup();

    let dataset_identity = dataset_identity(dataset);
    let chunk_identity = chunk_identity(candidates);
    let embedding_identity = embedding_identity(candidates);
    let document_set_fingerprint = fingerprint(&documents)?;
    let chunking_fingerprint = fingerprint(&chunking_sources)?;
    let chunk_set_fingerprint = fingerprint(&chunk_identity)?;
    let embedding_index_fingerprint = fingerprint(&embedding_identity)?;
    let embedding_model = &product.embedding.model;
    let indexed_chunk_count = candidates
        .iter()
        .filter(|candidate| embedding_is_current(candidate, embedding_model))
        .count() as u32;
    let missing_chunk_count = candidates
        .iter()
        .filter(|candidate| candidate.embedding.is_none())
        .count() as u32;
    let stale_chunk_count = candidates
        .iter()
        .filter(|candidate| {
            candidate
                .embedding
                .as_ref()
                .is_some_and(|_| !embedding_is_current(candidate, embedding_model))
        })
        .count() as u32;
    let mut runtime_flags = BTreeMap::new();
    runtime_flags.insert(
        "low_score_margin_ratio".to_owned(),
        product.debugger.low_score_margin_ratio.to_string(),
    );
    runtime_flags.insert(
        "max_top_k".to_owned(),
        product.retrieval.max_top_k.to_string(),
    );

    let identity = RetrievalEvalProvenanceIdentity {
        workspace_id,
        project_ids,
        dataset: RetrievalEvalDatasetProvenance {
            dataset_id: dataset.id,
            revision_fingerprint: fingerprint(&dataset_identity)?,
            case_count: dataset.cases.len() as u32,
        },
        corpus: RetrievalEvalCorpusProvenance {
            source_ids,
            document_count: documents.len() as u32,
            document_set_fingerprint,
            documents,
        },
        chunking: RetrievalEvalChunkingProvenance {
            fingerprint: chunking_fingerprint,
            sources: chunking_sources,
        },
        chunk_set: RetrievalEvalChunkSetProvenance {
            fingerprint: chunk_set_fingerprint,
            chunk_count: candidates.len() as u32,
        },
        embedding: RetrievalEvalEmbeddingProvenance {
            provider: embedding_model.provider.clone(),
            model_name: embedding_model.model_name.clone(),
            dimension: embedding_model.dimension,
            index_fingerprint: embedding_index_fingerprint,
            indexed_chunk_count,
            missing_chunk_count,
            stale_chunk_count,
        },
        retrieval: RetrievalEvalRetrievalProvenance {
            modes: normalized_modes,
            top_k,
            scoring: RetrievalEvalScoringProvenance {
                weights: product.retrieval.weights.clone(),
                min_evidence_score: product.retrieval.min_evidence_score,
                min_semantic_similarity: product.retrieval.min_semantic_similarity,
                answer_citation_limit: product.retrieval.answer_citation_limit,
                answerability: product.retrieval.answerability.clone(),
            },
            filters: RetrievalEvalFilterProvenance::default(),
            runtime_flags,
        },
    };

    Ok(RetrievalEvalExperimentProvenance {
        schema_version: RETRIEVAL_EVAL_PROVENANCE_SCHEMA_VERSION,
        fingerprint: fingerprint(&identity)?,
        identity,
        informational,
    })
}

pub fn experiment_compatibility(
    current: &RetrievalEvalExperiment,
    baseline: Option<&RetrievalEvalExperiment>,
    intentional_cross_configuration: bool,
) -> RetrievalEvalCompatibility {
    let Some(baseline) = baseline else {
        return RetrievalEvalCompatibility {
            classification: RetrievalEvalCompatibilityClassification::LegacyUnknown,
            intentional_cross_configuration: false,
            changed_fields: Vec::new(),
            reasons: vec![reason(
                RetrievalEvalCompatibilityReasonCode::NoBaseline,
                None,
                RetrievalEvalProvenanceFieldClass::IdentityDefining,
                false,
                true,
                "No baseline experiment is available.",
            )],
        };
    };
    let (Some(current_provenance), Some(baseline_provenance)) =
        (&current.provenance, &baseline.provenance)
    else {
        let missing = if current.provenance.is_none() {
            "current"
        } else {
            "baseline"
        };
        return RetrievalEvalCompatibility {
            classification: RetrievalEvalCompatibilityClassification::LegacyUnknown,
            intentional_cross_configuration,
            changed_fields: vec![format!("{missing}.provenance")],
            reasons: vec![reason(
                RetrievalEvalCompatibilityReasonCode::MissingProvenance,
                Some(format!("{missing}.provenance")),
                RetrievalEvalProvenanceFieldClass::IdentityDefining,
                true,
                true,
                "A legacy experiment has no immutable provenance snapshot.",
            )],
        };
    };

    let mut changed_fields = Vec::new();
    let mut reasons = Vec::new();
    identity_changes(
        current_provenance,
        baseline_provenance,
        &mut changed_fields,
        &mut reasons,
    );
    informational_changes(
        current_provenance,
        baseline_provenance,
        &mut changed_fields,
        &mut reasons,
    );
    let identity_changed = reasons
        .iter()
        .any(|reason| reason.code == RetrievalEvalCompatibilityReasonCode::IdentityChanged);
    let classification = if identity_changed && intentional_cross_configuration {
        reasons.push(reason(
            RetrievalEvalCompatibilityReasonCode::ExplicitCrossConfiguration,
            None,
            RetrievalEvalProvenanceFieldClass::IdentityDefining,
            false,
            false,
            "This cross-configuration baseline was selected explicitly; interpret metric deltas directionally.",
        ));
        RetrievalEvalCompatibilityClassification::PartiallyCompatible
    } else if identity_changed {
        RetrievalEvalCompatibilityClassification::Incompatible
    } else {
        RetrievalEvalCompatibilityClassification::Compatible
    };

    RetrievalEvalCompatibility {
        classification,
        intentional_cross_configuration,
        changed_fields,
        reasons,
    }
}

fn identity_changes(
    current: &RetrievalEvalExperimentProvenance,
    baseline: &RetrievalEvalExperimentProvenance,
    changed_fields: &mut Vec<String>,
    reasons: &mut Vec<RetrievalEvalCompatibilityReason>,
) {
    compare_identity(
        current.schema_version != baseline.schema_version,
        "provenance.schema_version",
        false,
        changed_fields,
        reasons,
    );
    let current = &current.identity;
    let baseline = &baseline.identity;
    compare_identity(
        current.workspace_id != baseline.workspace_id,
        "workspace_id",
        false,
        changed_fields,
        reasons,
    );
    compare_identity(
        current.project_ids != baseline.project_ids,
        "project_ids",
        false,
        changed_fields,
        reasons,
    );
    compare_identity(
        current.dataset.dataset_id != baseline.dataset.dataset_id,
        "dataset.id",
        false,
        changed_fields,
        reasons,
    );
    compare_identity(
        current.dataset.revision_fingerprint != baseline.dataset.revision_fingerprint,
        "dataset.revision_fingerprint",
        true,
        changed_fields,
        reasons,
    );
    compare_identity(
        current.corpus.source_ids != baseline.corpus.source_ids,
        "corpus.source_ids",
        false,
        changed_fields,
        reasons,
    );
    compare_identity(
        current.corpus.document_set_fingerprint != baseline.corpus.document_set_fingerprint,
        "corpus.document_set_fingerprint",
        true,
        changed_fields,
        reasons,
    );
    compare_identity(
        current.chunking.fingerprint != baseline.chunking.fingerprint,
        "chunking.fingerprint",
        false,
        changed_fields,
        reasons,
    );
    compare_identity(
        current.chunk_set.fingerprint != baseline.chunk_set.fingerprint,
        "chunk_set.fingerprint",
        true,
        changed_fields,
        reasons,
    );
    compare_identity(
        current.embedding.provider != baseline.embedding.provider,
        "embedding.provider",
        false,
        changed_fields,
        reasons,
    );
    compare_identity(
        current.embedding.model_name != baseline.embedding.model_name,
        "embedding.model",
        false,
        changed_fields,
        reasons,
    );
    compare_identity(
        current.embedding.dimension != baseline.embedding.dimension,
        "embedding.dimension",
        false,
        changed_fields,
        reasons,
    );
    compare_identity(
        current.embedding.index_fingerprint != baseline.embedding.index_fingerprint,
        "embedding.index_fingerprint",
        true,
        changed_fields,
        reasons,
    );
    compare_identity(
        current.retrieval.modes != baseline.retrieval.modes,
        "retrieval.modes",
        false,
        changed_fields,
        reasons,
    );
    compare_identity(
        current.retrieval.top_k != baseline.retrieval.top_k,
        "retrieval.top_k",
        false,
        changed_fields,
        reasons,
    );
    compare_identity(
        current.retrieval.scoring != baseline.retrieval.scoring,
        "retrieval.scoring",
        false,
        changed_fields,
        reasons,
    );
    compare_identity(
        current.retrieval.filters != baseline.retrieval.filters,
        "retrieval.filters",
        false,
        changed_fields,
        reasons,
    );
    compare_identity(
        current.retrieval.runtime_flags != baseline.retrieval.runtime_flags,
        "retrieval.runtime_flags",
        false,
        changed_fields,
        reasons,
    );
}

fn informational_changes(
    current: &RetrievalEvalExperimentProvenance,
    baseline: &RetrievalEvalExperimentProvenance,
    changed_fields: &mut Vec<String>,
    reasons: &mut Vec<RetrievalEvalCompatibilityReason>,
) {
    let current = &current.informational;
    let baseline = &baseline.informational;
    for (changed, field) in [
        (
            current.application_version != baseline.application_version,
            "build.application_version",
        ),
        (
            current.deployment_mode != baseline.deployment_mode,
            "build.deployment_mode",
        ),
        (
            current.runtime_environment != baseline.runtime_environment,
            "build.runtime_environment",
        ),
        (
            current.storage_backend != baseline.storage_backend,
            "build.storage_backend",
        ),
        (current.branch != baseline.branch, "ci.branch"),
        (current.commit_sha != baseline.commit_sha, "ci.commit_sha"),
        (current.base_ref != baseline.base_ref, "ci.base_ref"),
        (current.head_ref != baseline.head_ref, "ci.head_ref"),
        (current.labels != baseline.labels, "ci.labels"),
    ] {
        if changed {
            changed_fields.push(field.to_owned());
            reasons.push(reason(
                RetrievalEvalCompatibilityReasonCode::InformationalChanged,
                Some(field.to_owned()),
                RetrievalEvalProvenanceFieldClass::Informational,
                false,
                true,
                "Informational build or CI metadata changed without changing retrieval identity.",
            ));
        }
    }
}

fn compare_identity(
    changed: bool,
    field: &str,
    privacy_sensitive: bool,
    changed_fields: &mut Vec<String>,
    reasons: &mut Vec<RetrievalEvalCompatibilityReason>,
) {
    if changed {
        changed_fields.push(field.to_owned());
        reasons.push(reason(
            RetrievalEvalCompatibilityReasonCode::IdentityChanged,
            Some(field.to_owned()),
            RetrievalEvalProvenanceFieldClass::IdentityDefining,
            privacy_sensitive,
            false,
            "An identity-defining experiment input changed.",
        ));
    }
}

fn reason(
    code: RetrievalEvalCompatibilityReasonCode,
    field: Option<String>,
    field_class: RetrievalEvalProvenanceFieldClass,
    privacy_sensitive: bool,
    legacy_optional: bool,
    message: &str,
) -> RetrievalEvalCompatibilityReason {
    RetrievalEvalCompatibilityReason {
        code,
        field,
        field_class,
        privacy_sensitive,
        legacy_optional,
        message: message.to_owned(),
    }
}

#[derive(Serialize)]
struct DatasetCaseIdentity {
    id: String,
    query_fingerprint: String,
    expected_chunk_ids: Vec<String>,
    expected_document_ids: Vec<String>,
    provenance: Option<RetrievalEvalCaseProvenance>,
}

fn dataset_identity(dataset: &RetrievalEvalDataset) -> Vec<DatasetCaseIdentity> {
    let mut cases = dataset
        .cases
        .iter()
        .map(|case| {
            let mut expected_chunk_ids = case
                .expected_chunk_ids
                .iter()
                .map(|id| id.0.to_string())
                .collect::<Vec<_>>();
            expected_chunk_ids.sort();
            let mut expected_document_ids = case
                .expected_document_ids
                .iter()
                .map(|id| id.0.to_string())
                .collect::<Vec<_>>();
            expected_document_ids.sort();
            DatasetCaseIdentity {
                id: case.id.0.to_string(),
                query_fingerprint: digest(case.query.as_bytes()),
                expected_chunk_ids,
                expected_document_ids,
                provenance: case.provenance.clone(),
            }
        })
        .collect::<Vec<_>>();
    cases.sort_by(|left, right| left.id.cmp(&right.id));
    cases
}

#[derive(Serialize)]
struct ChunkIdentity {
    id: String,
    source_id: String,
    document_id: String,
    ordinal: u32,
    checksum: String,
    text_fingerprint: String,
    strategy: String,
    section_title_fingerprint: Option<String>,
    split_reason: String,
    quality_flags: Vec<String>,
    is_duplicate: bool,
    text_density: f32,
    evidence_score_hint: f32,
    token_count: u32,
    byte_start: u64,
    byte_end: u64,
}

fn chunk_identity(candidates: &[SearchableChunk]) -> Vec<ChunkIdentity> {
    let mut chunks = candidates
        .iter()
        .map(|candidate| {
            let mut quality_flags = candidate
                .chunk
                .quality_flags
                .iter()
                .map(|flag| chunk_quality_flag_code(*flag).to_owned())
                .collect::<Vec<_>>();
            quality_flags.sort();
            quality_flags.dedup();
            ChunkIdentity {
                id: candidate.chunk.id.0.to_string(),
                source_id: candidate.chunk.source_id.0.to_string(),
                document_id: candidate.chunk.document_id.0.to_string(),
                ordinal: candidate.chunk.ordinal,
                checksum: candidate.chunk.checksum.clone(),
                text_fingerprint: digest(candidate.chunk.text.as_bytes()),
                strategy: chunking_strategy_code(candidate.chunk.strategy).to_owned(),
                section_title_fingerprint: candidate
                    .chunk
                    .section_title
                    .as_ref()
                    .map(|title| digest(title.as_bytes())),
                split_reason: chunk_split_reason_code(candidate.chunk.split_reason).to_owned(),
                quality_flags,
                is_duplicate: candidate.chunk.is_duplicate,
                text_density: candidate.chunk.text_density,
                evidence_score_hint: candidate.chunk.evidence_score_hint,
                token_count: candidate.chunk.token_count,
                byte_start: candidate.chunk.byte_range.start,
                byte_end: candidate.chunk.byte_range.end,
            }
        })
        .collect::<Vec<_>>();
    chunks.sort_by(|left, right| left.id.cmp(&right.id));
    chunks
}

#[derive(Serialize)]
struct EmbeddingIdentity {
    chunk_id: String,
    chunk_checksum: String,
    embedding_chunk_checksum: Option<String>,
    provider: Option<String>,
    model_name: Option<String>,
    dimension: Option<u32>,
    vector_fingerprint: Option<String>,
}

fn embedding_identity(candidates: &[SearchableChunk]) -> Vec<EmbeddingIdentity> {
    let mut embeddings = candidates
        .iter()
        .map(|candidate| {
            let embedding = candidate.embedding.as_ref();
            EmbeddingIdentity {
                chunk_id: candidate.chunk.id.0.to_string(),
                chunk_checksum: candidate.chunk.checksum.clone(),
                embedding_chunk_checksum: embedding.map(|value| value.chunk_checksum.clone()),
                provider: embedding.map(|value| value.model.provider.clone()),
                model_name: embedding.map(|value| value.model.model_name.clone()),
                dimension: embedding.map(|value| value.model.dimension),
                vector_fingerprint: embedding.map(|value| {
                    let mut hasher = Sha256::new();
                    for component in &value.vector {
                        hasher.update(component.to_bits().to_be_bytes());
                    }
                    hex::encode(hasher.finalize())
                }),
            }
        })
        .collect::<Vec<_>>();
    embeddings.sort_by(|left, right| left.chunk_id.cmp(&right.chunk_id));
    embeddings
}

fn embedding_is_current(
    candidate: &SearchableChunk,
    model: &rag_debugger_core::EmbeddingModelInfo,
) -> bool {
    candidate.embedding.as_ref().is_some_and(|embedding| {
        embedding.chunk_checksum == candidate.chunk.checksum && &embedding.model == model
    })
}

fn fingerprint(value: &impl Serialize) -> Result<String, ProvenanceError> {
    fingerprint_value(serde_json::to_value(value)?)
}

fn fingerprint_value(value: Value) -> Result<String, ProvenanceError> {
    let canonical = canonicalize(value);
    Ok(digest(&serde_json::to_vec(&canonical)?))
}

fn canonicalize(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut keys = object.into_iter().collect::<Vec<_>>();
            keys.sort_by(|left, right| left.0.cmp(&right.0));
            Value::Object(
                keys.into_iter()
                    .map(|(key, value)| (key, canonicalize(value)))
                    .collect::<Map<_, _>>(),
            )
        }
        Value::Array(values) => Value::Array(values.into_iter().map(canonicalize).collect()),
        value => value,
    }
}

fn digest(value: &[u8]) -> String {
    hex::encode(Sha256::digest(value))
}

fn mode_code(mode: RetrievalMode) -> &'static str {
    match mode {
        RetrievalMode::Lexical => "lexical",
        RetrievalMode::Vector => "vector",
        RetrievalMode::Hybrid => "hybrid",
    }
}

fn chunking_strategy_code(strategy: rag_debugger_core::ChunkingStrategy) -> &'static str {
    match strategy.normalized() {
        rag_debugger_core::ChunkingStrategy::Structured
        | rag_debugger_core::ChunkingStrategy::SmartSections => "structured",
        rag_debugger_core::ChunkingStrategy::Whitespace => "whitespace",
    }
}

fn chunk_split_reason_code(reason: rag_debugger_core::ChunkSplitReason) -> &'static str {
    match reason {
        rag_debugger_core::ChunkSplitReason::SectionBoundary => "section_boundary",
        rag_debugger_core::ChunkSplitReason::TokenLimit => "token_limit",
        rag_debugger_core::ChunkSplitReason::DocumentEnd => "document_end",
        rag_debugger_core::ChunkSplitReason::FallbackWhitespace => "fallback_whitespace",
    }
}

fn chunk_quality_flag_code(flag: rag_debugger_core::ChunkQualityFlag) -> &'static str {
    match flag {
        rag_debugger_core::ChunkQualityFlag::HeadingOnly => "heading_only",
        rag_debugger_core::ChunkQualityFlag::TooShort => "too_short",
        rag_debugger_core::ChunkQualityFlag::TooLong => "too_long",
        rag_debugger_core::ChunkQualityFlag::Duplicate => "duplicate",
        rag_debugger_core::ChunkQualityFlag::LowTextDensity => "low_text_density",
        rag_debugger_core::ChunkQualityFlag::ExtractionWarning => "extraction_warning",
        rag_debugger_core::ChunkQualityFlag::GoodEvidenceCandidate => "good_evidence_candidate",
    }
}

#[cfg(test)]
mod tests {
    use rag_debugger_core::{
        ByteRange, Chunk, ChunkEmbedding, ChunkId, ChunkQualityFlag, ChunkSplitReason,
        ChunkingConfig, ChunkingStrategy, Document, DocumentId, DocumentProfile, DocumentSummary,
        ExtractionQuality, ProjectId, RetrievalEvalCase, RetrievalEvalCaseId,
        RetrievalEvalDatasetId, Source, SourceId, SourceKind, SourceSyncPolicy, TraceId,
        TraceIngestionPrivacyMode, TraceIngestionSource,
    };
    use serde_json::{json, Map};
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn canonical_fingerprint_is_stable_and_order_independent() {
        let (workspace_id, dataset, product, mut sources, mut candidates) = fixture();
        let first = build_experiment_provenance(
            workspace_id,
            &dataset,
            &[RetrievalMode::Vector, RetrievalMode::Hybrid],
            5,
            &product,
            ExperimentCorpusSnapshot {
                sources: &sources,
                candidates: &candidates,
            },
            information(),
        )
        .unwrap();
        sources.reverse();
        candidates.reverse();
        let second = build_experiment_provenance(
            workspace_id,
            &dataset,
            &[
                RetrievalMode::Hybrid,
                RetrievalMode::Vector,
                RetrievalMode::Hybrid,
            ],
            5,
            &product,
            ExperimentCorpusSnapshot {
                sources: &sources,
                candidates: &candidates,
            },
            information(),
        )
        .unwrap();

        assert_eq!(first.fingerprint, second.fingerprint);
        assert_eq!(first.identity, second.identity);

        let mut left = Map::new();
        left.insert("z".to_owned(), json!({"b": 2, "a": 1}));
        left.insert("a".to_owned(), json!(true));
        let mut right = Map::new();
        right.insert("a".to_owned(), json!(true));
        right.insert("z".to_owned(), json!({"a": 1, "b": 2}));
        assert_eq!(
            fingerprint_value(Value::Object(left)).unwrap(),
            fingerprint_value(Value::Object(right)).unwrap()
        );
    }

    #[test]
    fn corpus_chunking_and_chunk_set_changes_change_identity() {
        let (workspace_id, dataset, product, sources, candidates) = fixture();
        let original = build(&workspace_id, &dataset, &product, &sources, &candidates);

        let mut changed_document = sources.clone();
        changed_document[0].documents[0].document.checksum = "document-v2".to_owned();
        let document = build(
            &workspace_id,
            &dataset,
            &product,
            &changed_document,
            &candidates,
        );
        assert_ne!(
            original.identity.corpus.document_set_fingerprint,
            document.identity.corpus.document_set_fingerprint
        );

        let mut changed_path = sources.clone();
        changed_path[0].documents[0].document.path = "renamed-private-path.md".to_owned();
        let path = build(
            &workspace_id,
            &dataset,
            &product,
            &changed_path,
            &candidates,
        );
        assert_ne!(
            original.identity.corpus.document_set_fingerprint,
            path.identity.corpus.document_set_fingerprint
        );

        let mut changed_chunking = sources.clone();
        changed_chunking[0].source.chunking.target_tokens += 1;
        let chunking = build(
            &workspace_id,
            &dataset,
            &product,
            &changed_chunking,
            &candidates,
        );
        assert_ne!(
            original.identity.chunking.fingerprint,
            chunking.identity.chunking.fingerprint
        );

        let mut changed_chunks = candidates.clone();
        changed_chunks[0].chunk.checksum = "chunk-v2".to_owned();
        let chunks = build(&workspace_id, &dataset, &product, &sources, &changed_chunks);
        assert_ne!(
            original.identity.chunk_set.fingerprint,
            chunks.identity.chunk_set.fingerprint
        );

        for mutate in [
            |candidate: &mut SearchableChunk| candidate.chunk.text.push_str(" changed"),
            |candidate: &mut SearchableChunk| {
                candidate.chunk.section_title = Some("Changed private section".to_owned());
            },
            |candidate: &mut SearchableChunk| {
                candidate
                    .chunk
                    .quality_flags
                    .push(ChunkQualityFlag::TooShort);
            },
            |candidate: &mut SearchableChunk| candidate.chunk.evidence_score_hint -= 0.1,
        ] {
            let mut changed = candidates.clone();
            mutate(&mut changed[0]);
            assert_ne!(
                original.identity.chunk_set.fingerprint,
                build(&workspace_id, &dataset, &product, &sources, &changed)
                    .identity
                    .chunk_set
                    .fingerprint
            );
        }
    }

    #[test]
    fn embedding_configuration_and_index_changes_change_identity() {
        let (workspace_id, dataset, product, sources, candidates) = fixture();
        let original = build(&workspace_id, &dataset, &product, &sources, &candidates);

        for mutate in [
            |product: &mut ProductConfig| product.embedding.model.provider.push_str("-v2"),
            |product: &mut ProductConfig| product.embedding.model.model_name.push_str("-v2"),
            |product: &mut ProductConfig| product.embedding.model.dimension += 1,
        ] {
            let mut changed = product.clone();
            mutate(&mut changed);
            assert_ne!(
                original.fingerprint,
                build(&workspace_id, &dataset, &changed, &sources, &candidates).fingerprint
            );
        }

        let mut changed_index = candidates.clone();
        changed_index[0].embedding.as_mut().unwrap().vector[0] += 0.1;
        assert_ne!(
            original.identity.embedding.index_fingerprint,
            build(&workspace_id, &dataset, &product, &sources, &changed_index)
                .identity
                .embedding
                .index_fingerprint
        );

        let mut changed_embedding_checksum = candidates.clone();
        changed_embedding_checksum[0]
            .embedding
            .as_mut()
            .unwrap()
            .chunk_checksum
            .push_str("-stale");
        assert_ne!(
            original.identity.embedding.index_fingerprint,
            build(
                &workspace_id,
                &dataset,
                &product,
                &sources,
                &changed_embedding_checksum
            )
            .identity
            .embedding
            .index_fingerprint
        );

        let mut reindexed = candidates.clone();
        reindexed[0].embedding.as_mut().unwrap().indexed_at += time::Duration::hours(1);
        assert_eq!(
            original.identity.embedding.index_fingerprint,
            build(&workspace_id, &dataset, &product, &sources, &reindexed)
                .identity
                .embedding
                .index_fingerprint
        );

        let mut missing_index = candidates.clone();
        missing_index[0].embedding = None;
        let missing = build(&workspace_id, &dataset, &product, &sources, &missing_index);
        assert_eq!(missing.identity.embedding.indexed_chunk_count, 0);
        assert_eq!(missing.identity.embedding.missing_chunk_count, 1);
        assert_eq!(missing.identity.embedding.stale_chunk_count, 0);

        let mut stale_index = candidates.clone();
        stale_index[0]
            .embedding
            .as_mut()
            .unwrap()
            .model
            .model_name
            .push_str("-old");
        let stale = build(&workspace_id, &dataset, &product, &sources, &stale_index);
        assert_eq!(stale.identity.embedding.indexed_chunk_count, 0);
        assert_eq!(stale.identity.embedding.missing_chunk_count, 0);
        assert_eq!(stale.identity.embedding.stale_chunk_count, 1);
    }

    #[test]
    fn ranking_scoring_filters_and_dataset_revision_change_identity() {
        let (workspace_id, dataset, product, sources, candidates) = fixture();
        let original = build(&workspace_id, &dataset, &product, &sources, &candidates);

        let mut changed_scoring = product.clone();
        changed_scoring.retrieval.weights.lexical += 0.1;
        assert_ne!(
            original.fingerprint,
            build(
                &workspace_id,
                &dataset,
                &changed_scoring,
                &sources,
                &candidates
            )
            .fingerprint
        );

        let mut changed_limit = product.clone();
        changed_limit.retrieval.max_top_k += 1;
        assert_ne!(
            original.fingerprint,
            build(
                &workspace_id,
                &dataset,
                &changed_limit,
                &sources,
                &candidates
            )
            .fingerprint
        );

        let mut changed_dataset = dataset.clone();
        changed_dataset.cases[0].query.push_str(" changed");
        assert_ne!(
            original.identity.dataset.revision_fingerprint,
            build(
                &workspace_id,
                &changed_dataset,
                &product,
                &sources,
                &candidates
            )
            .identity
            .dataset
            .revision_fingerprint
        );

        let mut changed_case_provenance = dataset.clone();
        changed_case_provenance.cases[0].provenance = Some(RetrievalEvalCaseProvenance {
            source_trace_id: TraceId(id(30)),
            source: TraceIngestionSource::Native,
            privacy_mode: TraceIngestionPrivacyMode::MetadataOnly,
        });
        assert_ne!(
            original.identity.dataset.revision_fingerprint,
            build(
                &workspace_id,
                &changed_case_provenance,
                &product,
                &sources,
                &candidates
            )
            .identity
            .dataset
            .revision_fingerprint
        );

        let mut changed_filters = original.clone();
        changed_filters.identity.retrieval.filters.source_ids = vec![sources[0].source.id];
        changed_filters.fingerprint = fingerprint(&changed_filters.identity).unwrap();
        assert_ne!(original.fingerprint, changed_filters.fingerprint);
    }

    #[test]
    fn compatibility_is_strict_legacy_safe_and_supports_explicit_cross_config() {
        let (workspace_id, dataset, product, sources, candidates) = fixture();
        let provenance = build(&workspace_id, &dataset, &product, &sources, &candidates);
        let current = experiment(dataset.id, Some(provenance.clone()));
        let baseline = experiment(dataset.id, Some(provenance));
        assert_eq!(
            experiment_compatibility(&current, Some(&baseline), false).classification,
            RetrievalEvalCompatibilityClassification::Compatible
        );

        let mut changed = baseline.clone();
        changed
            .provenance
            .as_mut()
            .unwrap()
            .identity
            .retrieval
            .top_k += 1;
        let automatic = experiment_compatibility(&current, Some(&changed), false);
        assert_eq!(
            automatic.classification,
            RetrievalEvalCompatibilityClassification::Incompatible
        );
        assert!(automatic
            .changed_fields
            .contains(&"retrieval.top_k".to_owned()));
        let explicit = experiment_compatibility(&current, Some(&changed), true);
        assert_eq!(
            explicit.classification,
            RetrievalEvalCompatibilityClassification::PartiallyCompatible
        );
        assert!(explicit.intentional_cross_configuration);

        let legacy = experiment(dataset.id, None);
        assert_eq!(
            experiment_compatibility(&current, Some(&legacy), true).classification,
            RetrievalEvalCompatibilityClassification::LegacyUnknown
        );
    }

    #[test]
    fn serialized_provenance_never_contains_raw_document_or_query_content() {
        let (workspace_id, dataset, product, sources, candidates) = fixture();
        let encoded = serde_json::to_string(&build(
            &workspace_id,
            &dataset,
            &product,
            &sources,
            &candidates,
        ))
        .unwrap();
        assert!(!encoded.contains("PRIVATE query text"));
        assert!(!encoded.contains("PRIVATE document body"));
        assert!(!encoded.contains("private/path.md"));
        assert!(encoded.contains("document-v1"));
    }

    fn build(
        workspace_id: &WorkspaceId,
        dataset: &RetrievalEvalDataset,
        product: &ProductConfig,
        sources: &[SourceSummary],
        candidates: &[SearchableChunk],
    ) -> RetrievalEvalExperimentProvenance {
        build_experiment_provenance(
            *workspace_id,
            dataset,
            &[RetrievalMode::Hybrid],
            5,
            product,
            ExperimentCorpusSnapshot {
                sources,
                candidates,
            },
            information(),
        )
        .unwrap()
    }

    fn fixture() -> (
        WorkspaceId,
        RetrievalEvalDataset,
        ProductConfig,
        Vec<SourceSummary>,
        Vec<SearchableChunk>,
    ) {
        let workspace_id = WorkspaceId(id(1));
        let project_id = ProjectId(id(2));
        let source_id = SourceId(id(3));
        let document_id = DocumentId(id(4));
        let chunk_id = ChunkId(id(5));
        let source = Source {
            id: source_id,
            project_id,
            name: "Private source".to_owned(),
            kind: SourceKind::FileSet {
                root_hint: "private/root".to_owned(),
            },
            sync_policy: SourceSyncPolicy::Manual,
            chunking: ChunkingConfig::default(),
        };
        let document = Document {
            id: document_id,
            source_id,
            path: "private/path.md".to_owned(),
            mime_type: Some("text/markdown".to_owned()),
            checksum: "document-v1".to_owned(),
            byte_size: 21,
            profile: DocumentProfile::General,
            extraction_quality: ExtractionQuality::High,
            warnings: Vec::new(),
        };
        let chunk = Chunk {
            id: chunk_id,
            source_id,
            document_id,
            ordinal: 0,
            text: "PRIVATE document body".to_owned(),
            token_count: 3,
            byte_range: ByteRange { start: 0, end: 21 },
            checksum: "chunk-v1".to_owned(),
            strategy: ChunkingStrategy::Structured,
            section_title: None,
            split_reason: ChunkSplitReason::DocumentEnd,
            quality_flags: Vec::new(),
            is_duplicate: false,
            text_density: 1.0,
            evidence_score_hint: 0.8,
        };
        let embedding = ChunkEmbedding {
            chunk_id,
            chunk_checksum: chunk.checksum.clone(),
            model: ProductConfig::default().embedding.model,
            vector: vec![0.1, 0.2],
            indexed_at: OffsetDateTime::from_unix_timestamp(10).unwrap(),
        };
        let dataset = RetrievalEvalDataset {
            id: RetrievalEvalDatasetId(id(6)),
            name: "Private dataset".to_owned(),
            description: None,
            cases: vec![RetrievalEvalCase {
                id: RetrievalEvalCaseId(id(7)),
                case_key: "private-case".to_owned(),
                name: "Private case".to_owned(),
                query: "PRIVATE query text".to_owned(),
                top_k: 5,
                expected_chunk_ids: vec![chunk_id],
                expected_document_ids: vec![document_id],
                notes: None,
                provenance: None,
                created_at: OffsetDateTime::from_unix_timestamp(1).unwrap(),
            }],
            created_at: OffsetDateTime::from_unix_timestamp(1).unwrap(),
            updated_at: OffsetDateTime::from_unix_timestamp(1).unwrap(),
        };
        let sources = vec![SourceSummary {
            source: source.clone(),
            document_count: 1,
            chunk_count: 1,
            documents: vec![DocumentSummary {
                document: document.clone(),
                chunk_count: 1,
            }],
        }];
        let candidates = vec![SearchableChunk {
            source,
            document,
            chunk,
            embedding: Some(embedding),
        }];
        (
            workspace_id,
            dataset,
            ProductConfig::default(),
            sources,
            candidates,
        )
    }

    fn information() -> RetrievalEvalProvenanceInformation {
        RetrievalEvalProvenanceInformation {
            application_version: "test".to_owned(),
            ..Default::default()
        }
    }

    fn experiment(
        dataset_id: RetrievalEvalDatasetId,
        provenance: Option<RetrievalEvalExperimentProvenance>,
    ) -> RetrievalEvalExperiment {
        serde_json::from_value(json!({
            "id": id(20),
            "dataset_id": dataset_id,
            "dataset_name": "dataset",
            "name": "experiment",
            "modes": ["hybrid"],
            "top_k": 5,
            "config_snapshot": {
                "top_k": 5,
                "scoring_weights": rag_debugger_core::RetrievalWeights::default(),
                "embedding_model": rag_debugger_core::EmbeddingModelInfo::default(),
                "dataset_case_count": 1
            },
            "provenance": provenance,
            "mode_results": [],
            "comparison": {"best_mode": null, "mode_count": 0, "recall_delta": 0.0, "precision_delta": 0.0, "latency_delta_ms": 0, "summary": "none"},
            "gate": {"status": "passed", "average_recall_at_k": 1.0, "weak_evidence_rate": 0.0, "critical_failure_count": 0, "recall_threshold": 0.8, "weak_evidence_limit": 0.2, "reasons": []},
            "failures": [],
            "created_at": "2026-01-01T00:00:00Z"
        }))
        .unwrap()
    }

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }
}
