mod ci;
mod evidence;
mod experiment;
mod markdown;
mod privacy;
mod recommendations;
mod trace;

use rag_debugger_core::{
    DebugReportId, DebugReportPrivacyMode, ProjectId, RetrievalEmbeddingReadiness, RetrievalMode,
    WorkspaceId,
};
use thiserror::Error;
use time::OffsetDateTime;

pub use ci::build_ci_eval_debug_report;
pub use experiment::build_eval_experiment_debug_report;
pub use markdown::{render_debug_report_markdown, ReportExportError};
pub use trace::build_trace_debug_report;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DebugReportBuildContext {
    pub report_id: DebugReportId,
    pub workspace_id: WorkspaceId,
    pub project_id: ProjectId,
    pub privacy_mode: DebugReportPrivacyMode,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ReportBuildError {
    #[error("invalid report source: {0}")]
    InvalidSource(&'static str),
}

fn retrieval_mode_label(mode: RetrievalMode) -> &'static str {
    match mode {
        RetrievalMode::Lexical => "lexical",
        RetrievalMode::Vector => "vector",
        RetrievalMode::Hybrid => "hybrid",
    }
}

fn embedding_readiness_label(readiness: RetrievalEmbeddingReadiness) -> &'static str {
    match readiness {
        RetrievalEmbeddingReadiness::NotRequired => "not_required",
        RetrievalEmbeddingReadiness::Missing => "missing",
        RetrievalEmbeddingReadiness::Partial => "partial",
        RetrievalEmbeddingReadiness::Ready => "ready",
    }
}
