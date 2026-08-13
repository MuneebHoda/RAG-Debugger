use std::collections::{BTreeMap, HashMap, HashSet};

use opentelemetry_proto::tonic::{
    collector::trace::v1::ExportTraceServiceRequest,
    common::v1::{any_value::Value, InstrumentationScope, KeyValue},
    resource::v1::Resource,
    trace::v1::{span::SpanKind, status::StatusCode, Span},
};
use prost::Message;
use rag_debugger_core::*;
use rag_debugger_rag::imported_trace::build_native_trace;
use time::OffsetDateTime;

pub const MAX_RESOURCE_SPANS: usize = 16;
pub const MAX_SCOPE_SPANS: usize = 64;
pub const MAX_TRACES: usize = 32;
pub const MAX_TOTAL_SPANS: usize = 512;
pub const MAX_SPANS_PER_TRACE: usize = 256;

pub struct MappedBatch {
    pub traces: Vec<Trace>,
    pub rejected_spans: i64,
    pub rejection_reasons: BTreeMap<&'static str, u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct OtlpMapError {
    pub code: &'static str,
}

#[derive(Clone)]
enum Primitive {
    String(String),
    Int(i64),
    Double(f64),
    Bool(bool),
}

struct TraceGroup {
    spans: BTreeMap<String, Span>,
    total_spans: u32,
    conflicting: bool,
    invalid: Option<&'static str>,
    context: OtlpContext,
    context_conflict: bool,
}

#[derive(Clone, Default)]
struct OtlpContext {
    service_name: Option<String>,
    service_version: Option<String>,
    deployment_environment: Option<String>,
    instrumentation_scope_name: Option<String>,
    instrumentation_scope_version: Option<String>,
}

pub fn decode_and_map(
    body: &[u8],
    project: &Project,
    retrieval_config: &RetrievalConfig,
    debugger_config: &DebuggerConfig,
) -> Result<MappedBatch, OtlpMapError> {
    let request = ExportTraceServiceRequest::decode(body).map_err(|_| OtlpMapError {
        code: "malformed_protobuf",
    })?;
    if request.resource_spans.len() > MAX_RESOURCE_SPANS {
        return Err(OtlpMapError {
            code: "resource_span_limit_exceeded",
        });
    }
    let scope_count = request
        .resource_spans
        .iter()
        .map(|resource| resource.scope_spans.len())
        .sum::<usize>();
    if scope_count > MAX_SCOPE_SPANS {
        return Err(OtlpMapError {
            code: "scope_span_limit_exceeded",
        });
    }
    let total_spans = request
        .resource_spans
        .iter()
        .flat_map(|resource| &resource.scope_spans)
        .map(|scope| scope.spans.len())
        .sum::<usize>();
    if total_spans > MAX_TOTAL_SPANS {
        return Err(OtlpMapError {
            code: "span_limit_exceeded",
        });
    }

    let mut groups = HashMap::<String, TraceGroup>::new();
    let mut rejected_without_trace = 0_i64;
    for resource in request.resource_spans {
        if resource
            .resource
            .as_ref()
            .is_some_and(|value| value.attributes.len() > 32)
        {
            return Err(OtlpMapError {
                code: "resource_attribute_limit_exceeded",
            });
        }
        let resource_context = map_resource_context(resource.resource.as_ref());
        for scope in resource.scope_spans {
            if scope
                .scope
                .as_ref()
                .is_some_and(|value| value.attributes.len() > 16)
            {
                return Err(OtlpMapError {
                    code: "scope_attribute_limit_exceeded",
                });
            }
            let context = resource_context
                .as_ref()
                .map_err(|code| *code)
                .and_then(|value| map_scope_context(value.clone(), scope.scope.as_ref()));
            for span in scope.spans {
                if !valid_id(&span.trace_id, 16) {
                    rejected_without_trace += 1;
                    continue;
                }
                let trace_id = hex::encode(&span.trace_id);
                let group = groups.entry(trace_id).or_insert_with(|| TraceGroup {
                    spans: BTreeMap::new(),
                    total_spans: 0,
                    conflicting: false,
                    invalid: None,
                    context: OtlpContext::default(),
                    context_conflict: false,
                });
                match &context {
                    Ok(context) => {
                        group.context_conflict |= group.context.merge(context);
                    }
                    Err(code) => group.invalid = Some(code),
                }
                group.total_spans = group.total_spans.saturating_add(1);
                if !valid_id(&span.span_id, 8) {
                    group.invalid = Some("invalid_span_id");
                    continue;
                }
                let span_id = hex::encode(&span.span_id);
                if let Some(existing) = group.spans.get(&span_id) {
                    if existing != &span {
                        group.conflicting = true;
                    }
                } else {
                    group.spans.insert(span_id, span);
                }
            }
        }
    }
    if groups.len() > MAX_TRACES {
        return Err(OtlpMapError {
            code: "trace_limit_exceeded",
        });
    }

    let mut traces = Vec::new();
    let mut rejected_spans = rejected_without_trace;
    let mut reasons = BTreeMap::new();
    if rejected_without_trace > 0 {
        reasons.insert("invalid_trace_id", rejected_without_trace as u32);
    }
    for (trace_id, group) in groups {
        let group_count = i64::from(group.total_spans);
        if group.total_spans as usize > MAX_SPANS_PER_TRACE {
            rejected_spans += group_count;
            *reasons.entry("trace_span_limit_exceeded").or_default() += 1;
            continue;
        }
        if let Some(code) = group.invalid {
            rejected_spans += group_count;
            *reasons.entry(code).or_default() += 1;
            continue;
        }
        if group.conflicting {
            rejected_spans += group_count;
            *reasons.entry("conflicting_duplicate_span").or_default() += 1;
            continue;
        }
        match map_trace(
            project,
            trace_id,
            group.spans.into_values().collect(),
            group.context,
            group.context_conflict,
            retrieval_config,
            debugger_config,
        ) {
            Ok(trace) => {
                traces.push(trace);
            }
            Err(code) => {
                rejected_spans += group_count;
                *reasons.entry(code).or_default() += 1;
            }
        }
    }
    Ok(MappedBatch {
        traces,
        rejected_spans,
        rejection_reasons: reasons,
    })
}

