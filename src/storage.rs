use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::models::{Bucket, ListObjectsResponse, ObjectMeta, StorageStats};

/// File-system backed storage engine
pub struct StorageEngine {
    root: PathBuf,
    /// In-memory bucket metadata index (persisted to disk)
    buckets: RwLock<HashMap<String, Bucket>>,
}

impl StorageEngine {
    /// Initialize the storage engine, creating the root data directory if needed
    pub fn new(root: &str) -> Result<Self, AppError> {
        let root = PathBuf::from(root);
        fs::create_dir_all(&root).map_err(|e| AppError::StorageError(format!("Cannot create data dir: {}", e)))?;

        let engine = Self {
            root: root.clone(),
            buckets: RwLock::new(HashMap::new()),
        };

        // Load existing buckets from disk
        engine.scan_buckets()?;
        Ok(engine)
    }

    /// Scan the root directory for existing bucket folders
    fn scan_buckets(&self) -> Result<(), AppError> {
        let mut buckets = self.buckets.write().unwrap();
        if let Ok(entries) = fs::read_dir(&self.root) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') {
                        continue; // skip hidden dirs
                    }

                    // Try to load metadata
                    let meta_path = entry.path().join(".bucket_meta.json");
                    let bucket = if meta_path.exists() {
                        let data = fs::read_to_string(&meta_path)?;
                        serde_json::from_str::<Bucket>(&data)
                            .unwrap_or_else(|_| self.create_bucket_meta(&name))
                    } else {
                        let b = self.create_bucket_meta(&name);
                        // Save it
                        let json = serde_json::to_string_pretty(&b).unwrap();
                        let _ = fs::write(&meta_path, json);
                        b
                    };

