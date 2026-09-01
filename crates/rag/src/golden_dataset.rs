use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use rag_debugger_core::{
    is_valid_golden_dataset_key, ChunkId, DocumentId, GoldenDataset, GoldenDatasetCase,
    GoldenDatasetCaseProvenance, GoldenDatasetChunkReference, GoldenDatasetContentMode,
    GoldenDatasetDocumentReference, GoldenDatasetEvidenceIdentity, GoldenDatasetEvidenceKind,
    GoldenDatasetIdentity, GoldenDatasetImportAction, GoldenDatasetImportMode,
    GoldenDatasetInvalidCase, GoldenDatasetPrivacyField, GoldenDatasetUnresolvedEvidence,
    GoldenDatasetValidationCode, RetrievalEvalCase, RetrievalEvalCaseId,
    RetrievalEvalCaseProvenance, RetrievalEvalDataset, RetrievalEvalDatasetId, WorkspaceId,
    EVAL_LAB_EVIDENCE_MAX_REQUESTED_CHUNKS, EVAL_LAB_EVIDENCE_MAX_REQUESTED_DOCUMENTS,
    EVAL_LAB_EVIDENCE_MAX_REQUESTED_IDS, GOLDEN_DATASET_CASE_KEY_MAX_CHARS,
    GOLDEN_DATASET_SCHEMA_VERSION, RETRIEVAL_EVAL_EXPERIMENT_MAX_CASES,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum GoldenDatasetError {
    #[error(
        "golden dataset schema_version is required; unversioned legacy datasets are unsupported"
    )]
    MissingSchemaVersion,
    #[error("legacy golden dataset schema version {0} is unsupported; supported version is {GOLDEN_DATASET_SCHEMA_VERSION}")]
    LegacySchemaVersion(u64),
    #[error("unsupported future golden dataset schema version {0}; supported version is {GOLDEN_DATASET_SCHEMA_VERSION}")]
    FutureSchemaVersion(u64),
    #[error("golden dataset schema_version must be a non-negative integer")]
    InvalidSchemaVersion,
    #[error("golden dataset does not match schema version {GOLDEN_DATASET_SCHEMA_VERSION}")]
    InvalidSchema,
    #[error("golden dataset identity key must be the canonical lowercase key derived from the dataset name")]
    InvalidDatasetKey,
    #[error("eval dataset contains {0} cases, exceeding the supported golden dataset limit of {RETRIEVAL_EVAL_EXPERIMENT_MAX_CASES}")]
    TooManyCases(usize),
    #[error("full content export is not permitted for datasets containing full-local cases")]
    FullLocalExportNotPermitted,
    #[error(
        "dataset contains expected evidence that cannot be represented by the portable schema"
    )]
    UnresolvedExportEvidence,
    #[error("golden dataset serialization failed")]
    Serialization,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlannedGoldenDatasetImport {
    pub action: GoldenDatasetImportAction,
    pub target_dataset_id: Option<RetrievalEvalDatasetId>,
    pub expected_updated_at: Option<OffsetDateTime>,
    pub dataset_name: String,
    pub dataset_description: Option<String>,
    pub cases_to_create: Vec<RetrievalEvalCase>,
    pub cases_to_update: Vec<RetrievalEvalCase>,
    pub case_ids_to_delete: Vec<RetrievalEvalCaseId>,
    pub cases_skipped: u32,
    pub invalid_cases: Vec<GoldenDatasetInvalidCase>,
    pub unresolved_evidence: Vec<GoldenDatasetUnresolvedEvidence>,
    pub privacy_sensitive_fields: Vec<GoldenDatasetPrivacyField>,
}

impl PlannedGoldenDatasetImport {
    pub fn is_valid(&self) -> bool {
        self.invalid_cases.is_empty() && self.unresolved_evidence.is_empty()
    }

    pub fn cases_added(&self) -> u32 {
        self.cases_to_create.len() as u32
    }

    pub fn cases_changed(&self) -> u32 {
        self.cases_to_update.len() as u32
    }

    pub fn cases_removed(&self) -> u32 {
        self.case_ids_to_delete.len() as u32
    }
}

pub fn parse_golden_dataset(value: Value) -> Result<GoldenDataset, GoldenDatasetError> {
    let version = value
        .get("schema_version")
        .ok_or(GoldenDatasetError::MissingSchemaVersion)?
        .as_u64()
        .ok_or(GoldenDatasetError::InvalidSchemaVersion)?;
    if version < u64::from(GOLDEN_DATASET_SCHEMA_VERSION) {
        return Err(GoldenDatasetError::LegacySchemaVersion(version));
    }
    if version > u64::from(GOLDEN_DATASET_SCHEMA_VERSION) {
        return Err(GoldenDatasetError::FutureSchemaVersion(version));
    }
    serde_json::from_value(value).map_err(|_| GoldenDatasetError::InvalidSchema)
}

