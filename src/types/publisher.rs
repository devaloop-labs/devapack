use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherInfo {
    pub identifier: String,
    pub display_name: String,
    pub description: String,
    pub logo_url: Option<String>,
    pub banner_url: Option<String>,
    pub country_code: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherInfoUpdate {
    pub display_name: String,
    pub description: String,
    pub logo_url: Option<String>,
    pub banner_url: Option<String>,
    pub country_code: Option<String>,
    pub tags: Vec<String>,
}
