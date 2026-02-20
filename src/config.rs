/// Application configuration
#[derive(Clone, Debug)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub data_dir: String,
    pub max_upload_size: usize, // in bytes
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: std::env::var("FREEBUCKET_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("FREEBUCKET_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3210),
            data_dir: std::env::var("FREEBUCKET_DATA_DIR")
                .unwrap_or_else(|_| "./freebucket_data".to_string()),
            max_upload_size: 500 * 1024 * 1024, // 500MB default
        }
    }
}