pub fn export_golden_dataset(
    dataset: &RetrievalEvalDataset,
    evidence: &[GoldenDatasetEvidenceIdentity],
    content_mode: GoldenDatasetContentMode,
) -> Result<GoldenDataset, GoldenDatasetError> {
    if dataset.cases.len() > RETRIEVAL_EVAL_EXPERIMENT_MAX_CASES {
        return Err(GoldenDatasetError::TooManyCases(dataset.cases.len()));
    }
    if content_mode == GoldenDatasetContentMode::Full
        && dataset.cases.iter().any(|case| {
            case.provenance
                .as_ref()
                .is_some_and(RetrievalEvalCaseProvenance::is_full_local)
        })
    {
        return Err(GoldenDatasetError::FullLocalExportNotPermitted);
    }

    let document_by_id = evidence
        .iter()
        .map(|identity| (identity.document_id, identity.document_checksum.as_str()))
        .collect::<HashMap<_, _>>();
    let chunk_by_id = evidence
        .iter()
        .filter_map(|identity| {
            Some((
                identity.chunk_id?,
                (
                    identity.document_checksum.as_str(),
                    identity.chunk_checksum.as_deref()?,
                    identity.chunk_ordinal?,
                ),
            ))
        })
        .collect::<HashMap<_, _>>();

    let mut cases = Vec::new();
    cases
        .try_reserve(dataset.cases.len())
        .map_err(|_| GoldenDatasetError::Serialization)?;
    for eval_case in &dataset.cases {
        let mut expected_documents = Vec::new();
        let mut expected_chunks = Vec::new();
        if content_mode == GoldenDatasetContentMode::Full {
            for document_id in &eval_case.expected_document_ids {
                let checksum = document_by_id
                    .get(document_id)
                    .ok_or(GoldenDatasetError::UnresolvedExportEvidence)?;
                expected_documents.push(GoldenDatasetDocumentReference {
                    document_checksum: (*checksum).to_owned(),
                });
            }
            for chunk_id in &eval_case.expected_chunk_ids {
                let (document_checksum, chunk_checksum, ordinal) = chunk_by_id
                    .get(chunk_id)
                    .ok_or(GoldenDatasetError::UnresolvedExportEvidence)?;
                expected_chunks.push(GoldenDatasetChunkReference {
                    document_checksum: (*document_checksum).to_owned(),
                    chunk_checksum: (*chunk_checksum).to_owned(),
                    ordinal: *ordinal,
                });
            }
            expected_documents.sort();
            expected_documents.dedup();
            expected_chunks.sort();
            expected_chunks.dedup();
        }
        cases.push(GoldenDatasetCase {
            case_key: effective_case_key(eval_case),
            name: eval_case.name.clone(),
            query: (content_mode == GoldenDatasetContentMode::Full)
                .then(|| eval_case.query.clone()),
            top_k: eval_case.top_k,
            expected_documents,
            expected_chunks,
            notes: (content_mode == GoldenDatasetContentMode::Full)
                .then(|| eval_case.notes.clone())
                .flatten(),
            provenance: eval_case.provenance.as_ref().map(|provenance| {
                GoldenDatasetCaseProvenance {
                    source_trace_id: provenance.source_trace_id.0.to_string(),
                    source: provenance.source,
                    privacy_mode: provenance.privacy_mode,
                }
            }),
        });
    }
    cases.sort_by(|left, right| left.case_key.cmp(&right.case_key));

    Ok(GoldenDataset {
        schema_version: GOLDEN_DATASET_SCHEMA_VERSION,
        content_mode,
        dataset: GoldenDatasetIdentity {
            key: slug(&dataset.name, "dataset"),
            name: dataset.name.clone(),
            description: (content_mode == GoldenDatasetContentMode::Full)
                .then(|| dataset.description.clone())
                .flatten(),
        },
        cases,
    })
}

pub fn canonical_golden_dataset_json(
    dataset: &GoldenDataset,
) -> Result<String, GoldenDatasetError> {
    let mut canonical = dataset.clone();
    canonical
        .cases
        .sort_by(|left, right| left.case_key.cmp(&right.case_key));
    for case in &mut canonical.cases {
        case.expected_documents.sort();
        case.expected_documents.dedup();
        case.expected_chunks.sort();
        case.expected_chunks.dedup();
    }
    let mut json =
        serde_json::to_string_pretty(&canonical).map_err(|_| GoldenDatasetError::Serialization)?;
    json.push('\n');
    Ok(json)
}

