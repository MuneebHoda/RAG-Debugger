use axum::{extract::State, Json};
use rag_debugger_core::{
    DocumentProfile, DocumentSummary, EmbeddingIndexRequest, EmbeddingStatus, EvidenceStrength,
    FailureLabel, OverviewAction, OverviewActionPriority, OverviewActivity, OverviewActivityKind,
    OverviewDocumentProfile, OverviewEvalExperimentSummary, OverviewEvalRunSummary, OverviewHealth,
    OverviewHealthStatus, OverviewIssue, OverviewMetric, OverviewPipelineStep, OverviewResponse,
    OverviewSeverity, OverviewStepStatus, OverviewTone, RetrievalEvalExperiment,
    RetrievalEvalGateStatus, RetrievalEvalRun, SourceSummary, TraceSummary,
};
use rag_debugger_rag::embedding::{EmbeddingProvider, LocalHashEmbeddingProvider};
use time::OffsetDateTime;

use crate::{error::ApiError, state::AppState};

pub async fn get_overview(
    State(state): State<AppState>,
) -> Result<Json<OverviewResponse>, ApiError> {
    let repository = state.repository().ok_or(ApiError::NotReady)?;
    let provider = LocalHashEmbeddingProvider::new(state.config().product.embedding.model.clone());

    let sources = repository.list_sources().await?;
    let embedding_status = repository
        .embedding_status(&EmbeddingIndexRequest::default(), &provider.model())
        .await?;
    let traces = repository.list_traces().await?;
    let eval_cases = repository.list_retrieval_eval_cases().await?;
    let latest_eval_run = repository.latest_retrieval_eval_run().await?;
    let eval_datasets = repository.list_retrieval_eval_datasets().await?;
    let latest_eval_experiment = repository.latest_retrieval_eval_experiment().await?;

    Ok(Json(build_overview(
        sources,
        embedding_status,
        traces,
        eval_cases.len() as u32,
        latest_eval_run,
        eval_datasets.len() as u32,
        latest_eval_experiment,
    )))
}

fn build_overview(
    sources: Vec<SourceSummary>,
    embedding_status: EmbeddingStatus,
    traces: Vec<TraceSummary>,
    eval_case_count: u32,
    latest_eval_run: Option<RetrievalEvalRun>,
    eval_dataset_count: u32,
    latest_eval_experiment: Option<RetrievalEvalExperiment>,
) -> OverviewResponse {
    let generated_at = OffsetDateTime::now_utc();
    let source_count = sources.len() as u32;
    let documents = sources
        .iter()
        .flat_map(|source| source.documents.iter())
        .collect::<Vec<_>>();
    let document_count = documents.len() as u32;
    let chunk_count = sources.iter().map(|source| source.chunk_count).sum::<u32>();
    let warning_count = documents
        .iter()
        .map(|document| document.document.warnings.len() as u32)
        .sum::<u32>();
    let weak_trace_count = traces
        .iter()
        .filter(|trace| {
            trace.evidence_strength == EvidenceStrength::Weak || !trace.failure_labels.is_empty()
        })
        .count() as u32;
    let failed_trace_count = traces
        .iter()
        .filter(|trace| {
            trace.failure_labels.iter().any(|label| {
                matches!(
                    label,
                    FailureLabel::MissingDocument | FailureLabel::MissingEmbeddingIndex
                )
            })
        })
        .count() as u32;
    let counts = OverviewCounts {
        source_count,
        document_count,
        chunk_count,
        warning_count,
        trace_count: traces.len() as u32,
        weak_trace_count,
        failed_trace_count,
        eval_case_count,
    };
    let latest_eval_summary = latest_eval_run
        .as_ref()
        .map(OverviewEvalRunSummary::from_eval_run);
    let latest_experiment_summary = latest_eval_experiment
        .as_ref()
        .map(OverviewEvalExperimentSummary::from_experiment);
    let issues = build_issues(
        document_count,
        warning_count,
        &embedding_status,
        weak_trace_count,
        failed_trace_count,
        eval_case_count,
        latest_experiment_summary.as_ref(),
    );
    let health = build_health(
        document_count,
        warning_count,
        &embedding_status,
        weak_trace_count,
        failed_trace_count,
        eval_case_count,
        latest_eval_summary.as_ref(),
    );
    let metrics = build_metrics(
        &counts,
        &embedding_status,
        eval_dataset_count,
        latest_eval_summary.as_ref(),
        latest_experiment_summary.as_ref(),
    );
    let pipeline = build_pipeline(
        document_count,
        chunk_count,
        &embedding_status,
        traces.len() as u32,
        eval_case_count,
    );
    let actions = build_actions(&health, &issues, traces.len() as u32);
    let document_mix = build_document_mix(&documents);
    let recent_activity = build_recent_activity(
        &sources,
        &traces,
        latest_eval_summary.as_ref(),
        generated_at,
    );

    OverviewResponse {
        generated_at,
        health,
        metrics,
        pipeline,
        issues,
        actions,
        recent_activity,
        document_mix,
        embedding_status,
        latest_eval_run: latest_eval_summary,
        latest_eval_experiment: latest_experiment_summary,
    }
}

