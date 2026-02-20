use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a storage bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bucket {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub region: String,
    pub object_count: u64,
    pub total_size: u64,
}

/// Represents an object stored in a bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMeta {
    pub key: String,
    pub bucket: String,
    pub size: u64,
    pub content_type: String,
    pub etag: String,
    pub last_modified: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Request to create a new bucket
#[derive(Debug, Deserialize)]
pub struct CreateBucketRequest {
    pub name: String,
    #[serde(default = "default_region")]
    pub region: String,
}

fn default_region() -> String {
    "local".to_string()
}

/// Response for listing objects  
#[derive(Debug, Serialize)]
pub struct ListObjectsResponse {
    pub bucket: String,
    pub prefix: String,
    pub objects: Vec<ObjectMeta>,
    pub common_prefixes: Vec<String>,
    pub is_truncated: bool,
    pub max_keys: u32,
}

/// Query params for listing objects
#[derive(Debug, Deserialize)]
pub struct ListObjectsQuery {
    pub prefix: Option<String>,
    pub delimiter: Option<String>,
    pub max_keys: Option<u32>,
    pub continuation_token: Option<String>,
}

/// Response for listing buckets
#[derive(Debug, Serialize)]
pub struct ListBucketsResponse {
    pub buckets: Vec<Bucket>,
    pub owner: String,
}

/// Stats about storage usage
#[derive(Debug, Serialize)]
pub struct StorageStats {
    pub total_buckets: u64,
    pub total_objects: u64,
    pub total_size: u64,
    pub total_size_human: String,
}
