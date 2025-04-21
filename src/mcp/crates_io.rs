use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CrateResponse {
    pub crate_data: CrateData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CrateData {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub downloads: u64,
    pub version_downloads: u64,
    pub versions: Option<Vec<u64>>,
    pub max_version: String,
    pub documentation: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionsResponse {
    pub versions: Vec<Version>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Version {
    pub id: String,
    pub num: String,
    pub created_at: String,
    pub updated_at: String,
    pub downloads: u64,
    pub yanked: bool,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResponse {
    pub crates: Vec<CrateSummary>,
    pub meta: SearchMeta,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CrateSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub downloads: u64,
    pub max_version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchMeta {
    pub total: u64,
}