struct OverviewCounts {
    source_count: u32,
    document_count: u32,
    chunk_count: u32,
    warning_count: u32,
    trace_count: u32,
    weak_trace_count: u32,
    failed_trace_count: u32,
    eval_case_count: u32,
}

fn build_health(
    document_count: u32,
    warning_count: u32,
    embedding_status: &EmbeddingStatus,
    weak_trace_count: u32,
    failed_trace_count: u32,
    eval_case_count: u32,
    latest_eval_run: Option<&OverviewEvalRunSummary>,
) -> OverviewHealth {
    let status = if document_count == 0 {
        OverviewHealthStatus::NeedsDocuments
    } else if embedding_status.missing_chunks > 0 || embedding_status.stale_chunks > 0 {
        OverviewHealthStatus::NeedsIndexing
    } else if eval_case_count == 0 {
        OverviewHealthStatus::NeedsEvalCoverage
    } else {
        OverviewHealthStatus::Ready
    };
    let primary_action = primary_action(status, weak_trace_count, warning_count);

    let score = if document_count == 0 {
        0
    } else {
        let mut penalty = 0u32;
        let embedding_gap = embedding_status.missing_chunks + embedding_status.stale_chunks;
        if embedding_status.total_chunks > 0 {
            penalty += ((embedding_gap as f32 / embedding_status.total_chunks as f32) * 30.0)
                .round() as u32;
        }
        penalty += (warning_count * 5).min(20);
        penalty += (weak_trace_count * 5 + failed_trace_count * 10).min(25);
        if eval_case_count == 0 {
            penalty += 15;
        }
        if let Some(run) = latest_eval_run {
            penalty += ((1.0 - run.pass_rate).max(0.0) * 15.0).round() as u32;
        }
        100u32.saturating_sub(penalty.min(100))
    };

    OverviewHealth {
        score,
        status,
        summary: health_summary(status, score),
        primary_action: Some(primary_action),
    }
}