                    buckets.insert(name, bucket);
                }
            }
        }
        Ok(())
    }

    fn create_bucket_meta(&self, name: &str) -> Bucket {
        Bucket {
            name: name.to_string(),
            created_at: Utc::now(),
            region: "local".to_string(),
            object_count: 0,
            total_size: 0,
        }
    }

    fn bucket_path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn object_path(&self, bucket: &str, key: &str) -> PathBuf {
        self.root.join(bucket).join("objects").join(key)
    }

    fn object_meta_path(&self, bucket: &str, key: &str) -> PathBuf {
        let safe_key = key.replace('/', "__SLASH__");
        self.root.join(bucket).join(".meta").join(format!("{}.json", safe_key))
    }

    // ─── Bucket Operations ────────────────────────────────────────

    pub fn validate_bucket_name(name: &str) -> Result<(), AppError> {
        if name.len() < 3 || name.len() > 63 {
            return Err(AppError::InvalidBucketName(
                "Bucket name must be between 3 and 63 characters".to_string(),
            ));
        }
        if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.') {
            return Err(AppError::InvalidBucketName(
                "Bucket name can only contain lowercase letters, numbers, hyphens, and periods".to_string(),
            ));
        }
        if name.starts_with('-') || name.ends_with('-') {
            return Err(AppError::InvalidBucketName(
                "Bucket name cannot start or end with a hyphen".to_string(),
            ));
        }
        Ok(())
    }

    pub fn create_bucket(&self, name: &str, region: &str) -> Result<Bucket, AppError> {
        Self::validate_bucket_name(name)?;

        let mut buckets = self.buckets.write().unwrap();
        if buckets.contains_key(name) {
            return Err(AppError::BucketAlreadyExists(name.to_string()));
        }

        let bucket_dir = self.bucket_path(name);
        fs::create_dir_all(bucket_dir.join("objects"))?;
        fs::create_dir_all(bucket_dir.join(".meta"))?;

        let bucket = Bucket {
            name: name.to_string(),
            created_at: Utc::now(),
            region: region.to_string(),
            object_count: 0,
            total_size: 0,
        };

        // Persist metadata
        let meta_path = bucket_dir.join(".bucket_meta.json");
        let json = serde_json::to_string_pretty(&bucket).unwrap();
        fs::write(&meta_path, json)?;

        buckets.insert(name.to_string(), bucket.clone());
        tracing::info!("Created bucket: {}", name);
        Ok(bucket)
    }

    pub fn list_buckets(&self) -> Vec<Bucket> {
        let buckets = self.buckets.read().unwrap();
        let mut list: Vec<Bucket> = buckets.values().cloned().collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    pub fn get_bucket(&self, name: &str) -> Result<Bucket, AppError> {
        let buckets = self.buckets.read().unwrap();
        buckets
            .get(name)
            .cloned()
            .ok_or_else(|| AppError::BucketNotFound(name.to_string()))
    }

    pub fn delete_bucket(&self, name: &str) -> Result<(), AppError> {
        let mut buckets = self.buckets.write().unwrap();
        if !buckets.contains_key(name) {
            return Err(AppError::BucketNotFound(name.to_string()));
        }

        // Check if empty
        let objects_dir = self.bucket_path(name).join("objects");
        if objects_dir.exists() {
            let count = fs::read_dir(&objects_dir)?.count();
            if count > 0 {
                return Err(AppError::StorageError(
                    "Bucket is not empty. Delete all objects first.".to_string(),
                ));
            }
        }

        fs::remove_dir_all(self.bucket_path(name))?;
        buckets.remove(name);
        tracing::info!("Deleted bucket: {}", name);
        Ok(())
    }

    // ─── Object Operations ────────────────────────────────────────

    pub fn put_object(
        &self,
        bucket: &str,
        key: &str,
        data: &[u8],
        content_type: Option<&str>,
        metadata: HashMap<String, String>,
    ) -> Result<ObjectMeta, AppError> {
        // Check bucket exists
        {
            let buckets = self.buckets.read().unwrap();
            if !buckets.contains_key(bucket) {
                return Err(AppError::BucketNotFound(bucket.to_string()));
            }
        }

        if key.is_empty() || key.len() > 1024 {
            return Err(AppError::InvalidObjectKey(
                "Key must be between 1 and 1024 characters".to_string(),
            ));
        }

        // Determine content type
        let content_type = content_type
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                mime_guess::from_path(key)
                    .first_or_octet_stream()
                    .to_string()
            });

        // Compute ETag (SHA-256 hash)
        let mut hasher = Sha256::new();
        hasher.update(data);
        let etag = format!("\"{}\"", hex::encode(hasher.finalize()));

        // Write the file
        let obj_path = self.object_path(bucket, key);
        if let Some(parent) = obj_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::File::create(&obj_path)?;
        file.write_all(data)?;

        // Write metadata
        let meta = ObjectMeta {
            key: key.to_string(),
            bucket: bucket.to_string(),
            size: data.len() as u64,
            content_type,
            etag,
            last_modified: Utc::now(),
            metadata,
        };

        let meta_path = self.object_meta_path(bucket, key);
        if let Some(parent) = meta_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&meta).unwrap();
        fs::write(&meta_path, json)?;

        // Update bucket stats
        self.update_bucket_stats(bucket)?;

        tracing::info!("Put object: {}/{} ({} bytes)", bucket, key, data.len());
        Ok(meta)
    }

    pub fn get_object(&self, bucket: &str, key: &str) -> Result<(ObjectMeta, Vec<u8>), AppError> {
        // Check bucket exists
        {
            let buckets = self.buckets.read().unwrap();
            if !buckets.contains_key(bucket) {
                return Err(AppError::BucketNotFound(bucket.to_string()));
            }
        }

        let obj_path = self.object_path(bucket, key);
        if !obj_path.exists() {
            return Err(AppError::ObjectNotFound {
                bucket: bucket.to_string(),
                key: key.to_string(),
            });
        }

        let data = fs::read(&obj_path)?;
        let meta = self.get_object_meta(bucket, key)?;

        Ok((meta, data))
    }

    pub fn get_object_meta(&self, bucket: &str, key: &str) -> Result<ObjectMeta, AppError> {
        let meta_path = self.object_meta_path(bucket, key);
        if !meta_path.exists() {
            // Try to reconstruct metadata from file
            let obj_path = self.object_path(bucket, key);
            if !obj_path.exists() {
                return Err(AppError::ObjectNotFound {
                    bucket: bucket.to_string(),
                    key: key.to_string(),
                });
            }

            let file_meta = fs::metadata(&obj_path)?;
            let content_type = mime_guess::from_path(key)
                .first_or_octet_stream()
                .to_string();

            let mut hasher = Sha256::new();
            hasher.update(&fs::read(&obj_path)?);
            let etag = format!("\"{}\"", hex::encode(hasher.finalize()));

            return Ok(ObjectMeta {
                key: key.to_string(),
                bucket: bucket.to_string(),
                size: file_meta.len(),
                content_type,
                etag,
                last_modified: Utc::now(),
                metadata: HashMap::new(),
            });
        }

        let json = fs::read_to_string(&meta_path)?;
        serde_json::from_str(&json)
            .map_err(|e| AppError::StorageError(format!("Corrupt metadata: {}", e)))
    }

    pub fn delete_object(&self, bucket: &str, key: &str) -> Result<(), AppError> {
        {
            let buckets = self.buckets.read().unwrap();
            if !buckets.contains_key(bucket) {
                return Err(AppError::BucketNotFound(bucket.to_string()));
            }
        }

        let obj_path = self.object_path(bucket, key);
        if !obj_path.exists() {
            return Err(AppError::ObjectNotFound {
                bucket: bucket.to_string(),
                key: key.to_string(),
            });
        }

        fs::remove_file(&obj_path)?;

        // Remove metadata
        let meta_path = self.object_meta_path(bucket, key);
        if meta_path.exists() {
            fs::remove_file(&meta_path)?;
        }

        // Clean up empty parent directories inside objects/
        let objects_root = self.bucket_path(bucket).join("objects");
        if let Some(parent) = obj_path.parent() {
            Self::cleanup_empty_dirs(parent, &objects_root);
        }

        self.update_bucket_stats(bucket)?;
        tracing::info!("Deleted object: {}/{}", bucket, key);
        Ok(())
    }

    fn cleanup_empty_dirs(dir: &Path, stop_at: &Path) {
        let mut current = dir.to_path_buf();
        while current != stop_at.to_path_buf() {
            if let Ok(entries) = fs::read_dir(&current) {
                if entries.count() == 0 {
                    let _ = fs::remove_dir(&current);
                } else {
                    break;
                }
            } else {
                break;
            }
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
    }

    pub fn list_objects(
        &self,
        bucket: &str,
        prefix: &str,
        delimiter: Option<&str>,
        max_keys: u32,
    ) -> Result<ListObjectsResponse, AppError> {
        {
            let buckets = self.buckets.read().unwrap();
            if !buckets.contains_key(bucket) {
                return Err(AppError::BucketNotFound(bucket.to_string()));
            }
        }

        let objects_dir = self.bucket_path(bucket).join("objects");
        let mut objects = Vec::new();
        let mut common_prefixes = Vec::new();

        if objects_dir.exists() {
            self.walk_objects(&objects_dir, &objects_dir, bucket, prefix, delimiter, &mut objects, &mut common_prefixes)?;
        }

        // Sort by key
        objects.sort_by(|a, b| a.key.cmp(&b.key));
        common_prefixes.sort();
        common_prefixes.dedup();

        let is_truncated = objects.len() > max_keys as usize;
        objects.truncate(max_keys as usize);

        Ok(ListObjectsResponse {
            bucket: bucket.to_string(),
            prefix: prefix.to_string(),
            objects,
            common_prefixes,
            is_truncated,
            max_keys,
        })
    }

    fn walk_objects(
        &self,
        dir: &Path,
        root: &Path,
        bucket: &str,
        prefix: &str,
        delimiter: Option<&str>,
        objects: &mut Vec<ObjectMeta>,
        common_prefixes: &mut Vec<String>,
    ) -> Result<(), AppError> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                self.walk_objects(&path, root, bucket, prefix, delimiter, objects, common_prefixes)?;
            } else {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");

                if !rel.starts_with(prefix) {
                    continue;
                }

                // Handle delimiter (folder simulation)
                if let Some(delim) = delimiter {
                    let after_prefix = &rel[prefix.len()..];
                    if let Some(pos) = after_prefix.find(delim) {
                        let cp = format!("{}{}{}", prefix, &after_prefix[..pos], delim);
                        common_prefixes.push(cp);
                        continue;
                    }
                }

                // Load metadata
                if let Ok(meta) = self.get_object_meta(bucket, &rel) {
                    objects.push(meta);
                }
            }
        }
        Ok(())
    }

    fn update_bucket_stats(&self, bucket_name: &str) -> Result<(), AppError> {
        let objects_dir = self.bucket_path(bucket_name).join("objects");
        let (count, size) = Self::dir_stats(&objects_dir);

        let mut buckets = self.buckets.write().unwrap();
        if let Some(bucket) = buckets.get_mut(bucket_name) {
            bucket.object_count = count;
            bucket.total_size = size;

            // Persist
            let meta_path = self.bucket_path(bucket_name).join(".bucket_meta.json");
            let json = serde_json::to_string_pretty(&bucket).unwrap();
            let _ = fs::write(&meta_path, json);
        }

        Ok(())
    }

    fn dir_stats(dir: &Path) -> (u64, u64) {
        let mut count = 0u64;
        let mut size = 0u64;

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let (c, s) = Self::dir_stats(&path);
                    count += c;
                    size += s;
                } else {
                    count += 1;
                    size += fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                }
            }
        }

        (count, size)
    }

    pub fn get_stats(&self) -> StorageStats {
        let buckets = self.buckets.read().unwrap();
        let total_buckets = buckets.len() as u64;
        let total_objects: u64 = buckets.values().map(|b| b.object_count).sum();
        let total_size: u64 = buckets.values().map(|b| b.total_size).sum();

        StorageStats {
            total_buckets,
            total_objects,
            total_size,
            total_size_human: human_readable_size(total_size),
        }
    }
}

pub fn human_readable_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}
