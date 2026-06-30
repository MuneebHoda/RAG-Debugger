use rag_debugger_core::DebugReportPrivacyMode;

pub(super) const MAX_REPORT_SNIPPET_CHARS: usize = 280;

pub(super) fn permits_content(mode: DebugReportPrivacyMode) -> bool {
    !matches!(mode, DebugReportPrivacyMode::MetadataOnly)
}

pub(super) fn evidence_text(
    mode: DebugReportPrivacyMode,
    snippet: &str,
    full_text: &str,
) -> Option<String> {
    match mode {
        DebugReportPrivacyMode::MetadataOnly => None,
        DebugReportPrivacyMode::SnippetsAllowed => Some(bounded_snippet(snippet)),
        DebugReportPrivacyMode::FullLocalOnly => Some(full_text.to_owned()),
    }
}

pub(super) fn bounded_snippet(value: &str) -> String {
    if value.chars().count() <= MAX_REPORT_SNIPPET_CHARS {
        return value.to_owned();
    }

    let content_chars = MAX_REPORT_SNIPPET_CHARS.saturating_sub(3);
    let truncated = value.chars().take(content_chars).collect::<String>();
    if MAX_REPORT_SNIPPET_CHARS >= 3 {
        format!("{truncated}...")
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_mode_applies_a_character_bound() {
        let value = "x".repeat(MAX_REPORT_SNIPPET_CHARS + 10);

        let snippet = evidence_text(DebugReportPrivacyMode::SnippetsAllowed, &value, &value)
            .expect("snippet is permitted");

        assert_eq!(snippet.chars().count(), MAX_REPORT_SNIPPET_CHARS);
        assert!(snippet.ends_with("..."));
    }

    #[test]
    fn metadata_mode_excludes_content() {
        assert_eq!(
            evidence_text(DebugReportPrivacyMode::MetadataOnly, "snippet", "full text"),
            None
        );
    }
}