fn primary_action(
    status: OverviewHealthStatus,
    weak_trace_count: u32,
    warning_count: u32,
) -> OverviewAction {
    match status {
        OverviewHealthStatus::NeedsDocuments => action(
            "ingest_documents",
            "Ingest documents",
            "Add the first corpus source so CorpusLab can extract, chunk, and index evidence.",
            "/app/sources",
            OverviewActionPriority::Primary,
        ),
        OverviewHealthStatus::NeedsIndexing => action(
            "index_embeddings",
            "Index embeddings",
            "Refresh local embeddings so vector and hybrid retrieval have semantic coverage.",
            "/app/retrieval",
            OverviewActionPriority::Primary,
        ),
        OverviewHealthStatus::NeedsEvalCoverage => action(
            "create_eval_coverage",
            "Create eval coverage",
            "Save expected evidence for important questions before changing retrieval settings.",
            "/app/evals",
            OverviewActionPriority::Primary,
        ),
        OverviewHealthStatus::Ready if weak_trace_count > 0 => action(
            "review_weak_traces",
            "Review weak traces",
            "Inspect failure labels and rerun retrieval modes for weak saved traces.",
            "/app/traces",
            OverviewActionPriority::Primary,
        ),
        OverviewHealthStatus::Ready if warning_count > 0 => action(
            "inspect_sources",
            "Inspect sources",
            "Review extraction warnings and chunk quality before sharing reports.",
            "/app/sources",
            OverviewActionPriority::Primary,
        ),
        OverviewHealthStatus::Ready => action(
            "run_retrieval",
            "Run retrieval",
            "Ask a question, inspect cited evidence, and save strong runs as traces.",
            "/app/retrieval",
            OverviewActionPriority::Primary,
        ),
    }
}

fn health_summary(status: OverviewHealthStatus, score: u32) -> String {
    match status {
        OverviewHealthStatus::Ready => {
            format!(
                "Corpus operations are ready for retrieval review with a {score}% health score."
            )
        }
        OverviewHealthStatus::NeedsIndexing => {
            "Embeddings need attention before hybrid and vector retrieval can be trusted."
                .to_owned()
        }
        OverviewHealthStatus::NeedsEvalCoverage => {
            "Add eval cases so retrieval changes can be measured over time.".to_owned()
        }
        OverviewHealthStatus::NeedsDocuments => {
            "Ingest documents to begin corpus extraction, chunking, indexing, and retrieval."
                .to_owned()
        }
    }
}

fn build_metrics(
    counts: &OverviewCounts,
    embedding_status: &EmbeddingStatus,
    eval_dataset_count: u32,
    latest_eval_run: Option<&OverviewEvalRunSummary>,
    latest_eval_experiment: Option<&OverviewEvalExperimentSummary>,
) -> Vec<OverviewMetric> {
    vec![
        metric(
            "sources",
            "Sources",
            counts.source_count,
            "connected corpora",
            OverviewTone::Neutral,
        ),
        metric(
            "documents",
            "Documents",
            counts.document_count,
            "indexed files",
            OverviewTone::Neutral,
        ),
        metric(
            "chunks",
            "Chunks",
            counts.chunk_count,
            "retrieval units",
            OverviewTone::Neutral,
        ),
        OverviewMetric {
            id: "embeddings".to_owned(),
            label: "Embeddings".to_owned(),
            value: format!(
                "{}/{}",
                embedding_status.indexed_chunks, embedding_status.total_chunks
            ),
            detail: format!(
                "{} missing · {} stale",
                embedding_status.missing_chunks, embedding_status.stale_chunks
            ),
            tone: if embedding_status.missing_chunks == 0 && embedding_status.stale_chunks == 0 {
                OverviewTone::Good
            } else {
                OverviewTone::Warning
            },
        },
        OverviewMetric {
            id: "traces".to_owned(),
            label: "Traces".to_owned(),
            value: counts.trace_count.to_string(),
            detail: format!(
                "{} weak · {} failed",
                counts.weak_trace_count, counts.failed_trace_count
            ),
            tone: if counts.weak_trace_count == 0 && counts.failed_trace_count == 0 {
                OverviewTone::Good
            } else {
                OverviewTone::Warning
            },
        },
        OverviewMetric {
            id: "evals".to_owned(),
            label: "Eval coverage".to_owned(),
            value: counts.eval_case_count.to_string(),
            detail: latest_eval_experiment
                .map(|experiment| {
                    format!(
                        "{eval_dataset_count} datasets · {:?} gate · {:.0}% recall",
                        experiment.gate_status,
                        experiment.average_recall_at_k * 100.0
                    )
                })
                .or_else(|| {
                    latest_eval_run
                        .map(|run| format!("latest pass rate {}%", (run.pass_rate * 100.0).round()))
                })
                .unwrap_or_else(|| format!("{eval_dataset_count} datasets · no experiment yet")),
            tone: if counts.eval_case_count == 0 {
                OverviewTone::Warning
            } else if latest_eval_experiment
                .is_some_and(|experiment| experiment.gate_status == RetrievalEvalGateStatus::Failed)
            {
                OverviewTone::Critical
            } else {
                OverviewTone::Good
            },
        },
        OverviewMetric {
            id: "warnings".to_owned(),
            label: "Warnings".to_owned(),
            value: counts.warning_count.to_string(),
            detail: "extraction and corpus quality".to_owned(),
            tone: if counts.warning_count == 0 {
                OverviewTone::Good
            } else {
                OverviewTone::Warning
            },
        },
    ]
}

