#[derive(Debug, Clone)]
pub struct AddonInfo {
    pub addon_type: String,
    pub name: String,
    pub path: String,
    pub files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AddonMetadata {
    pub name: String,
    pub version: String,
    pub access: String,
    pub publisher: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AddonSubmissionData {
    pub id: Option<String>,
    pub name: String,
    pub addon_type: String,
    pub publisher: String,
    pub path: String,
    pub version: String,
    pub access: String,
    pub files: Vec<String>,
}
