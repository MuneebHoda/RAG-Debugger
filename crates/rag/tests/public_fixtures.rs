use std::{fs, path::PathBuf};

use serde_json::Value;

#[test]
fn public_regression_fixtures_are_valid_and_non_empty() {
    for relative_path in [
        "fixtures/expected/retrieval_cases.json",
        "fixtures/expected/trace_cases.json",
        "fixtures/expected/eval_lab_cases.json",
    ] {
        let path = workspace_root().join(relative_path);
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let fixture: Value = serde_json::from_str(&raw)
            .unwrap_or_else(|error| panic!("invalid JSON in {}: {error}", path.display()));

        assert_eq!(fixture["schema_version"], 1, "{}", path.display());
        assert!(
            fixture["cases"]
                .as_array()
                .is_some_and(|cases| !cases.is_empty()),
            "{} must define at least one case",
            path.display()
        );
    }
}

#[test]
fn public_corpora_cover_supported_regression_profiles() {
    for relative_path in [
        "fixtures/corpora/support_kb/account-recovery.md",
        "fixtures/corpora/policy_docs/data-retention.md",
        "fixtures/corpora/technical_docs/gpu-indexing.md",
    ] {
        let path = workspace_root().join(relative_path);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

        assert!(
            content.lines().count() >= 8,
            "{} is too small",
            path.display()
        );
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