fn map_trace(
    project: &Project,
    trace_id: String,
    spans: Vec<Span>,
    context: OtlpContext,
    context_conflict: bool,
    retrieval_config: &RetrievalConfig,
    debugger_config: &DebuggerConfig,
) -> Result<Trace, &'static str> {
    if spans.is_empty() {
        return Err("empty_trace");
    }
    let mut imported_spans = Vec::with_capacity(spans.len());
    let mut evidence = BTreeMap::<String, ImportedEvidence>::new();
    let mut model_config = TraceIngestionModelConfig {
        provider: None,
        generation_model: None,
        embedding_model: None,
        ranker: None,
        configuration_label: None,
    };
    let mut started_at = None;
    let mut completed_at = None;
    let mut has_failure = false;
    let mut failure_labels = std::collections::BTreeSet::new();
    let mut mapped_retrieval_mode = None;
    let mut top_k = None;
    let mut evaluation_passed = None;
    let mut evaluation_label = None;
    let span_ids = spans
        .iter()
        .map(|span| hex::encode(&span.span_id))
        .collect::<HashSet<_>>();
    let mut orphaned = false;
    let mut stripped_content = false;
    let mut unsupported_operation = false;
    let mut unsupported_span_kind = false;

    for span in spans {
        if span.attributes.len() > 64 || span.events.len() > 32 || span.links.len() > 32 {
            return Err("span_collection_limit_exceeded");
        }
        if span.name.len() > 256 || span.name.chars().any(char::is_control) {
            return Err("invalid_span_name");
        }
        if span.events.iter().any(|event| event.attributes.len() > 32)
            || span.links.iter().any(|link| link.attributes.len() > 32)
        {
            return Err("event_or_link_attribute_limit_exceeded");
        }
        for event in &span.events {
            if event.name.len() > 256 || event.name.chars().any(char::is_control) {
                return Err("invalid_event_name");
            }
            primitive_attributes(&event.attributes)?;
        }
        for link in &span.links {
            primitive_attributes(&link.attributes)?;
        }
        let attributes = primitive_attributes(&span.attributes)?;
        validate_label_attributes(&attributes)?;
        validate_failure_label_attribute(&span.attributes)?;
        stripped_content |= span
            .attributes
            .iter()
            .any(|attribute| is_sensitive_key(&attribute.key));
        let start = timestamp(span.start_time_unix_nano).ok_or("invalid_span_timestamp")?;
        let end = timestamp(span.end_time_unix_nano).ok_or("invalid_span_timestamp")?;
        if end < start {
            return Err("invalid_span_timestamp");
        }
        started_at = Some(started_at.map_or(start, |current: OffsetDateTime| current.min(start)));
        completed_at = Some(completed_at.map_or(end, |current: OffsetDateTime| current.max(end)));
        let operation = operation(&attributes);
        let kind = match SpanKind::try_from(span.kind) {
            Ok(SpanKind::Internal) => ImportedSpanKind::Internal,
            Ok(SpanKind::Server) => ImportedSpanKind::Server,
            Ok(SpanKind::Client) => ImportedSpanKind::Client,
            Ok(SpanKind::Producer) => ImportedSpanKind::Producer,
            Ok(SpanKind::Consumer) => ImportedSpanKind::Consumer,
            Ok(SpanKind::Unspecified) => ImportedSpanKind::Unspecified,
            Err(_) => {
                unsupported_span_kind = true;
                ImportedSpanKind::Unspecified
            }
        };
        unsupported_operation |= operation == ImportedSpanOperation::Other;
        mapped_retrieval_mode = mapped_retrieval_mode.or_else(|| {
            string_attr(&attributes, &["corpuslab.retrieval_mode"])
                .as_deref()
                .and_then(parse_retrieval_mode)
        });
        if top_k.is_none() {
            top_k = checked_u32_attr(&attributes, &["corpuslab.top_k"], 1, 25)?;
        }
        evaluation_passed =
            evaluation_passed.or_else(|| bool_attr(&attributes, &["corpuslab.evaluation_passed"]));
        evaluation_label =
            evaluation_label.or_else(|| string_attr(&attributes, &["corpuslab.evaluation_label"]));
        failure_labels.extend(failure_labels_attr(&span.attributes));
        let status = if span
            .status
            .as_ref()
            .is_some_and(|value| value.code == StatusCode::Error as i32)
        {
            has_failure = true;
            ImportedSpanStatus::Failed
        } else {
            ImportedSpanStatus::Succeeded
        };
        let parent_span_id = if span.parent_span_id.is_empty() {
            None
        } else if valid_id(&span.parent_span_id, 8) {
            let parent = hex::encode(&span.parent_span_id);
            orphaned |= !span_ids.contains(&parent);
            Some(parent)
        } else {
            return Err("invalid_parent_span_id");
        };
        let provider = string_attr(
            &attributes,
            &["corpuslab.provider", "gen_ai.provider.name", "llm.provider"],
        );
        let model = string_attr(
            &attributes,
            &[
                "corpuslab.generation_model",
                "gen_ai.response.model",
                "gen_ai.request.model",
                "llm.model_name",
            ],
        );
        model_config.provider = model_config.provider.or_else(|| provider.clone());
        if operation == ImportedSpanOperation::Generation {
            model_config.generation_model = model_config.generation_model.or_else(|| model.clone());
        }
        if operation == ImportedSpanOperation::Embedding {
            model_config.embedding_model = model_config.embedding_model.or_else(|| {
                string_attr(
                    &attributes,
                    &["corpuslab.embedding_model", "embedding.model_name"],
                )
            });
        }
        if operation == ImportedSpanOperation::Reranking {
            model_config.ranker = model_config
                .ranker
                .or_else(|| string_attr(&attributes, &["corpuslab.ranker", "reranker.model_name"]));
        }
        model_config.configuration_label = model_config
            .configuration_label
            .or_else(|| string_attr(&attributes, &["corpuslab.configuration_label"]));

        if let Some(external_chunk_id) = checked_id_attr(
            &attributes,
            &["corpuslab.evidence.external_chunk_id", "document.id"],
        )? {
            let rank = checked_u32_attr(&attributes, &["corpuslab.evidence.rank"], 1, 100)?
                .unwrap_or(evidence.len() as u32 + 1);
            let score =
                checked_score_attr(&attributes, &["corpuslab.evidence.score", "document.score"])?
                    .unwrap_or(0.0);
            evidence.insert(
                external_chunk_id.clone(),
                ImportedEvidence {
                    external_chunk_id,
                    document_label: None,
                    rank,
                    score,
                    lexical_score: checked_score_attr(
                        &attributes,
                        &["corpuslab.evidence.lexical_score"],
                    )?,
                    semantic_score: checked_score_attr(
                        &attributes,
                        &["corpuslab.evidence.semantic_score"],
                    )?,
                    citation_label: string_attr(&attributes, &["corpuslab.evidence.citation"]),
                    snippet: None,
                    answer_support_status: string_attr(
                        &attributes,
                        &["corpuslab.evidence.answer_support_status"],
                    )
                    .as_deref()
                    .map_or(AnswerSupportStatus::Unassessed, parse_support_status),
                    answer_support_reason: string_attr(
                        &attributes,
                        &["corpuslab.evidence.answer_support_reason"],
                    )
                    .as_deref()
                    .map_or(AnswerSupportReason::Unassessed, parse_support_reason),
                },
            );
        }
        imported_spans.push(ImportedSpan {
            external_span_id: hex::encode(&span.span_id),
            parent_span_id,
            operation,
            kind,
            name: operation.canonical_label().to_owned(),
            started_at: start,
            completed_at: Some(end),
            latency_ms: ((end - start).whole_nanoseconds().max(0) / 1_000_000) as u64,
            status,
            provider,
            model,
            input_tokens: checked_u32_attr(
                &attributes,
                &["gen_ai.usage.input_tokens", "llm.token_count.prompt"],
                0,
                u32::MAX,
            )?,
            output_tokens: checked_u32_attr(
                &attributes,
                &["gen_ai.usage.output_tokens", "llm.token_count.completion"],
                0,
                u32::MAX,
            )?,
            error_type: string_attr(&attributes, &["error.type"]),
        });
    }
    let started_at = started_at.ok_or("empty_trace")?;
    let completed_at = completed_at.ok_or("empty_trace")?;
    let request = NativeTraceIngestionRequest {
        schema_version: TRACE_INGESTION_SCHEMA_VERSION.to_owned(),
        project_id: project.id,
        external_trace_id: trace_id,
        privacy_mode: TraceIngestionPrivacyMode::MetadataOnly,
        query: None,
        prompt: None,
        answer: None,
        retrieval_mode: mapped_retrieval_mode.or_else(|| retrieval_mode(&imported_spans)),
        top_k,
        model_config: Some(model_config),
        retrieved_evidence: evidence.into_values().collect(),
        spans: imported_spans,
        failure_labels: failure_labels.into_iter().collect(),
        evaluation_passed,
        evaluation_label,
        started_at: Some(started_at),
        completed_at: Some(completed_at),
        latency_ms: None,
        status: Some(if has_failure {
            TraceStatus::Failed
        } else {
            TraceStatus::Completed
        }),
    };
    let mut trace = build_native_trace(project, request, retrieval_config, debugger_config)
        .map_err(|_| "mapped_trace_validation_failed")?;
    if let Some(metadata) = trace.ingestion.as_mut() {
        metadata.source = TraceIngestionSource::OtlpHttp;
        metadata.mapping_status = TraceMappingStatus::PartiallyMapped;
        metadata.service_name = context.service_name;
        metadata.service_version = context.service_version;
        metadata.deployment_environment = context.deployment_environment;
        metadata.instrumentation_scope_name = context.instrumentation_scope_name;
        metadata.instrumentation_scope_version = context.instrumentation_scope_version;
        if context_conflict {
            metadata
                .limitations
                .push("resource_or_scope_metadata_conflict".to_owned());
        }
        if orphaned {
            metadata.limitations.push("orphan_parent_span".to_owned());
        }
        if stripped_content {
            metadata
                .limitations
                .push("content_attributes_removed".to_owned());
        }
        if unsupported_operation {
            metadata
                .limitations
                .push("unsupported_operation".to_owned());
        }
        if unsupported_span_kind {
            metadata
                .limitations
                .push("unsupported_span_kind".to_owned());
        }
        metadata.limitations.sort();
        metadata.limitations.dedup();
    }
    Ok(trace)
}

