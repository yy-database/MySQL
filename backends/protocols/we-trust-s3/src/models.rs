use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ListAllMyBucketsResult {
    pub owner: Owner,
    pub buckets: Buckets,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Owner {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Buckets {
    #[serde(rename = "Bucket")]
    pub bucket_list: Vec<Bucket>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Bucket {
    pub name: String,
    pub creation_date: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PutObjectOutput {
    pub e_tag: String,
    pub version_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    pub resource: String,
    pub request_id: String,
}