fn build_pipeline(
    document_count: u32,
    chunk_count: u32,
    embedding_status: &EmbeddingStatus,
    trace_count: u32,
    eval_case_count: u32,
) -> Vec<OverviewPipelineStep> {
    vec![
        step(
            "ingest",
            "Ingest",
            if document_count == 0 {
                OverviewStepStatus::Blocked
            } else {
                OverviewStepStatus::Complete
            },
            document_count,
            "documents available",
            "/app/sources",
            "Open Sources",
        ),
        step(
            "chunk",
            "Chunk",
            if chunk_count == 0 {
                OverviewStepStatus::Blocked
            } else {
                OverviewStepStatus::Complete
            },
            chunk_count,
            "retrieval units",
            "/app/sources",
            "Inspect chunks",
        ),
        step(
            "embed",
            "Embed",
            if embedding_status.total_chunks == 0 {
                OverviewStepStatus::Pending
            } else if embedding_status.missing_chunks > 0 || embedding_status.stale_chunks > 0 {
                OverviewStepStatus::Warning
            } else {
                OverviewStepStatus::Complete
            },
            embedding_status.indexed_chunks,
            "chunks indexed",
            "/app/retrieval",
            "Index",
        ),
        step(
            "retrieve",
            "Retrieve",
            if chunk_count == 0 {
                OverviewStepStatus::Blocked
            } else if trace_count > 0 {
                OverviewStepStatus::Complete
            } else {
                OverviewStepStatus::Pending
            },
            trace_count,
            "saved evidence runs",
            "/app/retrieval",
            "Run query",
        ),
        step(
            "trace",
            "Trace",
            if trace_count == 0 {
                OverviewStepStatus::Pending
            } else {
                OverviewStepStatus::Complete
            },
            trace_count,
            "debugger timelines",
            "/app/traces",
            "Open traces",
        ),
        step(
            "eval",
            "Eval",
            if eval_case_count == 0 {
                OverviewStepStatus::Warning
            } else {
                OverviewStepStatus::Complete
            },
            eval_case_count,
            "coverage cases",
            "/app/evals",
            "Run evals",
        ),
        step(
            "report",
            "Report",
            if trace_count == 0 {
                OverviewStepStatus::Pending
            } else {
                OverviewStepStatus::Complete
            },
            trace_count,
            "trace-backed reports",
            "/app/reports",
            "Open reports",
        ),
    ]
}