pub fn plan_golden_dataset_import(
    portable: &GoldenDataset,
    mode: GoldenDatasetImportMode,
    current: Option<&RetrievalEvalDataset>,
    evidence: &[GoldenDatasetEvidenceIdentity],
    provenance: &HashMap<String, RetrievalEvalCaseProvenance>,
    max_top_k: u32,
    now: OffsetDateTime,
) -> Result<PlannedGoldenDatasetImport, GoldenDatasetError> {
    if portable.cases.len() > RETRIEVAL_EVAL_EXPERIMENT_MAX_CASES {
        return Err(GoldenDatasetError::TooManyCases(portable.cases.len()));
    }
    if !is_valid_golden_dataset_key(&portable.dataset.key)
        || portable.dataset.key != slug(portable.dataset.name.trim(), "dataset")
    {
        return Err(GoldenDatasetError::InvalidDatasetKey);
    }

    let action = match mode {
        GoldenDatasetImportMode::CreateNew => GoldenDatasetImportAction::CreateDataset,
        GoldenDatasetImportMode::MergeByCaseKey | GoldenDatasetImportMode::ReplaceDataset => {
            GoldenDatasetImportAction::UpdateDataset
        }
        GoldenDatasetImportMode::ValidateOnly => GoldenDatasetImportAction::ValidateOnly,
    };
    let mut invalid_cases = Vec::new();
    let mut unresolved_evidence = Vec::new();
    let mut seen_keys = HashSet::new();
    let document_matches = document_matches(evidence);
    let chunk_matches = chunk_matches(evidence);
    let mut imported_cases = Vec::new();
    imported_cases
        .try_reserve(portable.cases.len())
        .map_err(|_| GoldenDatasetError::Serialization)?;

    for case in &portable.cases {
        let mut valid = true;
        if !is_valid_golden_dataset_key(&case.case_key) {
            invalid_cases.push(invalid_case(
                case,
                GoldenDatasetValidationCode::InvalidCaseKey,
                "case_key must contain 1-128 lowercase letters, numbers, dots, underscores, or hyphens",
            ));
            valid = false;
        }
        if !seen_keys.insert(case.case_key.clone()) {
            invalid_cases.push(invalid_case(
                case,
                GoldenDatasetValidationCode::DuplicateCaseKey,
                "case_key is duplicated in the import",
            ));
            valid = false;
        }
        if case.name.trim().is_empty() {
            invalid_cases.push(invalid_case(
                case,
                GoldenDatasetValidationCode::MissingName,
                "case name must not be empty",
            ));
            valid = false;
        }
        let query = case.query.as_deref().map(str::trim).unwrap_or_default();
        if query.is_empty() {
            invalid_cases.push(invalid_case(
                case,
                GoldenDatasetValidationCode::MissingQuery,
                "full import requires a non-empty query",
            ));
            valid = false;
        }
        if case.top_k == 0 || case.top_k > max_top_k {
            invalid_cases.push(invalid_case(
                case,
                GoldenDatasetValidationCode::InvalidTopK,
                &format!("top_k must be between 1 and {max_top_k}"),
            ));
            valid = false;
        }
        if case.expected_documents.is_empty() && case.expected_chunks.is_empty() {
            invalid_cases.push(invalid_case(
                case,
                GoldenDatasetValidationCode::MissingExpectedEvidence,
                "case needs at least one expected document or chunk reference",
            ));
            valid = false;
        }
        if case.expected_documents.len() > EVAL_LAB_EVIDENCE_MAX_REQUESTED_DOCUMENTS
            || case.expected_chunks.len() > EVAL_LAB_EVIDENCE_MAX_REQUESTED_CHUNKS
            || case.expected_documents.len() + case.expected_chunks.len()
                > EVAL_LAB_EVIDENCE_MAX_REQUESTED_IDS
        {
            invalid_cases.push(invalid_case(
                case,
                GoldenDatasetValidationCode::TooManyEvidenceReferences,
                "case exceeds the supported Eval Lab evidence-reference limits",
            ));
            valid = false;
        }

        let mut expected_document_ids = Vec::new();
        for reference in &case.expected_documents {
            match document_matches.get(reference.document_checksum.as_str()) {
                Some(ids) if ids.len() == 1 => expected_document_ids.push(ids[0]),
                _ => {
                    unresolved_evidence.push(GoldenDatasetUnresolvedEvidence {
                        case_key: case.case_key.clone(),
                        kind: GoldenDatasetEvidenceKind::Document,
                        document_checksum: reference.document_checksum.clone(),
                        chunk_checksum: None,
                        ordinal: None,
                    });
                    valid = false;
                }
            }
        }

        let mut expected_chunk_ids = Vec::new();
        for reference in &case.expected_chunks {
            let key = (
                reference.document_checksum.as_str(),
                reference.chunk_checksum.as_str(),
                reference.ordinal,
            );
            match chunk_matches.get(&key) {
                Some(ids) if ids.len() == 1 => expected_chunk_ids.push(ids[0]),
                _ => {
                    unresolved_evidence.push(GoldenDatasetUnresolvedEvidence {
                        case_key: case.case_key.clone(),
                        kind: GoldenDatasetEvidenceKind::Chunk,
                        document_checksum: reference.document_checksum.clone(),
                        chunk_checksum: Some(reference.chunk_checksum.clone()),
                        ordinal: Some(reference.ordinal),
                    });
                    valid = false;
                }
            }
        }
        expected_document_ids.sort_by_key(|id| id.0);
        expected_document_ids.dedup();
        expected_chunk_ids.sort_by_key(|id| id.0);
        expected_chunk_ids.dedup();

        let case_provenance = if let Some(portable_provenance) = &case.provenance {
            match Uuid::parse_str(&portable_provenance.source_trace_id) {
                Ok(_) => match provenance.get(&case.case_key) {
                    Some(resolved) => Some(resolved.clone()),
                    None => {
                        invalid_cases.push(invalid_case(
                            case,
                            GoldenDatasetValidationCode::UnavailableProvenance,
                            "source trace provenance is unavailable in the target workspace",
                        ));
                        valid = false;
                        None
                    }
                },
                Err(_) => {
                    invalid_cases.push(invalid_case(
                        case,
                        GoldenDatasetValidationCode::MalformedId,
                        "source_trace_id must be a UUID",
                    ));
                    valid = false;
                    None
                }
            }
        } else {
            None
        };

        if valid {
            imported_cases.push(RetrievalEvalCase {
                id: RetrievalEvalCaseId(Uuid::now_v7()),
                case_key: case.case_key.clone(),
                name: case.name.trim().to_owned(),
                query: query.to_owned(),
                top_k: case.top_k,
                expected_chunk_ids,
                expected_document_ids,
                notes: case
                    .notes
                    .as_ref()
                    .map(|notes| notes.trim().to_owned())
                    .filter(|notes| !notes.is_empty()),
                provenance: case_provenance,
                created_at: now,
            });
        }
    }

    imported_cases.sort_by(|left, right| left.case_key.cmp(&right.case_key));
    invalid_cases.sort_by(|left, right| {
        left.case_key
            .cmp(&right.case_key)
            .then_with(|| left.code.cmp(&right.code))
    });
    unresolved_evidence.sort_by(|left, right| {
        left.case_key
            .cmp(&right.case_key)
            .then_with(|| left.document_checksum.cmp(&right.document_checksum))
            .then_with(|| left.chunk_checksum.cmp(&right.chunk_checksum))
            .then_with(|| left.ordinal.cmp(&right.ordinal))
    });

    let mut plan = PlannedGoldenDatasetImport {
        action,
        target_dataset_id: current.map(|dataset| dataset.id),
        expected_updated_at: current.map(|dataset| dataset.updated_at),
        dataset_name: portable.dataset.name.trim().to_owned(),
        dataset_description: portable
            .dataset
            .description
            .as_ref()
            .map(|description| description.trim().to_owned())
            .filter(|description| !description.is_empty()),
        cases_to_create: Vec::new(),
        cases_to_update: Vec::new(),
        case_ids_to_delete: Vec::new(),
        cases_skipped: 0,
        invalid_cases,
        unresolved_evidence,
        privacy_sensitive_fields: privacy_fields(portable),
    };

    if plan.dataset_name.is_empty() {
        plan.invalid_cases.push(GoldenDatasetInvalidCase {
            case_key: String::new(),
            code: GoldenDatasetValidationCode::MissingName,
            message: "dataset name must not be empty".to_owned(),
        });
    }

    match (mode, current) {
        (GoldenDatasetImportMode::CreateNew | GoldenDatasetImportMode::ValidateOnly, _) => {
            plan.cases_to_create = imported_cases;
        }
        (GoldenDatasetImportMode::MergeByCaseKey, Some(current)) => {
            plan_merge(&mut plan, current, imported_cases);
        }
        (GoldenDatasetImportMode::ReplaceDataset, Some(current)) => {
            plan_replace(&mut plan, current, imported_cases);
        }
        (
            GoldenDatasetImportMode::MergeByCaseKey | GoldenDatasetImportMode::ReplaceDataset,
            None,
        ) => {}
    }

    Ok(plan)
}