impl OtlpContext {
    fn merge(&mut self, incoming: &Self) -> bool {
        merge_field(&mut self.service_name, &incoming.service_name)
            | merge_field(&mut self.service_version, &incoming.service_version)
            | merge_field(
                &mut self.deployment_environment,
                &incoming.deployment_environment,
            )
            | merge_field(
                &mut self.instrumentation_scope_name,
                &incoming.instrumentation_scope_name,
            )
            | merge_field(
                &mut self.instrumentation_scope_version,
                &incoming.instrumentation_scope_version,
            )
    }
}

fn merge_field(target: &mut Option<String>, incoming: &Option<String>) -> bool {
    match (target.as_ref(), incoming) {
        (None, Some(value)) => {
            *target = Some(value.clone());
            false
        }
        (Some(current), Some(value)) => current != value,
        _ => false,
    }
}

fn map_resource_context(resource: Option<&Resource>) -> Result<OtlpContext, &'static str> {
    let attributes = primitive_attributes(resource.map_or(&[], |value| &value.attributes))?;
    validate_label_attributes(&attributes)?;
    Ok(OtlpContext {
        service_name: string_attr(&attributes, &["service.name"]),
        service_version: string_attr(&attributes, &["service.version"]),
        deployment_environment: string_attr(
            &attributes,
            &["deployment.environment.name", "deployment.environment"],
        ),
        ..OtlpContext::default()
    })
}