fn build_issues(
    document_count: u32,
    warning_count: u32,
    embedding_status: &EmbeddingStatus,
    weak_trace_count: u32,
    failed_trace_count: u32,
    eval_case_count: u32,
    latest_eval_experiment: Option<&OverviewEvalExperimentSummary>,
) -> Vec<OverviewIssue> {
    let mut issues = Vec::new();
    if document_count == 0 {
        issues.push(issue(
            "no_documents",
            OverviewSeverity::Critical,
            "No documents ingested",
            "CorpusLab needs documents before extraction, chunking, retrieval, traces, or evals can run.",
            "/app/sources",
            "Ingest documents",
        ));
    }
    if embedding_status.missing_chunks > 0 {
        issues.push(issue(
            "missing_embeddings",
            OverviewSeverity::Warning,
            "Missing embeddings",
            &format!(
                "{} chunks need local embeddings.",
                embedding_status.missing_chunks
            ),
            "/app/retrieval",
            "Index embeddings",
        ));
    }
    if embedding_status.stale_chunks > 0 {
        issues.push(issue(
            "stale_embeddings",
            OverviewSeverity::Warning,
            "Stale embeddings",
            &format!(
                "{} chunks changed since their embeddings were indexed.",
                embedding_status.stale_chunks
            ),
            "/app/retrieval",
            "Refresh index",
        ));
    }
    if warning_count > 0 {
        issues.push(issue(
            "extraction_warnings",
            OverviewSeverity::Warning,
            "Extraction warnings",
            &format!("{warning_count} document warnings may affect evidence quality."),
            "/app/sources",
            "Inspect sources",
        ));
    }
    if failed_trace_count > 0 || weak_trace_count > 0 {
        issues.push(issue(
            "weak_traces",
            OverviewSeverity::Warning,
            "Weak trace evidence",
            &format!("{weak_trace_count} traces contain weak evidence or failure labels."),
            "/app/traces",
            "Debug traces",
        ));
    }
    if eval_case_count == 0 {
        issues.push(issue(
            "missing_eval_coverage",
            OverviewSeverity::Warning,
            "Missing eval coverage",
            "No saved eval cases exist to measure retrieval changes.",
            "/app/evals",
            "Create evals",
        ));
    }
    if let Some(experiment) = latest_eval_experiment {
        if experiment.gate_status == RetrievalEvalGateStatus::Failed {
            issues.push(issue(
                "failed_eval_gate",
                OverviewSeverity::Critical,
                "Eval gate failed",
                &format!(
                    "{} failures in the latest Eval Lab experiment.",
                    experiment.failure_count
                ),
                "/app/evals",
                "Open Eval Lab",
            ));
        }
    }

    issues
}

fn build_actions(
    health: &OverviewHealth,
    issues: &[OverviewIssue],
    trace_count: u32,
) -> Vec<OverviewAction> {
    let mut actions = health
        .primary_action
        .clone()
        .into_iter()
        .collect::<Vec<_>>();
    actions.extend(issues.iter().take(3).map(|issue| {
        action(
            &issue.id,
            &issue.action_label,
            &issue.detail,
            &issue.route,
            OverviewActionPriority::Secondary,
        )
    }));
    if trace_count > 0 && !actions.iter().any(|action| action.route == "/app/reports") {
        actions.push(action(
            "generate_report",
            "Prepare report",
            "Use saved traces and corpus health signals in a stakeholder-ready report.",
            "/app/reports",
            OverviewActionPriority::Secondary,
        ));
    }
    dedupe_actions(actions)
}

fn build_document_mix(documents: &[&DocumentSummary]) -> Vec<OverviewDocumentProfile> {
    let mut profiles: Vec<(DocumentProfile, u32)> = Vec::new();
    for document in documents {
        if let Some((_, count)) = profiles
            .iter_mut()
            .find(|(profile, _)| *profile == document.document.profile)
        {
            *count += 1;
        } else {
            profiles.push((document.document.profile, 1));
        }
    }
    profiles.sort_by_key(|profile| std::cmp::Reverse(profile.1));
    let total = documents.len() as f32;
    profiles
        .into_iter()
        .map(|(profile, count)| OverviewDocumentProfile {
            profile,
            count,
            percentage: if total == 0.0 {
                0.0
            } else {
                count as f32 / total
            },
        })
        .collect()
}

