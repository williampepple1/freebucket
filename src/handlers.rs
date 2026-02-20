use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Router,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json,
};
use serde_json::json;

use crate::error::AppError;
use crate::models::*;

use crate::AppState;

type AppResult<T> = Result<T, AppError>;

// ─── REST API Routes ─────────────────────────────────────────────

pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Stats
        .route("/stats", get(get_stats))
        // Bucket operations
        .route("/buckets", get(list_buckets).post(create_bucket))
        .route("/buckets/:bucket", get(get_bucket).delete(delete_bucket))
        // Object listing
        .route("/buckets/:bucket/objects", get(list_objects))
        // Upload via multipart
        .route("/buckets/:bucket/upload", post(upload_object))
}

/// Wildcard routes that MUST be registered at top level (cannot be nested in Axum 0.7)
pub fn api_wildcard_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/object/*path", get(get_object).delete(delete_object))
}

// ─── S3-Compatible Routes ─────────────────────────────────────────

pub fn s3_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/s3", get(s3_list_buckets))
        .route("/s3/:bucket", get(s3_list_objects).put(s3_create_bucket).delete(s3_delete_bucket))
}

/// S3 wildcard routes — must be registered at top level
pub fn s3_wildcard_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/s3/obj/*path", get(s3_get_object).put(s3_put_object).delete(s3_delete_object))
}

// ─── Stats ───────────────────────────────────────────────────────

async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.storage.get_stats())
}

// ─── Bucket Handlers ─────────────────────────────────────────────

async fn list_buckets(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let buckets = state.storage.list_buckets();
    Json(ListBucketsResponse {
        buckets,
        owner: "freebucket-local".to_string(),
    })
}

async fn create_bucket(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateBucketRequest>,
) -> AppResult<impl IntoResponse> {
    let bucket = state.storage.create_bucket(&body.name, &body.region)?;
    Ok((StatusCode::CREATED, Json(bucket)))
}

async fn get_bucket(
    State(state): State<Arc<AppState>>,
    Path(bucket): Path<String>,
) -> AppResult<impl IntoResponse> {
    let bucket = state.storage.get_bucket(&bucket)?;
    Ok(Json(bucket))
}

async fn delete_bucket(
    State(state): State<Arc<AppState>>,
    Path(bucket): Path<String>,
) -> AppResult<impl IntoResponse> {
    state.storage.delete_bucket(&bucket)?;
    Ok(StatusCode::NO_CONTENT)
}

// ─── Object Handlers ─────────────────────────────────────────────

async fn list_objects(
    State(state): State<Arc<AppState>>,
    Path(bucket): Path<String>,
    Query(query): Query<ListObjectsQuery>,
) -> AppResult<impl IntoResponse> {
    let prefix = query.prefix.as_deref().unwrap_or("");
    let delimiter = query.delimiter.as_deref();
    let max_keys = query.max_keys.unwrap_or(1000);

    let response = state.storage.list_objects(&bucket, prefix, delimiter, max_keys)?;
    Ok(Json(response))
}

/// Parse a catch-all path like "mybucket/path/to/key.txt" into (bucket, key)
fn parse_bucket_key(path: &str) -> Result<(&str, &str), AppError> {
    let path = path.strip_prefix('/').unwrap_or(path);
    match path.find('/') {
        Some(pos) => Ok((&path[..pos], &path[pos + 1..])),
        None => Err(AppError::InvalidObjectKey(
            "Path must be in the format: {bucket}/{key}".to_string(),
        )),
    }
}

async fn get_object(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> AppResult<Response> {
    let (bucket, key) = parse_bucket_key(&path)?;
    let (meta, data) = state.storage.get_object(bucket, key)?;

    let mut headers = HeaderMap::new();
    headers.insert("content-type", meta.content_type.parse().unwrap());
    headers.insert("etag", meta.etag.parse().unwrap());
    headers.insert(
        "last-modified",
        meta.last_modified.to_rfc2822().parse().unwrap(),
    );
    headers.insert("content-length", meta.size.to_string().parse().unwrap());

    Ok((StatusCode::OK, headers, data).into_response())
}

async fn delete_object(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> AppResult<impl IntoResponse> {
    let (bucket, key) = parse_bucket_key(&path)?;
    state.storage.delete_object(bucket, key)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn upload_object(
    State(state): State<Arc<AppState>>,
    Path(bucket): Path<String>,
    mut multipart: Multipart,
) -> AppResult<impl IntoResponse> {
    let mut uploaded = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::StorageError(format!("Multipart error: {}", e))
    })? {
        let file_name = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("upload-{}", uuid::Uuid::new_v4()));

        let content_type = field.content_type().map(|s| s.to_string());
        let data = field.bytes().await.map_err(|e| {
            AppError::StorageError(format!("Failed to read upload data: {}", e))
        })?;

        let meta = state.storage.put_object(
            &bucket,
            &file_name,
            &data,
            content_type.as_deref(),
            HashMap::new(),
        )?;

        uploaded.push(meta);
    }

    Ok((StatusCode::CREATED, Json(json!({
        "uploaded": uploaded.len(),
        "objects": uploaded
    }))))
}

// ─── S3-Compatible Handlers ──────────────────────────────────────

async fn s3_list_buckets(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let buckets = state.storage.list_buckets();
    // Return XML-like JSON for simplicity (real S3 uses XML)
    Json(json!({
        "ListAllMyBucketsResult": {
            "Buckets": buckets.iter().map(|b| json!({
                "Name": b.name,
                "CreationDate": b.created_at.to_rfc3339()
            })).collect::<Vec<_>>(),
            "Owner": {
                "DisplayName": "freebucket-local",
                "ID": "freebucket"
            }
        }
    }))
}

async fn s3_create_bucket(
    State(state): State<Arc<AppState>>,
    Path(bucket): Path<String>,
) -> AppResult<impl IntoResponse> {
    state.storage.create_bucket(&bucket, "local")?;
    Ok(StatusCode::OK)
}

async fn s3_delete_bucket(
    State(state): State<Arc<AppState>>,
    Path(bucket): Path<String>,
) -> AppResult<impl IntoResponse> {
    state.storage.delete_bucket(&bucket)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn s3_list_objects(
    State(state): State<Arc<AppState>>,
    Path(bucket): Path<String>,
    Query(query): Query<ListObjectsQuery>,
) -> AppResult<impl IntoResponse> {
    let prefix = query.prefix.as_deref().unwrap_or("");
    let delimiter = query.delimiter.as_deref();
    let max_keys = query.max_keys.unwrap_or(1000);

    let response = state.storage.list_objects(&bucket, prefix, delimiter, max_keys)?;

    Ok(Json(json!({
        "ListBucketResult": {
            "Name": response.bucket,
            "Prefix": response.prefix,
            "MaxKeys": response.max_keys,
            "IsTruncated": response.is_truncated,
            "Contents": response.objects.iter().map(|o| json!({
                "Key": o.key,
                "Size": o.size,
                "LastModified": o.last_modified.to_rfc3339(),
                "ETag": o.etag,
                "StorageClass": "STANDARD"
            })).collect::<Vec<_>>(),
            "CommonPrefixes": response.common_prefixes.iter().map(|cp| json!({
                "Prefix": cp
            })).collect::<Vec<_>>()
        }
    })))
}

async fn s3_get_object(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> AppResult<Response> {
    get_object(State(state), Path(path)).await
}

async fn s3_put_object(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> AppResult<impl IntoResponse> {
    let (bucket, key) = parse_bucket_key(&path)?;
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Extract custom metadata from x-amz-meta-* headers
    let mut metadata = HashMap::new();
    for (hdr_key, value) in headers.iter() {
        if let Some(meta_key) = hdr_key.as_str().strip_prefix("x-amz-meta-") {
            if let Ok(val) = value.to_str() {
                metadata.insert(meta_key.to_string(), val.to_string());
            }
        }
    }

    let meta = state.storage.put_object(
        bucket,
        key,
        &body,
        content_type.as_deref(),
        metadata,
    )?;

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert("etag", meta.etag.parse().unwrap());

    Ok((StatusCode::OK, resp_headers))
}

async fn s3_delete_object(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> AppResult<impl IntoResponse> {
    let (bucket, key) = parse_bucket_key(&path)?;
    state.storage.delete_object(bucket, key)?;
    Ok(StatusCode::NO_CONTENT)
}