fn map_scope_context(
    mut context: OtlpContext,
    scope: Option<&InstrumentationScope>,
) -> Result<OtlpContext, &'static str> {
    let Some(scope) = scope else {
        return Ok(context);
    };
    let attributes = primitive_attributes(&scope.attributes)?;
    validate_label_attributes(&attributes)?;
    context.instrumentation_scope_name = direct_label(&scope.name)?;
    context.instrumentation_scope_version = direct_label(&scope.version)?;
    Ok(context)
}

fn direct_label(value: &str) -> Result<Option<String>, &'static str> {
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > 256 || value.chars().any(char::is_control) {
        return Err("invalid_label");
    }
    Ok(Some(value.to_owned()))
}

fn primitive_attributes(values: &[KeyValue]) -> Result<HashMap<String, Primitive>, &'static str> {
    let mut result = HashMap::new();
    for pair in values {
        if pair.key.is_empty() || pair.key.len() > 256 || pair.key.chars().any(char::is_control) {
            return Err("invalid_attribute_key");
        }
        if result.contains_key(&pair.key) {
            return Err("duplicate_attribute_key");
        }
        validate_any_value(
            pair.value.as_ref().and_then(|value| value.value.as_ref()),
            0,
        )?;
        let primitive = match pair.value.as_ref().and_then(|value| value.value.as_ref()) {
            Some(Value::StringValue(value)) if value.len() <= 4_096 => {
                Some(Primitive::String(value.clone()))
            }
            Some(Value::BoolValue(value)) => Some(Primitive::Bool(*value)),
            Some(Value::IntValue(value)) => Some(Primitive::Int(*value)),
            Some(Value::DoubleValue(value)) if value.is_finite() => Some(Primitive::Double(*value)),
            _ => None,
        };
        if let Some(primitive) = primitive {
            result.insert(pair.key.clone(), primitive);
        }
    }
    Ok(result)
}

fn validate_label_attributes(values: &HashMap<String, Primitive>) -> Result<(), &'static str> {
    for (key, value) in values {
        if is_label_key(key)
            && matches!(value, Primitive::String(value) if value.len() > 256 || value.chars().any(char::is_control))
        {
            return Err("invalid_label");
        }
    }
    Ok(())
}

fn is_label_key(key: &str) -> bool {
    matches!(
        key,
        "corpuslab.operation"
            | "corpuslab.provider"
            | "corpuslab.generation_model"
            | "corpuslab.embedding_model"
            | "corpuslab.ranker"
            | "corpuslab.configuration_label"
            | "corpuslab.retrieval_mode"
            | "corpuslab.evaluation_label"
            | "corpuslab.evidence.citation"
            | "corpuslab.evidence.answer_support_status"
            | "corpuslab.evidence.answer_support_reason"
            | "gen_ai.provider.name"
            | "gen_ai.operation.name"
            | "gen_ai.response.model"
            | "gen_ai.request.model"
            | "openinference.span.kind"
            | "llm.provider"
            | "llm.model_name"
            | "embedding.model_name"
            | "reranker.model_name"
            | "error.type"
            | "service.name"
            | "service.version"
            | "deployment.environment.name"
            | "deployment.environment"
    )
}

