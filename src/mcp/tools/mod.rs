pub mod crates_io_tools;

use crate::mcp::types::*;
use maplit::hashmap;
use crate::mcp::treesitter;
use rpc_router::{Handler, HandlerResult, RouterBuilder, RpcParams};
use serde::{Deserialize, Serialize};
use std::fs::{self}; // Remove unused File import
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
// Remove unused std::env import

/// register all tools to the router
pub fn register_tools(router_builder: RouterBuilder) -> RouterBuilder {
    router_builder
        .append_dyn("tools/list", tools_list.into_dyn())
        .append_dyn("execute_bash", execute_bash.into_dyn())
        .append_dyn("read_file", read_file.into_dyn())
        .append_dyn("edit_file", edit_file.into_dyn())
        .append_dyn("write_file", write_file.into_dyn())
        .append_dyn("check", check_code.into_dyn())
        .append_dyn("parse_code", parse_code.into_dyn())
        .append_dyn("search_crates", crates_io_tools::search_crates.into_dyn())
        .append_dyn("get_crate", crates_io_tools::get_crate.into_dyn())
        .append_dyn("get_crate_versions", crates_io_tools::get_crate_versions.into_dyn())
        .append_dyn("get_crate_dependencies", crates_io_tools::get_crate_dependencies.into_dyn())
        .append_dyn("lookup_crate_docs", crates_io_tools::lookup_crate_docs.into_dyn())
}

// Create a global state to track the current working directory
lazy_static::lazy_static! {
    static ref CURRENT_WORKING_DIR: Mutex<PathBuf> = Mutex::new(PathBuf::from(std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))));
}

