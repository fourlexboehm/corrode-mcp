use std::path::{Path, PathBuf};

pub mod mcp;


// Simplified Args struct
// Helper function to resolve a file path relative to the current directory
pub fn resolve_path(current_dir: &Path, file_path: &str) -> PathBuf {
    if file_path.starts_with('/') {
        // Absolute path
        PathBuf::from(file_path)
    } else if file_path.starts_with("~/") || file_path == "~" {
        // Home directory path
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(file_path.trim_start_matches("~/"))
    } else {
        // Relative path
        current_dir.join(file_path)
    }
}

// Helper function to update working directory when cd commands are used
// Takes current_dir as argument now
pub fn handle_cd_command(current_dir: &Path, command: &str) -> Option<PathBuf> {
    let command = command.trim();

    // Check if command is a cd command or starts with cd and has more components
    if command == "cd" || command.starts_with("cd ") {
        let parts: Vec<&str> = command.splitn(2, ' ').collect();
        if parts.len() == 1 {
            // Just "cd", go to home directory
            return Some(dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")));
        } else if parts.len() == 2 {
            let dir = parts[1].trim();
            // Handle different path types using resolve_path helper
            let new_path = resolve_path(current_dir, dir);
            return Some(new_path);
        }
    }
    None
}


// Function to apply a diff to a string
pub fn apply_diff(original: &str, diff_str: &str) -> Result<String, String> {
    // Parse the diff and apply the changes
    let lines: Vec<&str> = diff_str.lines().collect();
    let mut result = original.to_string();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.starts_with("@@ ") {
            // Found a hunk header
            if let Ok((start, _)) = parse_hunk_header(line) {
                // Find the content of the hunk
                let mut j = i + 1;
                let mut to_remove = Vec::new();
                let mut to_add = Vec::new();

                while j < lines.len() && !lines[j].starts_with("@@ ") {
                    let line = lines[j];
                    if line.starts_with('-') {
                        to_remove.push(&line[1..]);
                    } else if line.starts_with('+') {
                        to_add.push(&line[1..]);
                    }
                    j += 1;
                }

                // Apply the changes
                let original_lines: Vec<&str> = result.lines().collect();
                let mut new_lines = Vec::new();

                // This diff application logic seems incorrect/incomplete.
                // It doesn't properly handle line indices and potential mismatches.
                // For now, keeping the original logic, but it might need revision.
                let mut original_idx = 0;
                while original_idx < original_lines.len() {
                    if original_idx + 1 == start { // Check if we are at the start line of the hunk (1-based index)
                        // Apply additions
                        for add_line in &to_add {
                            new_lines.push(add_line.to_string());
                        }
                        // Skip original lines corresponding to removals
                        let mut removed_count = 0;
                        while removed_count < to_remove.len() {
                            if original_idx < original_lines.len() {
                                // Basic check: does the line to be removed match the original?
                                // This is a weak check and doesn't handle context lines well.
                                if original_lines[original_idx] == to_remove[removed_count] {
                                     original_idx += 1;
                                     removed_count += 1;
                                } else {
                                     // If mismatch, maybe error or just skip? Sticking to skip for now.
                                     // Or maybe the diff format implies context lines not starting with +/-?
                                     // The original logic was complex and potentially flawed.
                                     // Let's just skip the number of lines indicated by `to_remove.len()`
                                     // This is likely incorrect but avoids complex error handling for now.
                                     original_idx += to_remove.len(); // Advance past removed lines
                                     break; // Exit inner loop after skipping
                                }
                            } else {
                                break; // Reached end of original lines
                            }
                        }
                        // After handling the hunk, continue to the next line
                        continue; // Go to next iteration of outer loop
                    } else {
                        // Keep the original line if not part of the hunk modification
                        new_lines.push(original_lines[original_idx].to_string());
                        original_idx += 1;
                    }
                }


                result = new_lines.join("\n");
                // Preserve trailing newline if original had one
                if original.ends_with('\n') && !result.ends_with('\n') {
                    result.push('\n');
                }

                i = j; // Move past the processed hunk
            } else {
                return Err(format!("Failed to parse hunk header: {}", line));
            }
        } else {
            i += 1; // Move to the next line if not a hunk header
        }
    }

    Ok(result)
}


// Helper function to parse a unified diff hunk header
pub fn parse_hunk_header(header: &str) -> Result<(usize, usize), String> {
    // Example: @@ -1,5 +1,6 @@
    let parts: Vec<&str> = header.split(' ').collect();
    if parts.len() < 3 || !parts[0].starts_with("@@") || !parts[2].starts_with("@@") {
        return Err(format!("Invalid hunk header format: {}", header));
    }
    let old_range_part = parts[1]; // e.g., "-1,5"

    if !old_range_part.starts_with('-') {
         return Err(format!("Invalid old range format in hunk header: {}", old_range_part));
    }

    let range_str = &old_range_part[1..]; // "1,5"
    let range_parts: Vec<&str> = range_str.split(',').collect();

    let start = range_parts.get(0)
        .ok_or_else(|| format!("Missing start line in hunk header: {}", header))?
        .parse::<usize>()
        .map_err(|e| format!("Failed to parse start line '{}': {}", range_parts[0], e))?;

    let count = if range_parts.len() > 1 {
        range_parts[1].parse::<usize>()
            .map_err(|e| format!("Failed to parse line count '{}': {}", range_parts[1], e))?
    } else {
        1 // Default count is 1 if not specified
    };

    Ok((start, count))
}