fn build_recent_activity(
    sources: &[SourceSummary],
    traces: &[TraceSummary],
    latest_eval_run: Option<&OverviewEvalRunSummary>,
    generated_at: OffsetDateTime,
) -> Vec<OverviewActivity> {
    let mut activity = Vec::new();
    for source in sources.iter().take(2) {
        activity.push(OverviewActivity {
            id: format!("source:{}", source.source.id.0),
            kind: OverviewActivityKind::Source,
            label: source.source.name.clone(),
            detail: format!(
                "{} documents · {} chunks",
                source.document_count, source.chunk_count
            ),
            route: "/app/sources".to_owned(),
            created_at: None,
        });
    }
    for document in sources
        .iter()
        .flat_map(|source| source.documents.iter())
        .take(2)
    {
        activity.push(OverviewActivity {
            id: format!("document:{}", document.document.id.0),
            kind: OverviewActivityKind::Document,
            label: document.document.path.clone(),
            detail: format!(
                "{} chunks · {:?}",
                document.chunk_count, document.document.profile
            ),
            route: "/app/sources".to_owned(),
            created_at: None,
        });
    }
    for trace in traces.iter().take(3) {
        activity.push(OverviewActivity {
            id: format!("trace:{}", trace.id.0),
            kind: OverviewActivityKind::Trace,
            label: trace.query.clone(),
            detail: format!(
                "{:?} evidence · {} ms",
                trace.evidence_strength, trace.latency_ms
            ),
            route: "/app/traces".to_owned(),
            created_at: Some(trace.created_at),
        });
    }
    if let Some(run) = latest_eval_run {
        activity.push(OverviewActivity {
            id: format!("eval:{}", run.id.0),
            kind: OverviewActivityKind::Eval,
            label: format!("{} eval run", retrieval_mode_label(run.retrieval_mode)),
            detail: format!(
                "{}/{} passed · recall {:.0}%",
                run.passed_count,
                run.case_count,
                run.average_recall_at_k * 100.0
            ),
            route: "/app/evals".to_owned(),
            created_at: Some(run.created_at),
        });
    }
    if activity.is_empty() {
        activity.push(OverviewActivity {
            id: "empty".to_owned(),
            kind: OverviewActivityKind::Source,
            label: "No activity yet".to_owned(),
            detail: "Ingest documents to start the workbench timeline.".to_owned(),
            route: "/app/sources".to_owned(),
            created_at: Some(generated_at),
        });
    }
    activity.truncate(7);
    activity
}

fn metric(id: &str, label: &str, value: u32, detail: &str, tone: OverviewTone) -> OverviewMetric {
    OverviewMetric {
        id: id.to_owned(),
        label: label.to_owned(),
        value: value.to_string(),
        detail: detail.to_owned(),
        tone,
    }
}

fn step(
    id: &str,
    label: &str,
    status: OverviewStepStatus,
    count: u32,
    detail: &str,
    route: &str,
    action_label: &str,
) -> OverviewPipelineStep {
    OverviewPipelineStep {
        id: id.to_owned(),
        label: label.to_owned(),
        status,
        count,
        detail: detail.to_owned(),
        route: route.to_owned(),
        action_label: action_label.to_owned(),
    }
}

fn issue(
    id: &str,
    severity: OverviewSeverity,
    title: &str,
    detail: &str,
    route: &str,
    action_label: &str,
) -> OverviewIssue {
    OverviewIssue {
        id: id.to_owned(),
        severity,
        title: title.to_owned(),
        detail: detail.to_owned(),
        route: route.to_owned(),
        action_label: action_label.to_owned(),
    }
}

fn action(
    id: &str,
    label: &str,
    detail: &str,
    route: &str,
    priority: OverviewActionPriority,
) -> OverviewAction {
    OverviewAction {
        id: id.to_owned(),
        label: label.to_owned(),
        detail: detail.to_owned(),
        route: route.to_owned(),
        priority,
    }
}

fn dedupe_actions(actions: Vec<OverviewAction>) -> Vec<OverviewAction> {
    let mut deduped = Vec::new();
    for action in actions {
        if !deduped
            .iter()
            .any(|current: &OverviewAction| current.route == action.route)
        {
            deduped.push(action);
        }
    }
    deduped.truncate(4);
    deduped
}