// Helper function to update working directory when cd commands are used
fn handle_cd_command(command: &str) -> Option<PathBuf> {
    let command = command.trim();
    
    // Check if command is a cd command or starts with cd and has more components
    if command == "cd" || command.starts_with("cd ") {
        let parts: Vec<&str> = command.splitn(2, ' ').collect();
        if parts.len() == 1 {
            // Just "cd", go to home directory
            return Some(dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")));
        } else if parts.len() == 2 {
            let dir = parts[1].trim();
            let current_dir = CURRENT_WORKING_DIR.lock().unwrap();
            
            // Handle different path types
            let new_path = if dir.starts_with("/") {
                PathBuf::from(dir)
            } else if dir.starts_with("~") {
                let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                home.join(dir.trim_start_matches("~").trim_start_matches("/"))
            } else {
                current_dir.join(dir)
            };
            
            return Some(new_path);
        }
    }
    None
}

pub async fn tools_list(_request: Option<ListToolsRequest>) -> HandlerResult<ListToolsResult> {
    //let tools: Vec<Tool> = serde_json::from_str(include_str!("./templates/tools.json")).unwrap();
    let response = ListToolsResult {
        tools: vec![
            Tool {
                name: "execute_bash".to_string(),
                description: Some("Execute a command using bash shell, Ask user if you're unsure if it's ok to execute or if the command could be destructive.".to_string()),
                input_schema: ToolInputSchema {
                    type_name: "object".to_string(),
                    properties: hashmap! {
                        "command".to_string() => ToolInputSchemaProperty {
                            type_name: Some("string".to_owned()),
                            description: Some("The bash command to execute".to_owned()),
                            enum_values: None,
                        }
                    },
                    required: vec!["command".to_string()],
                },
            },
            Tool {
                name: "read_file".to_string(),
                description: Some("Read a file's contents, up to 1000 characters.".to_string()),
                input_schema: ToolInputSchema {
                    type_name: "object".to_string(),
                    properties: hashmap! {
                        "file_path".to_string() => ToolInputSchemaProperty {
                            type_name: Some("string".to_owned()),
                            description: Some("The path to the file to read".to_owned()),
                            enum_values: None,
                        },
                        "max_chars".to_string() => ToolInputSchemaProperty {
                            type_name: Some("integer".to_owned()),
                            description: Some("Maximum number of characters to read (defaults to 1000)".to_owned()),
                            enum_values: None,
                        }
                    },
                    required: vec!["file_path".to_string()],
                },
            },
            Tool {
                name: "edit_file".to_string(),
                description: Some("Edit a file by applying a diff to it.".to_string()),
                input_schema: ToolInputSchema {
                    type_name: "object".to_string(),
                    properties: hashmap! {
                        "file_path".to_string() => ToolInputSchemaProperty {
                            type_name: Some("string".to_owned()),
                            description: Some("The path to the file to edit".to_owned()),
                            enum_values: None,
                        },
                        "diff".to_string() => ToolInputSchemaProperty {
                            type_name: Some("string".to_owned()),
                            description: Some("The diff to apply to the file (unified diff format)".to_owned()),
                            enum_values: None,
                        }
                    },
                    required: vec!["file_path".to_string(), "diff".to_string()],
                },
            },
            Tool {
                name: "write_file".to_string(),
                description: Some("Write content to a file using the current working directory.".to_string()),
                input_schema: ToolInputSchema {
                    type_name: "object".to_string(),
                    properties: hashmap! {
                        "file_path".to_string() => ToolInputSchemaProperty {
                            type_name: Some("string".to_owned()),
                            description: Some("The path to the file to write, relative to current working directory".to_owned()),
                            enum_values: None,
                        },
                        "content".to_string() => ToolInputSchemaProperty {
                            type_name: Some("string".to_owned()),
                            description: Some("The content to write to the file".to_owned()),
                            enum_values: None,
                        }
                    },
                    required: vec!["file_path".to_string(), "content".to_string()],
                },
            },
            Tool {
                name: "check".to_string(),
                description: Some("Check code for errors after editing. For Rust projects, runs 'cargo check' in the current working directory. This tool should be used directly after running edit_file to verify changes are valid.".to_string()),
                input_schema: ToolInputSchema {
                    type_name: "object".to_string(),
                    properties: hashmap! {
                        // No parameters needed as it uses current working directory
                    },
                    required: vec![],
                },
            },
            Tool {
                name: "parse_code".to_string(),
                description: Some("Parse code using TreeSitter to extract function names, class definitions, and structure. Supports Rust, JavaScript, TypeScript, Python, Go, C, and C++.".to_string()),
                input_schema: ToolInputSchema { 
                    type_name: "object".to_string(),
                    properties: hashmap! {
                        "file_path".to_string() => ToolInputSchemaProperty {
                            type_name: Some("string".to_owned()),
                            description: Some("The path to the file to parse, relative to current working directory".to_owned()),
                            enum_values: None,
                        },
                        "project_path".to_string() => ToolInputSchemaProperty {
                            type_name: Some("string".to_owned()),
                            description: Some("Optional project root path to analyze. If not provided, will use the file's directory.".to_owned()),
                            enum_values: None,
                        }
                    },
                    required: vec!["file_path".to_string()],
                },
            },
            Tool {
                name: "search_crates".to_string(),
                description: Some("Search for packages on crates.io".to_string()),
                input_schema: ToolInputSchema {
                    type_name: "object".to_string(),
                    properties: hashmap! {
                        "query".to_string() => ToolInputSchemaProperty {
                            type_name: Some("string".to_owned()),
                            description: Some("Search query string".to_owned()),
                            enum_values: None,
                        },
                        "page".to_string() => ToolInputSchemaProperty {
                            type_name: Some("integer".to_owned()),
                            description: Some("Page number for pagination (defaults to 1)".to_owned()),
                            enum_values: None,
                        },
                        "per_page".to_string() => ToolInputSchemaProperty {
                            type_name: Some("integer".to_owned()),
                            description: Some("Number of results per page (defaults to 10, max 100)".to_owned()),
                            enum_values: None,
                        }
                    },
                    required: vec!["query".to_string()],
                },
            },
            Tool {
                name: "get_crate".to_string(),
                description: Some("Get detailed information about a specific crate".to_string()),
                input_schema: ToolInputSchema {
                    type_name: "object".to_string(),
                    properties: hashmap! {
                        "crate_name".to_string() => ToolInputSchemaProperty {
                            type_name: Some("string".to_owned()),
                            description: Some("Name of the crate".to_owned()),
                            enum_values: None,
                        }
                    },
                    required: vec!["crate_name".to_string()],
                },
            },
            Tool {
                name: "get_crate_versions".to_string(),
                description: Some("Get all versions of a specific crate, run this before adding a dependency to ensure you use the latest version".to_string()),
                input_schema: ToolInputSchema {
                    type_name: "object".to_string(),
                    properties: hashmap! {
                        "crate_name".to_string() => ToolInputSchemaProperty {
                            type_name: Some("string".to_owned()),
                            description: Some("Name of the crate".to_owned()),
                            enum_values: None,
                        }
                    },
                    required: vec!["crate_name".to_string()],
                },
            },
            Tool {
                name: "get_crate_dependencies".to_string(),
                description: Some("Get dependencies for a specific version of a crate".to_string()),
                input_schema: ToolInputSchema {
                    type_name: "object".to_string(),
                    properties: hashmap! {
                        "crate_name".to_string() => ToolInputSchemaProperty {
                            type_name: Some("string".to_owned()),
                            description: Some("Name of the crate".to_owned()),
                            enum_values: None,
                        },
                        "version".to_string() => ToolInputSchemaProperty {
                            type_name: Some("string".to_owned()),
                            description: Some("Version of the crate".to_owned()),
                            enum_values: None,
                        }
                    },
                    required: vec!["crate_name".to_string(), "version".to_string()],
                },
            },
            Tool {
                name: "lookup_crate_docs".to_string(),
                description: Some("Lookup documentation for a Rust crate from docs.rs, use this to get farmiliar with APIs for the latest version of a crate.".to_string()),
                input_schema: ToolInputSchema {
                    type_name: "object".to_string(),
                    properties: hashmap! {
                        "crateName".to_string() => ToolInputSchemaProperty {
                            type_name: Some("string".to_owned()),
                            description: Some("Name of the Rust crate to lookup documentation for (defaults to 'tokio')".to_owned()),
                            enum_values: None,
                        }
                    },
                    required: vec![], // crateName is optional
                },
            }
        ],
        next_cursor: None,
    };
    Ok(response)
}

#[derive(Deserialize, Serialize, RpcParams)]
pub struct CurrentTimeRequest {
    pub city: Option<String>,
}

#[allow(dead_code)]
pub async fn current_time(_request: CurrentTimeRequest) -> HandlerResult<CallToolResult> {
    let result = format!("Now: {}!", chrono::Local::now().to_rfc2822());
    Ok(CallToolResult {
        content: vec![CallToolResultContent::Text { text: result }],
        is_error: false,
    })
}

#[derive(Deserialize, Serialize, RpcParams)]
pub struct ExecuteBashRequest {
    pub command: String,
}

pub async fn execute_bash(request: ExecuteBashRequest) -> HandlerResult<CallToolResult> {
    let command = request.command.clone();
    let mut result = String::new();
    let mut is_error = false;
    
    // Split commands if they contain && or ;
    let commands: Vec<&str> = if command.contains("&&") {
        command.split("&&").collect()
    } else if command.contains(';') {
        command.split(';').collect()
    } else {
        vec![&command]
    };
    
    for cmd in commands {
        let cmd = cmd.trim();
        
        // Check if command is a cd command and update working directory if it is
        if let Some(new_dir) = handle_cd_command(cmd) {
            // Try to actually change to this directory to verify it exists
            if new_dir.exists() && new_dir.is_dir() {
                let mut current_dir = CURRENT_WORKING_DIR.lock().unwrap();
                *current_dir = new_dir.clone();
                result.push_str(&format!("Changed directory to: {}\n", new_dir.display()));
            } else {
                result.push_str(&format!("Directory does not exist: {}\n", new_dir.display()));
                is_error = true;
                break; // Stop executing further commands if cd fails
            }
            
            // If this is a pure cd command, we're done
            if cmd.starts_with("cd") && !cmd.contains("&&") && !cmd.contains(';') {
                continue;
            }
        }
        
        // For non-cd commands or combined commands, execute with proper working directory
        let current_dir = CURRENT_WORKING_DIR.lock().unwrap().clone();
        
        // Execute the command with the current working directory
        let output = Command::new("bash")
            .arg("-l") // Run as a login shell to load full environment
            .current_dir(&current_dir)
            .arg("-c")
            .arg(cmd)
            .output();
        
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                
                let cmd_result = format!("$ {}\n", cmd);
                result.push_str(&cmd_result);
                
                let exit_status = output.status.code().unwrap_or(-1);
                let cmd_is_error = !output.status.success();
                if cmd_is_error {
                    is_error = true;
                }
                
                result.push_str(&format!("Exit code: {}\n", exit_status));
                
                if !stdout.is_empty() {
                    result.push_str(&format!("\nStandard output:\n{}", stdout));
                }
                
                if !stderr.is_empty() {
                    result.push_str(&format!("\nStandard error:\n{}\n", stderr));
                }
                
                // If a command fails, stop executing
                if cmd_is_error {
                    break;
                }
            },
            Err(e) => {
                result.push_str(&format!("Failed to execute command '{}': {}\n", cmd, e));
                is_error = true;
                break;
            }
        }
    }
    
    Ok(CallToolResult {
        content: vec![CallToolResultContent::Text { text: result }],
        is_error,
    })
}

