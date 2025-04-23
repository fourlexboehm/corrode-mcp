use anyhow::{Result, anyhow};
use std::path::Path;
use yek::{config::YekConfig, serialize_repo};

const TOKEN_LIMIT: usize = 50000;

/// Uses yek's internal API to read file content with a fixed 50k token limit
pub async fn read_with_yek(path: &Path) -> Result<String> {
    // Check if path exists
    if !path.exists() {
        return Err(anyhow!("Path '{}' does not exist", path.display()));
    }

    // Get absolute path
    let path_str = path.canonicalize()?.display().to_string();

    let mut full_config = YekConfig::init_config();
    full_config.input_paths = vec![path_str];
    full_config.token_mode = true;
    full_config.tokens = TOKEN_LIMIT.to_string();
    full_config.stream = true; // Stream to string instead of file

    // Use a memory-bounded approach
    let result = match serialize_repo(&full_config) {
        Ok((content, processed_files)) => {
            // Calculate total bytes (for debugging purposes)
            let _total_bytes: usize = processed_files.iter().map(|file| file.content.len()).sum();

            // Force cleanup of processed_files which might be holding large strings
            drop(processed_files);

            Ok(content)
        }
        Err(e) => Err(anyhow!("Failed to read with yek: {}", e)),
    };

    // Explicitly trigger garbage collection
    std::mem::drop(full_config);
    result
}