fn retrieval_mode_label(mode: rag_debugger_core::RetrievalMode) -> &'static str {
    match mode {
        rag_debugger_core::RetrievalMode::Lexical => "lexical",
        rag_debugger_core::RetrievalMode::Vector => "vector",
        rag_debugger_core::RetrievalMode::Hybrid => "hybrid",
    }
}

#[cfg(test)]
mod tests {
    use rag_debugger_core::{EmbeddingModelInfo, RetrievalMode};
    use uuid::Uuid;

    use super::*;

    #[test]
    fn empty_overview_needs_documents() {
        let overview = build_overview(
            Vec::new(),
            embedding_status(0, 0, 0),
            Vec::new(),
            0,
            None,
            0,
            None,
        );

        assert_eq!(overview.health.status, OverviewHealthStatus::NeedsDocuments);
        assert_eq!(overview.health.score, 0);
        assert_eq!(
            overview
                .health
                .primary_action
                .expect("primary action")
                .route,
            "/app/sources"
        );
    }

    #[test]
    fn overview_with_missing_embeddings_needs_indexing() {
        let mut source = test_source_summary();
        source.chunk_count = 4;
        let overview = build_overview(
            vec![source],
            embedding_status(4, 2, 0),
            Vec::new(),
            1,
            None,
            1,
            None,
        );

        assert_eq!(overview.health.status, OverviewHealthStatus::NeedsIndexing);
        assert_eq!(
            overview
                .health
                .primary_action
                .expect("primary action")
                .route,
            "/app/retrieval"
        );
    }

    #[test]
    fn health_ready_requires_documents_embeddings_and_evals() {
        let mut source = test_source_summary();
        source.chunk_count = 1;
        let overview = build_overview(
            vec![source],
            embedding_status(1, 0, 0),
            Vec::new(),
            1,
            Some(RetrievalEvalRun {
                id: rag_debugger_core::RetrievalEvalRunId(Uuid::now_v7()),
                retrieval_mode: RetrievalMode::Hybrid,
                case_count: 1,
                passed_count: 1,
                average_recall_at_k: 1.0,
                average_precision_at_k: 1.0,
                created_at: OffsetDateTime::now_utc(),
                results: Vec::new(),
            }),
            1,
            None,
        );

        assert_eq!(overview.health.status, OverviewHealthStatus::Ready);
        assert!(overview.health.score >= 90);
    }

    fn embedding_status(total: u32, missing: u32, stale: u32) -> EmbeddingStatus {
        EmbeddingStatus {
            model: EmbeddingModelInfo {
                provider: "local".to_owned(),
                model_name: "local-hash-v1".to_owned(),
                dimension: 384,
            },
            total_chunks: total,
            indexed_chunks: total.saturating_sub(missing + stale),
            missing_chunks: missing,
            stale_chunks: stale,
            last_indexed_at: None,
        }
    }

    fn test_source_summary() -> SourceSummary {
        let source_id = rag_debugger_core::SourceId(Uuid::now_v7());
        SourceSummary {
            source: rag_debugger_core::Source {
                id: source_id,
                project_id: rag_debugger_core::ProjectId(Uuid::now_v7()),
                name: "Docs".to_owned(),
                kind: rag_debugger_core::SourceKind::FileSet {
                    root_hint: "test".to_owned(),
                },
                sync_policy: rag_debugger_core::SourceSyncPolicy::Manual,
                chunking: rag_debugger_core::ChunkingConfig::default(),
            },
            document_count: 1,
            chunk_count: 1,
            documents: vec![rag_debugger_core::DocumentSummary {
                document: rag_debugger_core::Document {
                    id: rag_debugger_core::DocumentId(Uuid::now_v7()),
                    source_id,
                    path: "guide.md".to_owned(),
                    mime_type: Some("text/markdown".to_owned()),
                    checksum: "abc".to_owned(),
                    byte_size: 20,
                    profile: DocumentProfile::TechnicalDocs,
                    extraction_quality: rag_debugger_core::ExtractionQuality::High,
                    warnings: Vec::new(),
                },
                chunk_count: 1,
            }],
        }
    }
}