#[derive(Deserialize, Serialize, RpcParams)]
pub struct EditFileRequest {
    pub file_path: String,
    pub diff: String,
}

pub async fn edit_file(request: EditFileRequest) -> HandlerResult<CallToolResult> {
    // Get the current working directory and resolve the file path
    let current_dir = CURRENT_WORKING_DIR.lock().unwrap().clone();
    let file_path = resolve_path(&current_dir, &request.file_path);
    let display_path = file_path.display().to_string();
    
    // Read the original file content
    let file_result = fs::read_to_string(&file_path);
    match file_result {
        Ok(original_content) => {
            // Apply the diff to the original content
            match apply_diff(&original_content, &request.diff) {
                Ok(new_content) => {
                    // Write the modified content back to the file
                    match fs::write(&file_path, new_content) {
                        Ok(_) => {
                            Ok(CallToolResult {
                                content: vec![CallToolResultContent::Text { 
                                    text: format!("Successfully applied diff to file: {}", display_path) 
                                }],
                                is_error: false,
                            })
                        },
                        Err(e) => {
                            Ok(CallToolResult {
                                content: vec![CallToolResultContent::Text { 
                                    text: format!("Error writing to file '{}': {}", display_path, e) 
                                }],
                                is_error: true,
                            })
                        }
                    }
                },
                Err(e) => {
                    Ok(CallToolResult {
                        content: vec![CallToolResultContent::Text { 
                            text: format!("Error applying diff: {}", e) 
                        }],
                        is_error: true,
                    })
                }
            }
        },
        Err(e) => {
            Ok(CallToolResult {
                content: vec![CallToolResultContent::Text { 
                    text: format!("Error reading file '{}': {}", display_path, e) 
                }],
                is_error: true,
            })
        }
    }
}

