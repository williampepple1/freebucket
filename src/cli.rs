use std::collections::HashMap;
use std::path::Path;

use crate::storage::{human_readable_size, StorageEngine};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "freebucket",
    about = "FreeBucket — Local S3-compatible storage bucket service",
    version,
    author
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Data directory for stored objects
    #[arg(long, global = true)]
    pub data_dir: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the web server and dashboard
    Serve {
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Port to listen on
        #[arg(short, long, default_value = "3210")]
        port: u16,
    },

    /// Create a new bucket
    #[command(visible_alias = "mb")]
    MakeBucket {
        /// Name of the bucket to create
        name: String,
        /// Region label
        #[arg(short, long, default_value = "local")]
        region: String,
    },

    /// Remove a bucket (must be empty)
    #[command(visible_alias = "rb")]
    RemoveBucket {
        /// Name of the bucket to delete
        name: String,
    },

    /// List buckets or objects in a bucket
    #[command(visible_alias = "ls")]
    List {
        /// Bucket name (omit to list all buckets)
        bucket: Option<String>,
        /// Filter objects by prefix
        #[arg(short, long)]
        prefix: Option<String>,
    },

    /// Upload a file to a bucket
    #[command(visible_alias = "cp")]
    Put {
        /// Local file path to upload
        source: String,
        /// Destination as bucket/key (e.g. my-bucket/photos/cat.jpg)
        destination: String,
    },

    /// Download an object from a bucket
    Get {
        /// Source as bucket/key
        source: String,
        /// Local file path to save to (defaults to the key filename)
        output: Option<String>,
    },

    /// Delete an object from a bucket
    #[command(visible_alias = "rm")]
    Remove {
        /// Object path as bucket/key
        path: String,
    },

    /// Show storage statistics
    Stats,

    /// Show information about a specific bucket
    Info {
        /// Bucket name
        bucket: String,
    },
}

