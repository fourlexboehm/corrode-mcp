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


// Function to apply a diff to a string (Corrected Logic)
pub fn apply_diff(original: &str, diff_str: &str) -> Result<String, String> {
    if diff_str.trim().is_empty() {
        return Ok(original.to_string());
    }

    // Basic validation moved here
    if !diff_str.contains("@@ ") {
        return Err(format!("Invalid diff format: Missing hunk header (should contain '@@ '). Diff content:\n{}", diff_str));
    }

    let diff_lines: Vec<&str> = diff_str.lines().collect();
    let original_lines: Vec<&str> = original.lines().collect();
    let mut new_lines: Vec<String> = Vec::new();
    let mut current_original_line_idx: usize = 0;
    let mut diff_line_idx: usize = 0;

    while diff_line_idx < diff_lines.len() {
        let line = diff_lines[diff_line_idx];

        // Skip empty lines or lines that are not hunk headers before the first hunk
        if !line.starts_with("@@ ") {
             if line.trim().is_empty() {
                 diff_line_idx += 1;
                 continue;
             } else {
                 // Handle potential leading text before the first hunk if necessary, or error out
                 return Err(format!("Unexpected content before first hunk header: '{}'", line));
             }
        }

        // Found a hunk header
        let (start, _count) = match parse_hunk_header(line) {
             Ok(res) => res,
             Err(e) => return Err(format!("Failed to parse hunk header '{}': {}. Ensure the format is '@@ -start,count +start,count @@'", line, e)),
        };
        diff_line_idx += 1; // Move past header

        let hunk_start_original_idx = start.saturating_sub(1); // 0-based index

        // Copy lines from original content *up to* where this hunk starts
        while current_original_line_idx < hunk_start_original_idx {
            if current_original_line_idx < original_lines.len() {
                new_lines.push(original_lines[current_original_line_idx].to_string());
            } else {
                 return Err(format!("Error applying diff: Trying to copy original line {} but file only has {} lines.", current_original_line_idx + 1, original_lines.len()));
            }
            current_original_line_idx += 1;
        }

        // Process lines within the hunk
        while diff_line_idx < diff_lines.len() && !diff_lines[diff_line_idx].starts_with("@@ ") {
            let hunk_line = diff_lines[diff_line_idx];
            diff_line_idx += 1;

            if hunk_line.starts_with('+') {
                // Add this line to the result
                new_lines.push(hunk_line[1..].to_string());
            } else if hunk_line.starts_with('-') {
                // Verify removal line matches original and skip it (consume original line)
                if current_original_line_idx >= original_lines.len() {
                     return Err(format!("Error applying diff: Trying to remove line corresponding to original line {} but file only has {} lines.", current_original_line_idx + 1, original_lines.len()));
                }
                let line_to_remove = &hunk_line[1..];
                if original_lines[current_original_line_idx] != line_to_remove {
                    let expected = line_to_remove;
                    let found = original_lines[current_original_line_idx];
                    let mut context = format!("Expected to remove (line {}): '{}'\nFound in file: '{}'", current_original_line_idx + 1, expected, found);
                    if current_original_line_idx > 0 { context.push_str(&format!("\nPrevious line in file: '{}'", original_lines[current_original_line_idx - 1])); }
                    if current_original_line_idx + 1 < original_lines.len() { context.push_str(&format!("\nNext line in file: '{}'", original_lines[current_original_line_idx + 1])); }
                    return Err(format!("Line mismatch at line {}:\n{}\n\nPlease make sure your diff matches the exact content of the file.", current_original_line_idx + 1, context));
                }
                current_original_line_idx += 1; // Consume the original line that was removed
            } else {
                // Context line: verify and add it (consume original line)
                 if current_original_line_idx >= original_lines.len() {
                     return Err(format!("Error applying diff: Trying to process context line corresponding to original line {} but file only has {} lines.", current_original_line_idx + 1, original_lines.len()));
                }
                let context_line = if hunk_line.starts_with(' ') { &hunk_line[1..] } else { hunk_line };
                 if original_lines[current_original_line_idx] != context_line {
                    let expected = context_line;
                    let found = original_lines[current_original_line_idx];
                    let mut context = format!("Context mismatch (line {}): Expected '{}', Found '{}'", current_original_line_idx + 1, expected, found);
                    if current_original_line_idx > 0 { context.push_str(&format!("\nPrevious line in file: '{}'", original_lines[current_original_line_idx - 1])); }
                    if current_original_line_idx + 1 < original_lines.len() { context.push_str(&format!("\nNext line in file: '{}'", original_lines[current_original_line_idx + 1])); }
                    return Err(format!("Context mismatch at line {}:\n{}\n\nPlease make sure your diff matches the exact content of the file.", current_original_line_idx + 1, context));
                }
                new_lines.push(original_lines[current_original_line_idx].to_string());
                current_original_line_idx += 1; // Consume the original line for context
            }
        }
        // End of hunk processing loop
    }
    // End of diff lines processing loop

    // Copy any remaining lines from the original content after the last hunk
    while current_original_line_idx < original_lines.len() {
        new_lines.push(original_lines[current_original_line_idx].to_string());
        current_original_line_idx += 1;
    }

    let mut result = new_lines.join("\n");
    // Preserve trailing newline if original had one and result doesn't
    if original.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
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



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_diff_hello_example() {
        let original = r#"fn main() {
    println!("Hello, world!");
}"#;

        let diff = r#"@@ -1,2 +1,7 @@
 fn main() {
-    println!("Hello, world!");
+    println!("Hello, MCP server!");
+
+    // Display a welcome message
+    let message = "Welcome to Corrode MCP";
+    println!("{}", message);
 }"#;

        let expected = r#"fn main() {
    println!("Hello, MCP server!");

    // Display a welcome message
    let message = "Welcome to Corrode MCP";
    println!("{}", message);
}"#;

        match apply_diff(original, diff) {
            Ok(result) => assert_eq!(result.trim(), expected.trim()),
            Err(e) => panic!("apply_diff failed: {}", e),
        }
    }

    #[test]
    fn test_apply_diff_simple_edit() {
        let original = "Line 1\nLine 2\nLine 3\n";
        // Note: The diff format requires context lines to start with a space.
        // Also ensure the hunk header is correct.
        let diff = "@@ -1,3 +1,3 @@\n Line 1\n-Line 2\n+Line 2 - Edited\n Line 3\n";
        let expected = "Line 1\nLine 2 - Edited\nLine 3\n";

        match apply_diff(original, diff) {
            Ok(result) => assert_eq!(result, expected), // Compare exact strings including trailing newline
            Err(e) => panic!("apply_diff failed: {}", e),
        }
    }

}