#[derive(Deserialize, Serialize, RpcParams)]
pub struct WriteFileRequest {
    pub file_path: String,
    pub content: String,
}

pub async fn write_file(request: WriteFileRequest) -> HandlerResult<CallToolResult> {
    // Get the current working directory and resolve the file path
    let current_dir = CURRENT_WORKING_DIR.lock().unwrap().clone();
    let file_path = resolve_path(&current_dir, &request.file_path);
    let display_path = file_path.display().to_string();
    
    // Ensure parent directories exist
    if let Some(parent) = file_path.parent() {
        if !parent.exists() {
            match fs::create_dir_all(parent) {
                Ok(_) => {},
                Err(e) => {
                    return Ok(CallToolResult {
                        content: vec![CallToolResultContent::Text { 
                            text: format!("Error creating directory structure for '{}': {}", display_path, e) 
                        }],
                        is_error: true,
                    });
                }
            }
        }
    }
    
    // Write the content to the file
    match fs::write(&file_path, &request.content) {
        Ok(_) => Ok(CallToolResult {
            content: vec![CallToolResultContent::Text { text: format!("Successfully wrote to file: {}", display_path) }],
            is_error: false,
        }),
        Err(e) => Ok(CallToolResult {
            content: vec![CallToolResultContent::Text { text: format!("Error writing to file '{}': {}", display_path, e) }],
            is_error: true,
        }),
    }
}

#[derive(Deserialize, Serialize, RpcParams)]
pub struct CheckRequest {}

pub async fn check_code(_request: Option<CheckRequest>) -> HandlerResult<CallToolResult> {
    // Get the current working directory
    let current_dir = CURRENT_WORKING_DIR.lock().unwrap().clone();
    
    // Check if this is a Rust project by looking for Cargo.toml
    let cargo_toml_path = current_dir.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        return Ok(CallToolResult {
            content: vec![CallToolResultContent::Text { 
                text: format!("No Cargo.toml found in '{}'. This doesn't appear to be a Rust project.", current_dir.display()) 
            }],
            is_error: true,
        });
    }

    // Execute 'cargo check' using the execute_bash tool
    let bash_request = ExecuteBashRequest {
        command: "cargo check".to_string(),
    };
    
    execute_bash(bash_request).await
}

#[derive(Deserialize, Serialize, RpcParams)]
pub struct ParseCodeRequest {
    pub file_path: String,
    pub project_path: Option<String>,
}