fn validate_any_value(value: Option<&Value>, depth: usize) -> Result<(), &'static str> {
    if depth > 8 {
        return Err("attribute_nesting_limit_exceeded");
    }
    match value {
        Some(Value::StringValue(value)) if value.len() > 4_096 => {
            Err("attribute_string_limit_exceeded")
        }
        Some(Value::BytesValue(value)) if value.len() > 4_096 => {
            Err("attribute_bytes_limit_exceeded")
        }
        Some(Value::ArrayValue(value)) => {
            if value.values.len() > 32 {
                return Err("attribute_collection_limit_exceeded");
            }
            for item in &value.values {
                validate_any_value(item.value.as_ref(), depth + 1)?;
            }
            Ok(())
        }
        Some(Value::KvlistValue(value)) => {
            if value.values.len() > 32 {
                return Err("attribute_collection_limit_exceeded");
            }
            for item in &value.values {
                validate_any_value(
                    item.value.as_ref().and_then(|value| value.value.as_ref()),
                    depth + 1,
                )?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn operation(attributes: &HashMap<String, Primitive>) -> ImportedSpanOperation {
    let value = string_attr(attributes, &["corpuslab.operation"])
        .or_else(|| string_attr(attributes, &["openinference.span.kind"]))
        .or_else(|| string_attr(attributes, &["gen_ai.operation.name"]))
        .unwrap_or_default()
        .to_ascii_lowercase();
    if value.contains("retriev") {
        ImportedSpanOperation::Retrieval
    } else if value.contains("embed") {
        ImportedSpanOperation::Embedding
    } else if value.contains("rerank") {
        ImportedSpanOperation::Reranking
    } else if value.contains("generat") || value.contains("chat") {
        ImportedSpanOperation::Generation
    } else if value.contains("tool") {
        ImportedSpanOperation::Tool
    } else if value.contains("eval") || value.contains("guardrail") {
        ImportedSpanOperation::Eval
    } else {
        ImportedSpanOperation::Other
    }
}

fn retrieval_mode(spans: &[ImportedSpan]) -> Option<RetrievalMode> {
    spans
        .iter()
        .any(|span| span.operation == ImportedSpanOperation::Retrieval)
        .then_some(RetrievalMode::Hybrid)
}

fn string_attr(values: &HashMap<String, Primitive>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| match values.get(*key) {
        Some(Primitive::String(value))
            if !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control) =>
        {
            Some(value.clone())
        }
        _ => None,
    })
}

fn checked_id_attr(
    values: &HashMap<String, Primitive>,
    keys: &[&str],
) -> Result<Option<String>, &'static str> {
    let Some(value) = keys.iter().find_map(|key| values.get(*key)) else {
        return Ok(None);
    };
    let Primitive::String(value) = value else {
        return Err("invalid_identifier_attribute");
    };
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err("invalid_identifier_attribute");
    }
    Ok(Some(value.clone()))
}

fn checked_u32_attr(
    values: &HashMap<String, Primitive>,
    keys: &[&str],
    min: u32,
    max: u32,
) -> Result<Option<u32>, &'static str> {
    let Some(value) = keys.iter().find_map(|key| values.get(*key)) else {
        return Ok(None);
    };
    let value = match value {
        Primitive::Int(value) => u32::try_from(*value).ok(),
        Primitive::Double(value)
            if value.is_finite()
                && value.fract() == 0.0
                && *value >= min as f64
                && *value <= max as f64 =>
        {
            Some(*value as u32)
        }
        _ => None,
    }
    .filter(|value| (min..=max).contains(value))
    .ok_or("invalid_numeric_attribute")?;
    Ok(Some(value))
}

fn checked_score_attr(
    values: &HashMap<String, Primitive>,
    keys: &[&str],
) -> Result<Option<f32>, &'static str> {
    let Some(value) = keys.iter().find_map(|key| values.get(*key)) else {
        return Ok(None);
    };
    let value = match value {
        Primitive::Double(value) if value.is_finite() && (0.0..=1.0).contains(value) => {
            Some(*value as f32)
        }
        Primitive::Int(value) if (0..=1).contains(value) => Some(*value as f32),
        _ => None,
    }
    .ok_or("invalid_numeric_attribute")?;
    Ok(Some(value))
}

fn bool_attr(values: &HashMap<String, Primitive>, keys: &[&str]) -> Option<bool> {
    keys.iter().find_map(|key| match values.get(*key) {
        Some(Primitive::Bool(value)) => Some(*value),
        _ => None,
    })
}

fn failure_labels_attr(values: &[KeyValue]) -> Vec<FailureLabel> {
    values
        .iter()
        .find(|value| value.key == "corpuslab.failure_labels")
        .and_then(|value| value.value.as_ref())
        .and_then(|value| value.value.as_ref())
        .and_then(|value| match value {
            Value::ArrayValue(values) => Some(&values.values),
            _ => None,
        })
        .into_iter()
        .flatten()
        .filter_map(|value| match value.value.as_ref() {
            Some(Value::StringValue(value)) => parse_failure_label(value),
            _ => None,
        })
        .collect()
}

fn validate_failure_label_attribute(values: &[KeyValue]) -> Result<(), &'static str> {
    let Some(values) = values
        .iter()
        .find(|value| value.key == "corpuslab.failure_labels")
        .and_then(|value| value.value.as_ref())
        .and_then(|value| value.value.as_ref())
        .and_then(|value| match value {
            Value::ArrayValue(values) => Some(&values.values),
            _ => None,
        })
    else {
        return Ok(());
    };
    if values.iter().any(|value| {
        matches!(
            value.value.as_ref(),
            Some(Value::StringValue(value))
                if value.len() > 256 || value.chars().any(char::is_control)
        )
    }) {
        return Err("invalid_label");
    }
    Ok(())
}