pub fn import_validation_token(
    workspace_id: WorkspaceId,
    mode: GoldenDatasetImportMode,
    portable: &GoldenDataset,
    current: Option<&RetrievalEvalDataset>,
) -> Result<String, GoldenDatasetError> {
    let mut hasher = Sha256::new();
    hasher.update(b"corpuslab-golden-dataset-import-v1\0");
    hasher.update(workspace_id.0.as_bytes());
    hasher.update(serde_json::to_vec(&mode).map_err(|_| GoldenDatasetError::Serialization)?);
    hasher.update(canonical_golden_dataset_json(portable)?.as_bytes());
    if let Some(dataset) = current {
        hasher.update(dataset.id.0.as_bytes());
        hasher.update(dataset.updated_at.unix_timestamp_nanos().to_be_bytes());
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn next_case_key(
    name: &str,
    query: &str,
    existing: impl IntoIterator<Item = String>,
) -> String {
    let existing = existing.into_iter().collect::<HashSet<_>>();
    let mut base = slug(name, "case");
    if base == "case" && !query.trim().is_empty() {
        base = slug(query, "case");
    }
    base.truncate(GOLDEN_DATASET_CASE_KEY_MAX_CHARS.saturating_sub(8));
    if !existing.contains(&base) {
        return base;
    }
    for suffix in 2_u32.. {
        let candidate = format!("{base}-{suffix}");
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!("finite existing case keys always leave a numeric suffix")
}

fn plan_merge(
    plan: &mut PlannedGoldenDatasetImport,
    current: &RetrievalEvalDataset,
    imported_cases: Vec<RetrievalEvalCase>,
) {
    let current_by_key = current
        .cases
        .iter()
        .map(|case| (effective_case_key(case), case))
        .collect::<BTreeMap<_, _>>();
    for mut imported in imported_cases {
        if let Some(local) = current_by_key.get(&imported.case_key) {
            if local.provenance != imported.provenance {
                plan.invalid_cases.push(GoldenDatasetInvalidCase {
                    case_key: imported.case_key.clone(),
                    code: GoldenDatasetValidationCode::ImmutableProvenanceConflict,
                    message: "import would change immutable case provenance".to_owned(),
                });
                continue;
            }
            imported.id = local.id;
            imported.created_at = local.created_at;
            if same_case_content(local, &imported) {
                plan.cases_skipped += 1;
            } else {
                plan.cases_to_update.push(imported);
            }
        } else {
            plan.cases_to_create.push(imported);
        }
    }
}

fn plan_replace(
    plan: &mut PlannedGoldenDatasetImport,
    current: &RetrievalEvalDataset,
    imported_cases: Vec<RetrievalEvalCase>,
) {
    let current_by_key = current
        .cases
        .iter()
        .map(|case| (effective_case_key(case), case))
        .collect::<BTreeMap<_, _>>();
    let imported_keys = imported_cases
        .iter()
        .map(|case| case.case_key.clone())
        .collect::<BTreeSet<_>>();
    plan.case_ids_to_delete = current
        .cases
        .iter()
        .filter(|case| !imported_keys.contains(&effective_case_key(case)))
        .map(|case| case.id)
        .collect();
    for mut imported in imported_cases {
        if let Some(local) = current_by_key.get(&imported.case_key) {
            if local.provenance != imported.provenance {
                plan.invalid_cases.push(GoldenDatasetInvalidCase {
                    case_key: imported.case_key.clone(),
                    code: GoldenDatasetValidationCode::ImmutableProvenanceConflict,
                    message: "import would change immutable case provenance".to_owned(),
                });
                continue;
            }
            imported.id = local.id;
            imported.created_at = local.created_at;
            if same_case_content(local, &imported) {
                plan.cases_skipped += 1;
            } else {
                plan.cases_to_update.push(imported);
            }
        } else {
            plan.cases_to_create.push(imported);
        }
    }
}

fn same_case_content(left: &RetrievalEvalCase, right: &RetrievalEvalCase) -> bool {
    left.case_key == right.case_key
        && left.name == right.name
        && left.query == right.query
        && left.top_k == right.top_k
        && left.expected_chunk_ids == right.expected_chunk_ids
        && left.expected_document_ids == right.expected_document_ids
        && left.notes == right.notes
        && left.provenance == right.provenance
}

fn document_matches(evidence: &[GoldenDatasetEvidenceIdentity]) -> HashMap<&str, Vec<DocumentId>> {
    let mut matches = HashMap::<&str, Vec<DocumentId>>::new();
    for identity in evidence {
        let ids = matches
            .entry(identity.document_checksum.as_str())
            .or_default();
        if !ids.contains(&identity.document_id) {
            ids.push(identity.document_id);
        }
    }
    matches
}

type ChunkChecksumKey<'a> = (&'a str, &'a str, u32);

fn chunk_matches(
    evidence: &[GoldenDatasetEvidenceIdentity],
) -> HashMap<ChunkChecksumKey<'_>, Vec<ChunkId>> {
    let mut matches = HashMap::<ChunkChecksumKey<'_>, Vec<ChunkId>>::new();
    for identity in evidence {
        if let (Some(chunk_id), Some(chunk_checksum), Some(ordinal)) = (
            identity.chunk_id,
            identity.chunk_checksum.as_deref(),
            identity.chunk_ordinal,
        ) {
            matches
                .entry((identity.document_checksum.as_str(), chunk_checksum, ordinal))
                .or_default()
                .push(chunk_id);
        }
    }
    matches
}

fn privacy_fields(portable: &GoldenDataset) -> Vec<GoldenDatasetPrivacyField> {
    let mut fields = BTreeSet::new();
    for case in &portable.cases {
        if case.query.as_ref().is_some_and(|query| !query.is_empty()) {
            fields.insert(GoldenDatasetPrivacyField::Queries);
        }
        if case.notes.as_ref().is_some_and(|notes| !notes.is_empty()) {
            fields.insert(GoldenDatasetPrivacyField::Notes);
        }
        if !case.expected_documents.is_empty() || !case.expected_chunks.is_empty() {
            fields.insert(GoldenDatasetPrivacyField::EvidenceReferences);
        }
        if case.provenance.is_some() {
            fields.insert(GoldenDatasetPrivacyField::Provenance);
        }
    }
    fields.into_iter().collect()
}

fn invalid_case(
    case: &GoldenDatasetCase,
    code: GoldenDatasetValidationCode,
    message: &str,
) -> GoldenDatasetInvalidCase {
    GoldenDatasetInvalidCase {
        case_key: case.case_key.clone(),
        code,
        message: message.to_owned(),
    }
}

fn effective_case_key(eval_case: &RetrievalEvalCase) -> String {
    if !eval_case.case_key.is_empty() {
        return eval_case.case_key.clone();
    }
    let mut hasher = Sha256::new();
    hasher.update(eval_case.name.as_bytes());
    hasher.update(b"\0");
    hasher.update(eval_case.query.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("{}-{}", slug(&eval_case.name, "case"), &digest[..12])
}

fn slug(value: &str, fallback: &str) -> String {
    let mut slug = String::new();
    let mut separator = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            separator = false;
        } else if !slug.is_empty() && !separator {
            slug.push('-');
            separator = true;
        }
        if slug.len() >= GOLDEN_DATASET_CASE_KEY_MAX_CHARS {
            break;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        fallback.to_owned()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use rag_debugger_core::{
        GoldenDatasetCaseProvenance, RetrievalEvalCaseProvenance, TraceIngestionPrivacyMode,
        TraceIngestionSource,
    };

    use super::*;

    #[test]
    fn export_is_canonical_stable_and_round_trips_case_keys() {
        assert_eq!(
            next_case_key(
                "Release Gate",
                "ignored",
                ["release-gate".to_owned(), "release-gate-2".to_owned()]
            ),
            "release-gate-3"
        );
        let evidence = evidence();
        let dataset = dataset(vec![
            eval_case("z-case", "Second"),
            eval_case("a-case", "First"),
        ]);
        let first = export_golden_dataset(&dataset, &evidence, GoldenDatasetContentMode::Full)
            .expect("export");
        let second = export_golden_dataset(&dataset, &evidence, GoldenDatasetContentMode::Full)
            .expect("repeat export");
        let first_json = canonical_golden_dataset_json(&first).expect("canonical JSON");
        let second_json = canonical_golden_dataset_json(&second).expect("repeat canonical JSON");

        assert_eq!(first_json, second_json);
        assert!(first_json.ends_with('\n'));
        assert!(first_json.find("a-case").unwrap() < first_json.find("z-case").unwrap());

        let plan = plan_golden_dataset_import(
            &first,
            GoldenDatasetImportMode::CreateNew,
            None,
            &evidence,
            &HashMap::new(),
            25,
            timestamp(20),
        )
        .expect("round-trip plan");
        assert!(plan.is_valid());
        assert_eq!(
            plan.cases_to_create
                .iter()
                .map(|case| case.case_key.as_str())
                .collect::<Vec<_>>(),
            vec!["a-case", "z-case"]
        );
    }

    #[test]
    fn metadata_export_redacts_content_and_full_local_blocks_full_export() {
        let mut case = eval_case("private-case", "Private");
        case.notes = Some("sensitive notes".to_owned());
        case.provenance = Some(RetrievalEvalCaseProvenance {
            source_trace_id: rag_debugger_core::TraceId(id(90)),
            source: TraceIngestionSource::Native,
            privacy_mode: TraceIngestionPrivacyMode::FullLocalOnly,
        });
        let dataset = dataset(vec![case]);

        assert!(matches!(
            export_golden_dataset(&dataset, &evidence(), GoldenDatasetContentMode::Full),
            Err(GoldenDatasetError::FullLocalExportNotPermitted)
        ));
        let metadata = export_golden_dataset(
            &dataset,
            &evidence(),
            GoldenDatasetContentMode::MetadataOnly,
        )
        .expect("metadata export");
        assert_eq!(metadata.cases[0].query, None);
        assert_eq!(metadata.cases[0].notes, None);
        assert!(metadata.cases[0].expected_documents.is_empty());
        assert!(metadata.cases[0].expected_chunks.is_empty());
        assert_eq!(metadata.dataset.description, None);
    }

    #[test]
    fn import_modes_are_deterministic_and_preserve_unmentioned_merge_cases() {
        let current = dataset(vec![
            eval_case("alpha", "Old"),
            eval_case("local-only", "Local"),
        ]);
        let mut portable = export_golden_dataset(
            &dataset(vec![
                eval_case("alpha", "Changed"),
                eval_case("beta", "New"),
            ]),
            &evidence(),
            GoldenDatasetContentMode::Full,
        )
        .expect("portable dataset");
        portable.cases[0].name = "Changed imported name".to_owned();

        let merge = plan_golden_dataset_import(
            &portable,
            GoldenDatasetImportMode::MergeByCaseKey,
            Some(&current),
            &evidence(),
            &HashMap::new(),
            25,
            timestamp(20),
        )
        .expect("merge plan");
        assert!(merge.is_valid());
        assert_eq!(merge.cases_added(), 1);
        assert_eq!(merge.cases_changed(), 1);
        assert_eq!(merge.cases_removed(), 0);

        let replace = plan_golden_dataset_import(
            &portable,
            GoldenDatasetImportMode::ReplaceDataset,
            Some(&current),
            &evidence(),
            &HashMap::new(),
            25,
            timestamp(20),
        )
        .expect("replace plan");
        assert!(replace.is_valid());
        assert_eq!(replace.cases_removed(), 1);
        assert_eq!(replace.case_ids_to_delete[0], current.cases[1].id);
    }

    #[test]
    fn validation_reports_duplicates_unresolved_references_and_malformed_ids() {
        let mut portable = export_golden_dataset(
            &dataset(vec![eval_case("duplicate", "First")]),
            &evidence(),
            GoldenDatasetContentMode::Full,
        )
        .expect("portable dataset");
        let mut duplicate = portable.cases[0].clone();
        duplicate.expected_documents[0].document_checksum = "missing".to_owned();
        duplicate.provenance = Some(GoldenDatasetCaseProvenance {
            source_trace_id: "not-a-uuid".to_owned(),
            source: TraceIngestionSource::Native,
            privacy_mode: TraceIngestionPrivacyMode::FullLocalOnly,
        });
        portable.cases.push(duplicate);

        let plan = plan_golden_dataset_import(
            &portable,
            GoldenDatasetImportMode::ValidateOnly,
            None,
            &evidence(),
            &HashMap::new(),
            25,
            timestamp(20),
        )
        .expect("validation plan");
        assert!(!plan.is_valid());
        assert!(plan
            .invalid_cases
            .iter()
            .any(|issue| issue.code == GoldenDatasetValidationCode::DuplicateCaseKey));
        assert!(plan
            .invalid_cases
            .iter()
            .any(|issue| issue.code == GoldenDatasetValidationCode::MalformedId));
        assert_eq!(plan.unresolved_evidence.len(), 1);
    }

    #[test]
    fn schema_versions_and_case_limits_are_explicit() {
        assert!(matches!(
            parse_golden_dataset(serde_json::json!({"dataset": {}, "cases": []})),
            Err(GoldenDatasetError::MissingSchemaVersion)
        ));
        assert!(matches!(
            parse_golden_dataset(serde_json::json!({"schema_version": 0})),
            Err(GoldenDatasetError::LegacySchemaVersion(0))
        ));
        assert!(matches!(
            parse_golden_dataset(serde_json::json!({"schema_version": 2})),
            Err(GoldenDatasetError::FutureSchemaVersion(2))
        ));

        let mut invalid_identity = export_golden_dataset(
            &dataset(vec![eval_case("case-key", "Question")]),
            &evidence(),
            GoldenDatasetContentMode::Full,
        )
        .expect("portable dataset");
        for invalid_key in ["", "different-name"] {
            invalid_identity.dataset.key = invalid_key.to_owned();
            assert!(matches!(
                plan_golden_dataset_import(
                    &invalid_identity,
                    GoldenDatasetImportMode::ValidateOnly,
                    None,
                    &evidence(),
                    &HashMap::new(),
                    25,
                    timestamp(20),
                ),
                Err(GoldenDatasetError::InvalidDatasetKey)
            ));
        }

        let at_limit = dataset(
            (0..RETRIEVAL_EVAL_EXPERIMENT_MAX_CASES)
                .map(|index| eval_case(&format!("case-{index}"), "Question"))
                .collect(),
        );
        let at_limit_portable =
            export_golden_dataset(&at_limit, &evidence(), GoldenDatasetContentMode::Full)
                .expect("at-limit portable dataset");
        let at_limit_plan = plan_golden_dataset_import(
            &at_limit_portable,
            GoldenDatasetImportMode::ValidateOnly,
            None,
            &evidence(),
            &HashMap::new(),
            25,
            timestamp(20),
        )
        .expect("at-limit import plan");
        assert!(at_limit_plan.is_valid());
        assert_eq!(
            at_limit_plan.cases_to_create.len(),
            RETRIEVAL_EVAL_EXPERIMENT_MAX_CASES
        );
        let mut over_limit_portable = at_limit_portable;
        let mut extra_portable_case = over_limit_portable.cases[0].clone();
        extra_portable_case.case_key = "over-limit".to_owned();
        over_limit_portable.cases.push(extra_portable_case);
        assert!(matches!(
            plan_golden_dataset_import(
                &over_limit_portable,
                GoldenDatasetImportMode::ValidateOnly,
                None,
                &evidence(),
                &HashMap::new(),
                25,
                timestamp(20),
            ),
            Err(GoldenDatasetError::TooManyCases(_))
        ));
        let mut over_limit = at_limit;
        over_limit.cases.push(eval_case("over-limit", "Question"));
        assert!(matches!(
            export_golden_dataset(
                &over_limit,
                &evidence(),
                GoldenDatasetContentMode::MetadataOnly
            ),
            Err(GoldenDatasetError::TooManyCases(_))
        ));
    }

    fn dataset(cases: Vec<RetrievalEvalCase>) -> RetrievalEvalDataset {
        RetrievalEvalDataset {
            id: RetrievalEvalDatasetId(id(1)),
            name: "Golden dataset".to_owned(),
            description: Some("Review fixture".to_owned()),
            cases,
            created_at: timestamp(1),
            updated_at: timestamp(2),
        }
    }

    fn eval_case(case_key: &str, name: &str) -> RetrievalEvalCase {
        RetrievalEvalCase {
            id: RetrievalEvalCaseId(Uuid::now_v7()),
            case_key: case_key.to_owned(),
            name: name.to_owned(),
            query: format!("{name} query"),
            top_k: 5,
            expected_chunk_ids: vec![ChunkId(id(4))],
            expected_document_ids: vec![DocumentId(id(3))],
            notes: None,
            provenance: None,
            created_at: timestamp(3),
        }
    }

    fn evidence() -> Vec<GoldenDatasetEvidenceIdentity> {
        vec![
            GoldenDatasetEvidenceIdentity {
                document_id: DocumentId(id(3)),
                document_checksum: "document-checksum".to_owned(),
                chunk_id: None,
                chunk_checksum: None,
                chunk_ordinal: None,
            },
            GoldenDatasetEvidenceIdentity {
                document_id: DocumentId(id(3)),
                document_checksum: "document-checksum".to_owned(),
                chunk_id: Some(ChunkId(id(4))),
                chunk_checksum: Some("chunk-checksum".to_owned()),
                chunk_ordinal: Some(0),
            },
        ]
    }

    fn id(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn timestamp(value: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(value).expect("timestamp")
    }
}