pub async fn parse_code(request: ParseCodeRequest) -> HandlerResult<CallToolResult> {
    // Get the current working directory
    let current_dir = CURRENT_WORKING_DIR.lock().unwrap().clone();

    // Show path debug info
    let mut diagnostic_info = String::new();
    diagnostic_info.push_str(&format!("Current working directory: {}\n", current_dir.display()));
    diagnostic_info.push_str(&format!("Requested file_path: {}\n", request.file_path));
    if let Some(ref project_path) = request.project_path {
        diagnostic_info.push_str(&format!("Requested project_path: {}\n", project_path));
    }
    
    // Determine project directory
    let project_dir = if let Some(ref project_path) = request.project_path {
        // First, try treating project_path as absolute
        let absolute_path = PathBuf::from(project_path);
        if absolute_path.is_absolute() && absolute_path.exists() && absolute_path.is_dir() {
            diagnostic_info.push_str("Using project_path as absolute path\n");
            absolute_path
        } else {
            // Otherwise resolve relative to current directory
            let resolved_path = resolve_path(&current_dir, project_path);
            diagnostic_info.push_str(&format!("Resolved project_path to: {}\n", resolved_path.display()));
            resolved_path
        }
    } else {
        // If no project_path provided, use current directory
        diagnostic_info.push_str("No project_path provided, using current directory\n");
        current_dir.clone()
    };
    
    // Check if project directory exists and is accessible
    if !project_dir.exists() {
        return Ok(CallToolResult {
            content: vec![CallToolResultContent::Text { 
                text: format!("Error: Project directory '{}' does not exist\n\nDiagnostic Info:\n{}", 
                            project_dir.display(), diagnostic_info) 
            }],
            is_error: true,
        });
    }
    
    if !project_dir.is_dir() {
        return Ok(CallToolResult {
            content: vec![CallToolResultContent::Text { 
                text: format!("Error: '{}' is not a directory\n\nDiagnostic Info:\n{}", 
                            project_dir.display(), diagnostic_info) 
            }],
            is_error: true,
        });
    }
    
    // Resolve file path
    let file_path = if request.project_path.is_some() {
        // When project_path is specified, treat file_path as relative to project_dir
        // Ensure we clean up the path by removing any leading slashes
        let clean_path = request.file_path.trim_start_matches('/');
        
        // Handle special case where file_path starts with the project directory name
        if let Some(proj_name) = project_dir.file_name() {
            let proj_name_str = proj_name.to_string_lossy().to_string();
            if clean_path.starts_with(&format!("{}/", proj_name_str)) {
                // If file_path starts with project name followed by '/', strip it
                let stripped_path = clean_path.strip_prefix(&format!("{}/", proj_name_str))
                    .unwrap_or(clean_path);
                    
                diagnostic_info.push_str(&format!("Detected project name prefix in file_path, stripped to: {}\n", stripped_path));
                project_dir.join(stripped_path)
            } else {
                diagnostic_info.push_str(&format!("Joining project_dir with file_path: {}\n", clean_path));
                project_dir.join(clean_path)
            }
        } else {
            diagnostic_info.push_str(&format!("Joining project_dir with file_path: {}\n", clean_path));
            project_dir.join(clean_path)
        }
    } else {
        // When no project_path is specified, use standard path resolution
        let resolved_path = resolve_path(&current_dir, &request.file_path);
        diagnostic_info.push_str(&format!("Resolved file_path to: {}\n", resolved_path.display()));
        resolved_path
    };
    
    // Verify the file exists
    if !file_path.exists() {
        return Ok(CallToolResult {
            content: vec![CallToolResultContent::Text { 
                text: format!("Error: File '{}' does not exist\n\nDiagnostic Info:\n{}", 
                            file_path.display(), diagnostic_info) 
            }],
            is_error: true,
        });
    }
    
    if !file_path.is_file() {
        return Ok(CallToolResult {
            content: vec![CallToolResultContent::Text { 
                text: format!("Error: '{}' is not a file\n\nDiagnostic Info:\n{}", 
                            file_path.display(), diagnostic_info) 
            }],
            is_error: true,
        });
    }
    
    let _file_content = match fs::read_to_string(&file_path) { // Prefix unused variable
        Ok(content) => content,
        Err(e) => {
            return Ok(CallToolResult {
                content: vec![CallToolResultContent::Text { 
                    text: format!("Error reading file '{}': {}\n\nDiagnostic Info:\n{}", 
                                file_path.display(), e, diagnostic_info) 
                }],
                is_error: true,
            });
        }
    };
    
    // Now analyze the code with TreeSitter
    match treesitter::parse_file(&file_path, None) {
        Some(file_info) => {
            // Serialize the FileInfo struct to JSON string
            match serde_json::to_string_pretty(&file_info) {
                Ok(json_string) => Ok(CallToolResult {
                    content: vec![CallToolResultContent::Text { text: json_string }],
                    is_error: false,
                }),
                Err(e) => Ok(CallToolResult {
                    content: vec![CallToolResultContent::Text {
                        text: format!("Error serializing parse results: {}\n\nDiagnostic Info:\n{}", e, diagnostic_info)
                    }],
                    is_error: true,
                }),
            }
        },
        None => {
            Ok(CallToolResult {
                content: vec![CallToolResultContent::Text {
                    text: format!("Error parsing file: Could not parse '{}'\n\nDiagnostic Info:\n{}", file_path.display(), diagnostic_info)
                }],
                is_error: true,
            })
        }
    }
}