fn parse_failure_label(value: &str) -> Option<FailureLabel> {
    match value {
        "missing_document" => Some(FailureLabel::MissingDocument),
        "bad_chunking" => Some(FailureLabel::BadChunking),
        "bad_embedding" => Some(FailureLabel::BadEmbedding),
        "bad_ranking" => Some(FailureLabel::BadRanking),
        "bad_prompt" => Some(FailureLabel::BadPrompt),
        "unsupported_question" => Some(FailureLabel::UnsupportedQuestion),
        "hallucinated_answer" => Some(FailureLabel::HallucinatedAnswer),
        "weak_evidence" => Some(FailureLabel::WeakEvidence),
        "missing_embedding_index" => Some(FailureLabel::MissingEmbeddingIndex),
        "duplicate_evidence" => Some(FailureLabel::DuplicateEvidence),
        "heading_only_evidence" => Some(FailureLabel::HeadingOnlyEvidence),
        _ => None,
    }
}

fn parse_retrieval_mode(value: &str) -> Option<RetrievalMode> {
    match value {
        "lexical" => Some(RetrievalMode::Lexical),
        "vector" => Some(RetrievalMode::Vector),
        "hybrid" => Some(RetrievalMode::Hybrid),
        _ => None,
    }
}

fn parse_support_status(value: &str) -> AnswerSupportStatus {
    match value {
        "supported" => AnswerSupportStatus::Supported,
        "unsupported" => AnswerSupportStatus::Unsupported,
        _ => AnswerSupportStatus::Unassessed,
    }
}

fn parse_support_reason(value: &str) -> AnswerSupportReason {
    match value {
        "direct_body_support" => AnswerSupportReason::DirectBodySupport,
        "insufficient_body_overlap" => AnswerSupportReason::InsufficientBodyOverlap,
        "semantic_only_match" => AnswerSupportReason::SemanticOnlyMatch,
        "metadata_only_match" => AnswerSupportReason::MetadataOnlyMatch,
        "path_only_match" => AnswerSupportReason::PathOnlyMatch,
        "section_only_match" => AnswerSupportReason::SectionOnlyMatch,
        "weak_evidence" => AnswerSupportReason::WeakEvidence,
        "heading_only_evidence" => AnswerSupportReason::HeadingOnlyEvidence,
        _ => AnswerSupportReason::Unassessed,
    }
}

fn is_sensitive_key(key: &str) -> bool {
    matches!(
        key,
        "corpuslab.query"
            | "corpuslab.answer"
            | "corpuslab.prompt"
            | "corpuslab.evidence.snippet"
            | "corpuslab.evidence.document_label"
            | "document.content"
            | "input.value"
            | "output.value"
            | "gen_ai.prompt"
            | "gen_ai.completion"
            | "gen_ai.retrieval.query.text"
            | "gen_ai.input.messages"
            | "gen_ai.output.messages"
            | "exception.message"
            | "exception.stacktrace"
    )
}

fn valid_id(value: &[u8], len: usize) -> bool {
    value.len() == len && value.iter().any(|byte| *byte != 0)
}

