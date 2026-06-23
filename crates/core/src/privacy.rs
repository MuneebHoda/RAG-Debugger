use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum PrivacyMode {
    LocalOnly,
    RedactedCloudSync,
    ExplicitSnippetSync,
}

impl PrivacyMode {
    pub fn permits_raw_document_upload(self) -> bool {
        matches!(self, Self::ExplicitSnippetSync)
    }
}