// Function to apply a diff to a string
fn apply_diff(original: &str, diff_str: &str) -> Result<String, String> {
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
                
                for (idx, line) in original_lines.iter().enumerate() {
                    if idx + 1 == start as usize {
                        // Skip the lines to be removed
                        for (remove_idx, remove_line) in to_remove.iter().enumerate() {
                            if remove_idx < original_lines.len() - idx && **remove_line != *original_lines[idx + remove_idx] {
                                return Err(format!("Diff mismatch at line {}", start + remove_idx));
                            }
                        }
                        
                        // Add the new lines
                        for add_line in &to_add { // Iterate over a slice to avoid moving
                            new_lines.push(add_line.to_string());
                        }
                        
                        // Skip over the removed lines
                        for _ in 0..to_remove.len() {
                            if idx < original_lines.len() - 1 {
                                continue;
                            }
                        }
                    } else {
                        new_lines.push(line.to_string());
                    }
                }
                
                result = new_lines.join("\n");
                if !result.ends_with('\n') && original.ends_with('\n') {
                    result.push('\n');
                }
                
                i = j;
            } else {
                return Err("Failed to parse hunk header".to_string());
            }
        } else {
            i += 1;
        }
    }
    
    Ok(result)
}

// Helper function to parse a unified diff hunk header
fn parse_hunk_header(header: &str) -> Result<(usize, usize), String> {
    let parts: Vec<&str> = header.split(' ').collect();
    for part in parts {
        if part.starts_with('-') {
            let line_spec = &part[1..];
            if let Some(comma_idx) = line_spec.find(',') {
                let start = line_spec[..comma_idx].parse::<usize>().map_err(|e| e.to_string())?;
                let count = line_spec[comma_idx+1..].parse::<usize>().map_err(|e| e.to_string())?;
                return Ok((start, count));
            }
        }
    }
    Err("Invalid hunk header format".to_string())
}

#[derive(Deserialize, Serialize, RpcParams)]
pub struct ReadFileRequest {
    pub file_path: String,
    pub max_chars: Option<usize>,
}

pub async fn read_file(request: ReadFileRequest) -> HandlerResult<CallToolResult> {
    // Get the current working directory
    let current_dir = CURRENT_WORKING_DIR.lock().unwrap().clone();
    
    // Resolve the file path
    let file_path = resolve_path(&current_dir, &request.file_path);
    let display_path = file_path.display().to_string();
    
    // Check if the file exists
    if !file_path.exists() {
        return Ok(CallToolResult {
            content: vec![CallToolResultContent::Text { 
                text: format!("Error: File '{}' does not exist", display_path) 
            }],
            is_error: true,
        });
    }
    
    if !file_path.is_file() {
        return Ok(CallToolResult {
            content: vec![CallToolResultContent::Text { 
                text: format!("Error: '{}' is not a file", display_path) 
            }],
            is_error: true,
        });
    }
    
    // Read the file content
    match fs::read_to_string(&file_path) {
        Ok(content) => {
            // Limit the content length if max_chars is specified
            let max_chars = request.max_chars.unwrap_or(1000);
            let truncated = if content.len() > max_chars {
                let truncated_content = content.chars().take(max_chars).collect::<String>();
                truncated_content + "\n\n(content truncated due to size limit)"
            } else {
                content
            };
            
            Ok(CallToolResult {
                content: vec![CallToolResultContent::Text { text: truncated }],
                is_error: false,
            })
        },
        Err(e) => {
            Ok(CallToolResult {
                content: vec![CallToolResultContent::Text { 
                    text: format!("Error reading file '{}': {}", display_path, e) 
                }],
                is_error: true,
            })
        }
    }
}

// Helper function to resolve a file path relative to the current directory
fn resolve_path(current_dir: &Path, file_path: &str) -> PathBuf {
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