fn timestamp(nanos: u64) -> Option<OffsetDateTime> {
    (nanos != 0)
        .then(|| OffsetDateTime::from_unix_timestamp_nanos(i128::from(nanos)).ok())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry_proto::tonic::{
        collector::trace::v1::ExportTraceServiceRequest,
        common::v1::{AnyValue, KeyValue},
        trace::v1::{ResourceSpans, ScopeSpans, Span},
    };

    #[test]
    fn standard_protobuf_maps_multiple_spans_without_content() {
        let trace_id = vec![1; 16];
        let mut retrieval = span(trace_id.clone(), vec![2; 8], Vec::new(), "retrieval");
        retrieval.kind = SpanKind::Server as i32;
        let mut generation = span(trace_id, vec![3; 8], vec![2; 8], "generation");
        generation.kind = SpanKind::Client as i32;
        let request = ExportTraceServiceRequest {
            resource_spans: vec![ResourceSpans {
                resource: Some(Resource {
                    attributes: vec![string_attribute("service.name", "collector-demo")],
                    ..Resource::default()
                }),
                scope_spans: vec![ScopeSpans {
                    scope: Some(InstrumentationScope {
                        name: "corpuslab.test".to_owned(),
                        version: "1.0".to_owned(),
                        ..InstrumentationScope::default()
                    }),
                    schema_url: String::new(),
                    spans: vec![retrieval, generation],
                }],
                schema_url: String::new(),
            }],
        };
        let batch = decode_and_map(
            &request.encode_to_vec(),
            &project(),
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("map protobuf");
        assert_eq!(batch.traces.len(), 1);
        let metadata = batch.traces[0].ingestion.as_ref().expect("ingestion");
        assert_eq!(metadata.source, TraceIngestionSource::OtlpHttp);
        assert_eq!(metadata.spans.len(), 2);
        assert_eq!(metadata.spans[0].name, "Retrieval");
        assert_eq!(metadata.spans[0].kind, ImportedSpanKind::Server);
        assert_eq!(metadata.spans[0].status, ImportedSpanStatus::Succeeded);
        assert_eq!(metadata.spans[1].kind, ImportedSpanKind::Client);
        assert_eq!(metadata.spans[1].status, ImportedSpanStatus::Succeeded);
        assert_eq!(batch.traces[0].status, TraceStatus::Warning);
        assert_eq!(
            batch.traces[0].summary,
            "Imported trace is only partially mapped; diagnosis is limited."
        );
        assert!(metadata
            .limitations
            .iter()
            .any(|value| value == "span_names_not_retained"));
        assert_eq!(metadata.service_name.as_deref(), Some("collector-demo"));
        assert_eq!(
            metadata.instrumentation_scope_name.as_deref(),
            Some("corpuslab.test")
        );
        assert!(batch.traces[0].input.is_empty());
    }

    #[test]
    fn maps_multiple_trace_ids_and_rejects_only_a_malformed_sibling() {
        let valid_one = span(vec![1; 16], vec![1; 8], Vec::new(), "retrieval");
        let valid_two = span(vec![2; 16], vec![2; 8], Vec::new(), "generation");
        let request = export(vec![valid_one.clone(), valid_two]);
        let batch = decode_and_map(
            &request.encode_to_vec(),
            &project(),
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("map two traces");
        assert_eq!(batch.traces.len(), 2);
        assert_eq!(batch.rejected_spans, 0);

        let mut malformed = span(vec![3; 16], vec![3; 8], Vec::new(), "generation");
        malformed.end_time_unix_nano = malformed.start_time_unix_nano - 1;
        let batch = decode_and_map(
            &export(vec![valid_one, malformed]).encode_to_vec(),
            &project(),
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("partially map export");
        assert_eq!(batch.traces.len(), 1);
        assert_eq!(batch.rejected_spans, 1);
        assert_eq!(
            batch.rejection_reasons.get("invalid_span_timestamp"),
            Some(&1)
        );
    }

    #[test]
    fn conflicting_duplicate_span_rejects_the_whole_trace_deterministically() {
        let first = span(vec![9; 16], vec![7; 8], Vec::new(), "retrieval");
        let mut conflict = first.clone();
        conflict.name = "different".to_owned();
        let batch = decode_and_map(
            &export(vec![first, conflict]).encode_to_vec(),
            &project(),
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("decode conflicting export");
        assert!(batch.traces.is_empty());
        assert_eq!(batch.rejected_spans, 2);
        assert_eq!(
            batch.rejection_reasons.get("conflicting_duplicate_span"),
            Some(&1)
        );
    }

    #[test]
    fn mapping_precedence_preserves_unknown_operations_with_a_limitation() {
        let trace_id = vec![4; 16];
        let mut corpuslab = span(trace_id.clone(), vec![1; 8], Vec::new(), "retrieval");
        corpuslab.attributes.extend([
            string_attribute("openinference.span.kind", "LLM"),
            string_attribute("gen_ai.operation.name", "chat"),
        ]);
        let mut openinference = span(trace_id.clone(), vec![2; 8], Vec::new(), "ignored");
        openinference.attributes.remove(0);
        openinference.attributes.extend([
            string_attribute("openinference.span.kind", "RERANKER"),
            string_attribute("gen_ai.operation.name", "chat"),
        ]);
        let mut genai = span(trace_id.clone(), vec![3; 8], Vec::new(), "ignored");
        genai.attributes = vec![string_attribute("gen_ai.operation.name", "embeddings")];
        let mut unknown = span(trace_id, vec![4; 8], Vec::new(), "ignored");
        unknown.attributes.clear();
        let request = ExportTraceServiceRequest {
            resource_spans: vec![ResourceSpans {
                resource: None,
                scope_spans: vec![ScopeSpans {
                    scope: None,
                    spans: vec![corpuslab, openinference, genai, unknown],
                    schema_url: String::new(),
                }],
                schema_url: String::new(),
            }],
        };
        let batch = decode_and_map(
            &request.encode_to_vec(),
            &project(),
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("map semantic precedence");
        let metadata = batch.traces[0].ingestion.as_ref().expect("ingestion");
        assert_eq!(
            metadata.spans[0].operation,
            ImportedSpanOperation::Retrieval
        );
        assert_eq!(
            metadata.spans[1].operation,
            ImportedSpanOperation::Reranking
        );
        assert_eq!(
            metadata.spans[2].operation,
            ImportedSpanOperation::Embedding
        );
        assert_eq!(metadata.spans[3].operation, ImportedSpanOperation::Other);
        assert!(metadata
            .limitations
            .iter()
            .any(|value| value == "unsupported_operation"));
    }

    #[test]
    fn corpuslab_attributes_map_evidence_support_failure_and_eval_metadata() {
        let trace_id = vec![5; 16];
        let mut retrieval = span(trace_id, vec![1; 8], Vec::new(), "retrieval");
        retrieval.attributes.extend([
            string_attribute("corpuslab.retrieval_mode", "lexical"),
            int_attribute("corpuslab.top_k", 3),
            string_attribute("corpuslab.evidence.external_chunk_id", "chunk-7"),
            int_attribute("corpuslab.evidence.rank", 1),
            double_attribute("corpuslab.evidence.score", 0.9),
            string_attribute("corpuslab.evidence.citation", "E1"),
            string_attribute("corpuslab.evidence.answer_support_status", "supported"),
            string_attribute(
                "corpuslab.evidence.answer_support_reason",
                "direct_body_support",
            ),
            bool_attribute("corpuslab.evaluation_passed", false),
            string_attribute("corpuslab.evaluation_label", "release gate"),
            string_array_attribute("corpuslab.failure_labels", &["weak_evidence"]),
        ]);
        let request = ExportTraceServiceRequest {
            resource_spans: vec![ResourceSpans {
                resource: None,
                scope_spans: vec![ScopeSpans {
                    scope: None,
                    spans: vec![retrieval],
                    schema_url: String::new(),
                }],
                schema_url: String::new(),
            }],
        };
        let batch = decode_and_map(
            &request.encode_to_vec(),
            &project(),
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("map CorpusLab attributes");
        let trace = &batch.traces[0];
        let metadata = trace.ingestion.as_ref().expect("ingestion");
        assert_eq!(metadata.retrieval_mode, Some(RetrievalMode::Lexical));
        assert_eq!(metadata.top_k, Some(3));
        assert_eq!(metadata.evaluation_passed, Some(false));
        assert_eq!(metadata.evaluation_label.as_deref(), Some("release gate"));
        assert_eq!(metadata.evidence[0].external_chunk_id, "chunk-7");
        assert_eq!(
            metadata.evidence[0].answer_support_status,
            AnswerSupportStatus::Supported
        );
        assert_eq!(
            metadata.evidence[0].answer_support_reason,
            AnswerSupportReason::DirectBodySupport
        );
        assert!(trace.failure_labels.contains(&FailureLabel::WeakEvidence));
    }

    #[test]
    fn invalid_bounded_numeric_attributes_reject_the_owning_trace() {
        let trace_id = vec![6; 16];
        let mut retrieval = span(trace_id, vec![1; 8], Vec::new(), "retrieval");
        retrieval.attributes.extend([
            string_attribute("corpuslab.evidence.external_chunk_id", "chunk-7"),
            int_attribute("corpuslab.evidence.rank", 101),
        ]);
        let mut request = ExportTraceServiceRequest {
            resource_spans: vec![ResourceSpans {
                resource: None,
                scope_spans: vec![ScopeSpans {
                    scope: None,
                    spans: vec![retrieval],
                    schema_url: String::new(),
                }],
                schema_url: String::new(),
            }],
        };
        let batch = decode_and_map(
            &request.encode_to_vec(),
            &project(),
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("decode export");

        assert!(batch.traces.is_empty());
        assert_eq!(batch.rejected_spans, 1);
        assert_eq!(
            batch.rejection_reasons.get("invalid_numeric_attribute"),
            Some(&1)
        );

        let span = &mut request.resource_spans[0].scope_spans[0].spans[0];
        span.attributes.retain(|attribute| {
            !matches!(
                attribute.key.as_str(),
                "corpuslab.evidence.external_chunk_id" | "corpuslab.evidence.rank"
            )
        });
        span.attributes.extend([
            string_attribute("corpuslab.evidence.external_chunk_id", "bad/id"),
            int_attribute("corpuslab.evidence.rank", 1),
        ]);
        let batch = decode_and_map(
            &request.encode_to_vec(),
            &project(),
            &RetrievalConfig::default(),
            &DebuggerConfig::default(),
        )
        .expect("decode export");
        assert_eq!(batch.rejected_spans, 1);
        assert_eq!(
            batch.rejection_reasons.get("invalid_identifier_attribute"),
            Some(&1)
        );
    }

    fn span(trace_id: Vec<u8>, span_id: Vec<u8>, parent_span_id: Vec<u8>, operation: &str) -> Span {
        Span {
            trace_id,
            span_id,
            parent_span_id,
            name: "sensitive user supplied name".to_owned(),
            start_time_unix_nano: 1_800_000_000_000_000_000,
            end_time_unix_nano: 1_800_000_000_010_000_000,
            attributes: vec![string_attribute("corpuslab.operation", operation)],
            ..Span::default()
        }
    }

    fn export(spans: Vec<Span>) -> ExportTraceServiceRequest {
        ExportTraceServiceRequest {
            resource_spans: vec![ResourceSpans {
                resource: None,
                scope_spans: vec![ScopeSpans {
                    scope: None,
                    spans,
                    schema_url: String::new(),
                }],
                schema_url: String::new(),
            }],
        }
    }

    fn string_attribute(key: &str, value: &str) -> KeyValue {
        attribute(key, Value::StringValue(value.to_owned()))
    }

    fn int_attribute(key: &str, value: i64) -> KeyValue {
        attribute(key, Value::IntValue(value))
    }

    fn double_attribute(key: &str, value: f64) -> KeyValue {
        attribute(key, Value::DoubleValue(value))
    }

    fn bool_attribute(key: &str, value: bool) -> KeyValue {
        attribute(key, Value::BoolValue(value))
    }

    fn string_array_attribute(key: &str, values: &[&str]) -> KeyValue {
        use opentelemetry_proto::tonic::common::v1::{AnyValue, ArrayValue};
        attribute(
            key,
            Value::ArrayValue(ArrayValue {
                values: values
                    .iter()
                    .map(|value| AnyValue {
                        value: Some(Value::StringValue((*value).to_owned())),
                    })
                    .collect(),
            }),
        )
    }

    fn attribute(key: &str, value: Value) -> KeyValue {
        KeyValue {
            key: key.to_owned(),
            value: Some(AnyValue { value: Some(value) }),
            key_strindex: 0,
        }
    }

    fn project() -> Project {
        let now = OffsetDateTime::now_utc();
        Project {
            id: ProjectId(uuid::Uuid::now_v7()),
            name: "test".to_owned(),
            privacy_mode: PrivacyMode::LocalOnly,
            created_at: now,
            updated_at: now,
        }
    }
}
