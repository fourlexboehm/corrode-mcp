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
    if diff_str.trim().is_empty() {
        return Ok(original.to_string());
    }

    // Parse the diff and apply the changes
    let lines: Vec<&str> = diff_str.lines().collect();
    let mut result = original.to_string();

    // Better input validation
    if !diff_str.contains("@@ ") {
        return Err(format!("Invalid diff format: Missing hunk header (should contain '@@ '). Diff content:\n{}", diff_str));
    }

    // If there are no lines in the diff, return the original content
    if lines.is_empty() {
        return Ok(result);
    }
    
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        
        // Skip empty lines
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        
        if line.starts_with("@@ ") {
            // Found a hunk header
            match parse_hunk_header(line) {
                Ok((start, _count)) => {
                    // Find the content of the hunk
                    let mut j = i + 1;
                    let mut to_remove = Vec::new();
                    let mut to_add = Vec::new();
                    let mut context_lines = Vec::new();

                    while j < lines.len() && !lines[j].starts_with("@@ ") {
                        let line = lines[j];
                        if line.starts_with('-') {
                            to_remove.push(&line[1..]);
                        } else if line.starts_with('+') {
                            to_add.push(&line[1..]);
                        } else if !line.trim().is_empty() {
                            // This is a context line (not starting with + or -)
                            context_lines.push(line);
                        }
                        j += 1;
                    }

                    // Apply the changes
                    let original_lines: Vec<&str> = result.lines().collect();
                    let mut new_lines = Vec::new();

                    // Validate that the start line is within bounds
                    if start > original_lines.len() + 1 {
                        return Err(format!(
                            "Hunk start line {} is beyond the end of the file (which has {} lines). Check your line numbers.",
                            start, original_lines.len()
                        ));
                    }

                    // Copy lines before the hunk
                    for idx in 0..(start - 1) {
                        if idx < original_lines.len() {
                            new_lines.push(original_lines[idx].to_string());
                        }
                    }

                    // Add the new lines
                    for add_line in &to_add {
                        new_lines.push(add_line.to_string());
                    }

                    // Skip the lines that were removed
                    let mut original_idx = start - 1;
                    let mut removed_count = 0;

                    // Verify that lines to be removed match the original
                    while removed_count < to_remove.len() && original_idx < original_lines.len() {
                        if original_lines[original_idx] != to_remove[removed_count] {
                            // Enhanced error message with context
                            let expected = to_remove[removed_count];
                            let found = original_lines[original_idx];
                            
                            // Create a context visualization
                            let mut context = format!("Expected to remove (line {}): '{}'\nFound in file: '{}'",
                                                     original_idx + 1, expected, found);
                            
                            // Add surrounding lines for context if available
                            if original_idx > 0 {
                                context.push_str(&format!("\nPrevious line in file: '{}'",
                                                        original_lines[original_idx - 1]));
                            }
                            
                            if original_idx + 1 < original_lines.len() {
                                context.push_str(&format!("\nNext line in file: '{}'",
                                                        original_lines[original_idx + 1]));
                            }
                            
                            return Err(format!(
                                "Line mismatch at line {}:\n{}\n\nPlease make sure your diff matches the exact content of the file.",
                                original_idx + 1, context
                            ));
                        }
                        original_idx += 1;
                        removed_count += 1;
                    }

                    if removed_count < to_remove.len() {
                        return Err(format!(
                            "Unexpected end of file while trying to remove line {}. There are only {} lines in the file, but the diff tries to remove '{}' at line {}.",
                            start + removed_count,
                            original_lines.len(),
                            to_remove[removed_count],
                            start + removed_count
                        ));
                    }

                    // Copy the rest of the original lines
                    for idx in original_idx..original_lines.len() {
                        new_lines.push(original_lines[idx].to_string());
                    }

                    result = new_lines.join("\n");
                    
                    // Preserve trailing newline if original had one
                    if original.ends_with('\n') && !result.ends_with('\n') {
                        result.push('\n');
                    }

                    i = j; // Move past the processed hunk
                }
                Err(e) => {
                    return Err(format!("Failed to parse hunk header '{}': {}. Ensure the format is '@@ -start,count +start,count @@'", line, e));
                }
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
    
    // Validate basic format
    if !header.starts_with("@@ ") || !header.contains(" @@") {
        return Err(format!("Invalid hunk header format (should be '@@ -start,count +start,count @@'): '{}'", header));
    }
    
    let parts: Vec<&str> = header.split_whitespace().collect();
    
    // We need at least three parts: "@@", "-1,5", "+1,6", "@@"
    if parts.len() < 3 {
        return Err(format!("Invalid hunk header format, insufficient parts: '{}'", header));
    }
    
    // Find the part that starts with "-" (there could be whitespace)
    let old_range_part = match parts.iter().find(|part| part.starts_with('-')) {
        Some(part) => *part,
        None => return Err(format!("Missing old range (-start,count) in hunk header: '{}'", header))
    };
    
    let range_str = &old_range_part[1..]; // "1,5"
    
    // Handle both formats: single number or start,count
    let range_parts: Vec<&str> = range_str.split(',').collect();
    
    let start = match range_parts.get(0) {
        Some(s) if !s.is_empty() => {
            match s.parse::<usize>() {
                Ok(num) => num,
                Err(e) => return Err(format!("Failed to parse start line '{}': {}", s, e))
            }
        },
        _ => return Err(format!("Missing or invalid start line number in hunk header: '{}'", header))
    };
    
    let count = if range_parts.len() > 1 && !range_parts[1].is_empty() {
        match range_parts[1].parse::<usize>() {
            Ok(num) => num,
            Err(e) => return Err(format!("Failed to parse line count '{}': {}", range_parts[1], e))
        }
    } else {
        // Default count is 1 if not specified or empty
        1
    };
    
    Ok((start, count))
}
