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
                let mut lines_processed_in_hunk = 0; // Track lines consumed from original within the hunk
                while original_idx < original_lines.len() {
                    // Check if the current original line is the start of the hunk
                    if original_idx + 1 == start && lines_processed_in_hunk == 0 {
                        // Apply additions
                        for add_line in &to_add {
                            new_lines.push(add_line.to_string());
                        }

                        // Verify and skip original lines corresponding to removals
                        let mut removed_count = 0;
                        while removed_count < to_remove.len() {
                            if original_idx < original_lines.len() {
                                // Basic check: does the line to be removed match the original?
                                if original_lines[original_idx] == to_remove[removed_count] {
                                    original_idx += 1; // Consume the original line
                                    removed_count += 1;
                                    lines_processed_in_hunk += 1;
                                } else {
                                    // If the lines don't match, return a helpful error
                                    return Err(format!(
                                        "Line mismatch at original line {}:\nExpected to remove: '{}'\nFound: '{}'",
                                        start + removed_count, // Use start + removed_count for original line number
                                        to_remove[removed_count],
                                        original_lines[original_idx]
                                    ));
                                }
                            } else {
                                // Reached end of original lines unexpectedly during removal check
                                return Err(format!(
                                    "Unexpected end of file while trying to remove line {} ('{}')",
                                    start + removed_count,
                                    to_remove[removed_count]
                                ));
                            }
                        }
                        // If no lines were removed, we still need to advance original_idx past the context lines
                        // covered by the hunk header's original count, if that count was > 0.
                        // However, the simple unified diff format used here doesn't provide enough context
                        // to reliably skip context lines. A more robust diff library is needed for that.
                        // For this specific implementation, we assume the hunk only covers removed lines.
                        // If lines_processed_in_hunk is still 0 (only additions), we don't advance original_idx here.

                    } else {
                        // Keep the original line if not part of the hunk modification
                        new_lines.push(original_lines[original_idx].to_string());
                        original_idx += 1;
                    }
                    // Reset lines_processed_in_hunk if we moved past the hunk's influence
                    // This simple logic might still be insufficient for complex diffs.
                    if lines_processed_in_hunk > 0 && original_idx + 1 > start + lines_processed_in_hunk {
                         lines_processed_in_hunk = 0;
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