pub fn run_cli(cli: Cli) {
    let data_dir = cli.data_dir
        .or_else(|| std::env::var("FREEBUCKET_DATA_DIR").ok())
        .unwrap_or_else(|| "./freebucket_data".to_string());

    let storage = match StorageEngine::new(&data_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: Failed to initialize storage at '{}': {:?}", data_dir, e);
            std::process::exit(1);
        }
    };

    match cli.command.unwrap() {
        Commands::Serve { .. } => unreachable!("Serve is handled in main"),

        Commands::MakeBucket { name, region } => {
            match storage.create_bucket(&name, &region) {
                Ok(bucket) => {
                    println!("✓ Bucket '{}' created successfully", bucket.name);
                    println!("  Region:  {}", bucket.region);
                    println!("  Created: {}", bucket.created_at.format("%Y-%m-%d %H:%M:%S"));
                }
                Err(e) => {
                    eprintln!("✗ {}", format_error(&e));
                    std::process::exit(1);
                }
            }
        }

        Commands::RemoveBucket { name } => {
            match storage.delete_bucket(&name) {
                Ok(()) => println!("✓ Bucket '{}' deleted", name),
                Err(e) => {
                    eprintln!("✗ {}", format_error(&e));
                    std::process::exit(1);
                }
            }
        }

        Commands::List { bucket, prefix } => {
            match bucket {
                None => {
                    // List all buckets
                    let buckets = storage.list_buckets();
                    if buckets.is_empty() {
                        println!("No buckets found. Create one with: freebucket make-bucket <name>");
                        return;
                    }
                    println!("{:<30} {:>8} {:>12}  {}", "BUCKET", "OBJECTS", "SIZE", "CREATED");
                    println!("{}", "─".repeat(70));
                    for b in &buckets {
                        println!(
                            "{:<30} {:>8} {:>12}  {}",
                            b.name,
                            b.object_count,
                            human_readable_size(b.total_size),
                            b.created_at.format("%Y-%m-%d %H:%M")
                        );
                    }
                    println!("{}", "─".repeat(70));
                    println!("{} bucket(s)", buckets.len());
                }
                Some(bucket_name) => {
                    // List objects in bucket
                    let prefix_str = prefix.as_deref().unwrap_or("");
                    match storage.list_objects(&bucket_name, prefix_str, None, 1000) {
                        Ok(result) => {
                            if result.objects.is_empty() {
                                println!("No objects in bucket '{}'{}", bucket_name,
                                    if !prefix_str.is_empty() { format!(" with prefix '{}'", prefix_str) } else { String::new() });
                                return;
                            }
                            println!("{:<50} {:>12}  {}", "KEY", "SIZE", "LAST MODIFIED");
                            println!("{}", "─".repeat(85));
                            for obj in &result.objects {
                                println!(
                                    "{:<50} {:>12}  {}",
                                    if obj.key.len() > 48 {
                                        format!("…{}", &obj.key[obj.key.len()-47..])
                                    } else {
                                        obj.key.clone()
                                    },
                                    human_readable_size(obj.size),
                                    obj.last_modified.format("%Y-%m-%d %H:%M")
                                );
                            }
                            println!("{}", "─".repeat(85));
                            println!("{} object(s)", result.objects.len());
                        }
                        Err(e) => {
                            eprintln!("✗ {}", format_error(&e));
                            std::process::exit(1);
                        }
                    }
                }
            }
        }

        Commands::Put { source, destination } => {
            // Parse destination as bucket/key
            let (bucket, key) = match destination.find('/') {
                Some(pos) => (&destination[..pos], &destination[pos + 1..]),
                None => {
                    // If no key given, use the filename
                    let filename = Path::new(&source)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "upload".to_string());
                    // Can't borrow destination and filename at the same time easily,
                    // so handle it differently
                    let data = match std::fs::read(&source) {
                        Ok(d) => d,
                        Err(e) => {
                            eprintln!("✗ Cannot read file '{}': {}", source, e);
                            std::process::exit(1);
                        }
                    };
                    match storage.put_object(&destination, &filename, &data, None, HashMap::new()) {
                        Ok(meta) => {
                            println!("✓ Uploaded '{}' → {}/{}", source, destination, filename);
                            println!("  Size: {}  ETag: {}", human_readable_size(meta.size), meta.etag);
                        }
                        Err(e) => {
                            eprintln!("✗ {}", format_error(&e));
                            std::process::exit(1);
                        }
                    }
                    return;
                }
            };

            let data = match std::fs::read(&source) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("✗ Cannot read file '{}': {}", source, e);
                    std::process::exit(1);
                }
            };

            match storage.put_object(bucket, key, &data, None, HashMap::new()) {
                Ok(meta) => {
                    println!("✓ Uploaded '{}' → {}/{}", source, bucket, key);
                    println!("  Size: {}  ETag: {}", human_readable_size(meta.size), meta.etag);
                }
                Err(e) => {
                    eprintln!("✗ {}", format_error(&e));
                    std::process::exit(1);
                }
            }
        }

        Commands::Get { source, output } => {
            let (bucket, key) = match source.find('/') {
                Some(pos) => (&source[..pos], &source[pos + 1..]),
                None => {
                    eprintln!("✗ Source must be in format: bucket/key");
                    std::process::exit(1);
                }
            };

            match storage.get_object(bucket, key) {
                Ok((meta, data)) => {
                    let out_path = output.unwrap_or_else(|| {
                        key.rsplit('/').next().unwrap_or(key).to_string()
                    });

                    match std::fs::write(&out_path, &data) {
                        Ok(()) => {
                            println!("✓ Downloaded {}/{} → '{}'", bucket, key, out_path);
                            println!("  Size: {}  Type: {}", human_readable_size(meta.size), meta.content_type);
                        }
                        Err(e) => {
                            eprintln!("✗ Cannot write to '{}': {}", out_path, e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("✗ {}", format_error(&e));
                    std::process::exit(1);
                }
            }
        }

        Commands::Remove { path } => {
            let (bucket, key) = match path.find('/') {
                Some(pos) => (&path[..pos], &path[pos + 1..]),
                None => {
                    eprintln!("✗ Path must be in format: bucket/key");
                    std::process::exit(1);
                }
            };

            match storage.delete_object(bucket, key) {
                Ok(()) => println!("✓ Deleted {}/{}", bucket, key),
                Err(e) => {
                    eprintln!("✗ {}", format_error(&e));
                    std::process::exit(1);
                }
            }
        }

        Commands::Stats => {
            let stats = storage.get_stats();
            println!("FreeBucket Storage Statistics");
            println!("{}", "─".repeat(35));
            println!("  Buckets:  {}", stats.total_buckets);
            println!("  Objects:  {}", stats.total_objects);
            println!("  Size:     {}", stats.total_size_human);
            println!("  Data dir: {}", data_dir);
        }

        Commands::Info { bucket } => {
            match storage.get_bucket(&bucket) {
                Ok(b) => {
                    println!("Bucket: {}", b.name);
                    println!("{}", "─".repeat(35));
                    println!("  Region:   {}", b.region);
                    println!("  Objects:  {}", b.object_count);
                    println!("  Size:     {}", human_readable_size(b.total_size));
                    println!("  Created:  {}", b.created_at.format("%Y-%m-%d %H:%M:%S"));
                }
                Err(e) => {
                    eprintln!("✗ {}", format_error(&e));
                    std::process::exit(1);
                }
            }
        }
    }
}

fn format_error(e: &crate::error::AppError) -> String {
    match e {
        crate::error::AppError::BucketNotFound(name) => format!("Bucket '{}' not found", name),
        crate::error::AppError::BucketAlreadyExists(name) => format!("Bucket '{}' already exists", name),
        crate::error::AppError::ObjectNotFound { bucket, key } => format!("Object '{}/{}' not found", bucket, key),
        crate::error::AppError::InvalidBucketName(msg) => format!("Invalid bucket name: {}", msg),
        crate::error::AppError::InvalidObjectKey(msg) => format!("Invalid key: {}", msg),
        crate::error::AppError::StorageError(msg) => format!("Storage error: {}", msg),
        crate::error::AppError::IoError(e) => format!("I/O error: {}", e),
    }
}
