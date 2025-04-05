mod mcp;
use corrode_mcp::{apply_diff, handle_cd_command, resolve_path};
use mcp_attr::Result;
use mcp_attr::schema::{ // Import all schema types here
    GetPromptResult, CallToolResult, Role, TextContent, PromptMessage
};
use serde_json::Value;
use crate::mcp::treesitter;
use std::collections::HashMap; // Keep for now
use mcp_attr::server::{mcp_server, McpServer, serve_stdio};
// Remove unused imports based on cargo check warnings
use std::sync::Mutex;
use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use std::env;
use dirs;
use crate::mcp::crates_io::{CratesIoClient, RequestOptions, FetchResponse};
use serde::Deserialize;
use schemars::JsonSchema;
use reqwest;
use html2text;

// --- Argument Structs for Tools (derive Deserialize and JsonSchema) ---

#[derive(Deserialize, JsonSchema)]
struct SearchCratesArgs {
    query: String,
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
struct GetCrateArgs {
    crate_name: String,
}

#[derive(Deserialize, JsonSchema)]
struct GetCrateVersionsArgs {
    crate_name: String,
}

#[derive(Deserialize, JsonSchema)]
struct GetCrateDependenciesArgs {
    crate_name: String,
    version: String,
}

#[derive(Deserialize, JsonSchema)]
struct LookupCrateDocsArgs {
    #[serde(rename = "crateName")] // Keep original name for compatibility
    crate_name: Option<String>,
}


// Define the server state structure (can be expanded later)
struct ServerData {
    current_working_dir: PathBuf,
    // Add state fields here if needed
    http_client: reqwest::Client,
}

// Define the main server struct
struct CorrodeMcpServer(Mutex<ServerData>);

// Implement the McpServer trait using the attribute macro
#[mcp_server]
impl McpServer for CorrodeMcpServer {
    // Prompt, Resource, and Tool methods will be added here later
    /// Search for crates on crates.io
    #[prompt]
    async fn search_crates(
        &self,
        /// Search query string
        query: String,
        /// Page number (optional)
        _page: Option<String>, // Prefix unused variable
        /// Results per page (optional)
        _per_page: Option<String>, // Prefix unused variable
    ) -> Result<GetPromptResult> { // Updated return type
        // Note: page and per_page are currently unused in the prompt text generation
        let prompt_text = format!("Search crates.io for '{}'. Summarize the top results.", query);
        // Return a simple String, letting `Into<GetPromptResult>` handle conversion
        Ok(GetPromptResult::from(prompt_text))
    }

    // /// Get details for a specific crate
    // #[prompt]
    // async fn get_crate(
    //     &self,
    //     /// Name of the crate
    //     crate_name: String,
    // ) -> Result<GetPromptResult> {
    //     Ok(GetPromptResult {
    //         description: Some("Get crate details".to_string()),
    //         messages: Some(vec![PromptMessage {
    //             role: Role::User,
    //             content: TextContent {
    //                 text: format!("Provide a summary of the crate '{}' based on its details.", crate_name),
    //                 type_: "text".to_string(),
    //                 annotations: None, // Assuming annotations are optional
    //             },
    //         }]),
    //         meta: Default::default(), // Use Default::default() for the Map
    //     })
    // }

    // /// Get versions for a specific crate
    // #[prompt]
    // async fn get_crate_versions(
    //     &self,
    //     /// Name of the crate
    //     crate_name: String,
    // ) -> Result<GetPromptResult> {
    //     Ok(GetPromptResult {
    //         description: Some("Get crate versions".to_string()),
    //         messages: Some(vec![PromptMessage {
    //             role: Role::User,
    //             content: TextContent {
    //                 text: format!("List the recent versions of the crate '{}'.", crate_name),
    //                 type_: "text".to_string(),
    //                 annotations: None,
    //             },
    //         }]),
    //         meta: Default::default(), // Use Default::default()
    //     })
    // }

    // /// Get dependencies for a specific crate version
    // #[prompt]
    // async fn get_crate_dependencies(
    //     &self,
    //     /// Name of the crate
    //     crate_name: String,
    //     /// Version of the crate
    //     version: String,
    // ) -> Result<GetPromptResult> {
    //      Ok(GetPromptResult {
    //         description: Some("Get crate dependencies".to_string()),
    //         messages: Some(vec![PromptMessage {
    //             role: Role::User,
    //             content: TextContent {
    //                 text: format!("List the main dependencies for crate '{}' version {}.", crate_name, version),
    //                 type_: "text".to_string(),
    //                 annotations: None,
    //             },
    //         }]),
    //         meta: Default::default(), // Use Default::default()
    //     })
    // }

    // /// Lookup documentation for a Rust crate from docs.rs
    // #[prompt]
    // async fn lookup_crate_docs(
    //     &self,
    //     /// Name of the Rust crate (defaults to 'tokio')
    //     #[arg("crateName")] // Use #[arg] to match the original argument name
    //     crate_name: Option<String>, // Optional argument
    // ) -> Result<GetPromptResult> {
    //     let name = crate_name.unwrap_or_else(|| "tokio".to_string());
    //     Ok(GetPromptResult {
    //         description: Some("Lookup crate documentation".to_string()),
    //         messages: Some(vec![PromptMessage {
    //             role: Role::User,
    //             content: TextContent {
    //                 text: format!(
    //                     "Please analyze and summarize the documentation for the Rust crate '{}'. Focus on:\n1. The main purpose and features of the crate\n2. Key types and functions\n3. Common usage patterns\n4. Any important notes or warnings\n5. VERY IMPORTANT: Latest Version\n\nDocumentation content will follow.",
    //                     name
    //                 ),
    //                 type_: "text".to_string(),
    //                 annotations: None,
    //             },
    //         }]),
    //         meta: Default::default(), // Use Default::default()
    //     })
    // }
    // /// Execute a bash command
    // #[prompt]
    // async fn execute_bash(
    //     &self,
    //     /// The bash command to execute
    //     command: String,
    // ) -> Result<GetPromptResult> {
    //     Ok(GetPromptResult {
    //         description: Some("Execute bash command".to_string()),
    //         messages: Some(vec![PromptMessage {
    //             role: Role::User,
    //             content: TextContent {
    //                 text: format!("Execute the following bash command and report the output:\n```bash\n{}\n```", command),
    //                 type_: "text".to_string(),
    //                 annotations: None,
    //             },
    //         }]),
    //         meta: Default::default(), // Use Default::default()
    //     })
    // }

    // /// Read the content of a file
    // #[prompt]
    // async fn read_file(
    //     &self,
    //     /// Path to the file to read
    //     file_path: String,
    //     /// Maximum characters to read (optional, default 1000)
    //     max_chars: Option<String>, // Keep as String for now, parsing logic would be in the tool itself
    // ) -> Result<GetPromptResult> {
    //     // Note: max_chars is currently unused in the prompt text generation
    //     Ok(GetPromptResult {
    //         description: Some("Read file content".to_string()),
    //         messages: Some(vec![PromptMessage {
    //             role: Role::User,
    //             content: TextContent {
    //                 text: format!("Read the content of the file '{}' and provide it.", file_path),
    //                 type_: "text".to_string(),
    //                 annotations: None,
    //             },
    //         }]),
    //         meta: Default::default(), // Use Default::default()
    //     })
    // }

    // /// Edit a file using a diff
    // #[prompt]
    // async fn edit_file(
    //     &self,
    //     /// Path to the file to edit
    //     file_path: String,
    //     /// The diff content (unified format)
    //     diff: String,
    // ) -> Result<GetPromptResult> {
    //     Ok(GetPromptResult {
    //         description: Some("Edit file with diff".to_string()),
    //         messages: Some(vec![PromptMessage {
    //             role: Role::User,
    //             content: TextContent {
    //                 text: format!("Apply the following diff to the file '{}':\n```diff\n{}\n```", file_path, diff),
    //                 type_: "text".to_string(),
    //                 annotations: None,
    //             },
    //         }]),
    //         meta: Default::default(), // Use Default::default()
    //     })
    // }

    // /// Write content to a file
    // #[prompt]
    // async fn write_file(
    //     &self,
    //     /// Path to the file to write
    //     file_path: String,
    //     /// The content to write
    //     content: String,
    // ) -> Result<GetPromptResult> {
    //     Ok(GetPromptResult {
    //         description: Some("Write content to file".to_string()),
    //         messages: Some(vec![PromptMessage {
    //             role: Role::User,
    //             content: TextContent {
    //                 text: format!("Write the following content to the file '{}':\n```\n{}\n```", file_path, content),
    //                 type_: "text".to_string(),
    //                 annotations: None,
    //             },
    //         }]),
    //         meta: Default::default(), // Use Default::default()
    //     })
    // }

    // /// Check Rust code for errors using 'cargo check'
    // #[prompt]
    // async fn check(&self) -> Result<GetPromptResult> { // No arguments
    //      Ok(GetPromptResult {
    //         description: Some("Check Rust code".to_string()),
    //         messages: Some(vec![PromptMessage {
    //             role: Role::User,
    //             content: TextContent {
    //                 text: "Run 'cargo check' on the current Rust project and report any errors or warnings.".to_string(),
    //                 type_: "text".to_string(),
    //                 annotations: None,
    //             },
    //         }]),
    //         meta: Default::default(), // Use Default::default()
    //     })
    // }

    // /// Parse code structure using TreeSitter
    // #[prompt]
    // async fn parse_code(
    //     &self,
    //     /// Path to the file to parse
    //     file_path: String,
    //     /// Optional project root path
    //     project_path: Option<String>,
    // ) -> Result<GetPromptResult> {
    //     // Note: project_path is currently unused in the prompt text generation
    //     Ok(GetPromptResult {
    //         description: Some("Parse code structure".to_string()),
    //         messages: Some(vec![PromptMessage {
    //             role: Role::User,
    //             content: TextContent {
    //                 text: format!("Parse the code structure (functions, classes, etc.) of the file '{}' using TreeSitter.", file_path),
    //                 type_: "text".to_string(),
    //                 annotations: None,
    //             },
    //         }]),
    //         meta: Default::default(), // Use Default::default()
    //     })
    // }
    // --- Tool Implementations ---

    /// Execute a command using bash shell. Handles 'cd' to change server's working directory.
    #[tool] 
    async fn tool_execute_bash(&self, command: String) -> Result<CallToolResult> { // Revert to CallToolResult
        let mut result = String::new();

        // Split commands if they contain && or ;
        let commands: Vec<&str> = if command.contains("&&") {
            command.split("&&").collect()
        } else if command.contains(';') {
            command.split(';').collect()
        } else {
            vec![&command]
        };

        // Lock the state once for the duration of processing this command sequence
        let mut server_state = self.0.lock().unwrap();

        for cmd in commands {
            let cmd = cmd.trim();
            let current_dir_path = server_state.current_working_dir.clone(); // Clone for use in this iteration

            // Check if command is a cd command and update working directory if it is
            if let Some(new_dir) = handle_cd_command(&current_dir_path, cmd) {
                // Try to actually change to this directory to verify it exists
                if new_dir.exists() && new_dir.is_dir() {
                    // Update the server state's CWD
                    server_state.current_working_dir = new_dir.clone();
                    result.push_str(&format!("Changed directory to: {}\n", new_dir.display()));
                } else {
                    result.push_str(&format!("Directory does not exist: {}\n", new_dir.display()));
                    // Stop executing further commands if cd fails
                    // Use bail! which converts to the appropriate error type for Result<CallToolResult>
                    mcp_attr::bail!("Directory does not exist: {}", new_dir.display());
                }

                // If this is a pure cd command, we're done with this part of the sequence
                if cmd == "cd" || (cmd.starts_with("cd ") && !cmd.contains("&&") && !cmd.contains(';')) {
                     continue;
                }
                 // If cd was part of a chain (e.g., cd foo && ls), update current_dir_path for the next part
                 // This might be complex if the chain involves multiple cds.
                 // For simplicity, we assume the CWD change applies to subsequent commands *in this tool call*.
                 // The state `server_state.current_working_dir` is updated for future tool calls.
                 // Re-cloning might be needed if the loop structure changes significantly.
                 // current_dir_path = server_state.current_working_dir.clone(); // Re-sync if needed
            }

            // For non-cd commands or combined commands, execute with proper working directory
            // Use the potentially updated current_dir_path for this specific command execution
            let output = Command::new("bash")
                .arg("-l") // Run as a login shell to load full environment
                .current_dir(&current_dir_path) // Use the CWD relevant to this command
                .arg("-c")
                .arg(cmd) // Execute the potentially non-cd part
                .output();

            match output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                    let cmd_result = format!("$ {}\n", cmd);
                    result.push_str(&cmd_result);

                    let exit_status = output.status.code().unwrap_or(-1);
                    let cmd_is_error = !output.status.success();
                    // Store error status but continue accumulating output unless it's a fatal error

                    result.push_str(&format!("Exit code: {}\n", exit_status));

                    if !stdout.is_empty() {
                        result.push_str(&format!("\nStandard output:\n{}", stdout));
                    }

                    if !stderr.is_empty() {
                        result.push_str(&format!("\nStandard error:\n{}\n", stderr));
                    }

                    // If a command fails, stop executing
                    // If a command fails, stop executing and return the accumulated output + error
                    if cmd_is_error {
                         // Use bail! which converts to the appropriate error type for Result<CallToolResult>
                         mcp_attr::bail!("Command failed with exit code {}. Output:\n{}", exit_status, result);
                    }
                },
                Err(e) => {
                    result.push_str(&format!("Failed to execute command '{}': {}\n", cmd, e));
                     // Use bail! which converts to the appropriate error type for Result<CallToolResult>
                     mcp_attr::bail!("Failed to execute command '{}': {}", cmd, e);
                }
            }
        }

        // Drop the lock explicitly before returning Ok
        drop(server_state);

        // If all commands succeeded
        // Wrap the final string result in CallToolResult
        Ok(CallToolResult::from(result))
    }

    /// Edit a file by applying a unified diff to it.
    #[tool]
    async fn tool_edit_file(&self, file_path: String, diff: String) -> Result<CallToolResult> {
        let current_dir = self.0.lock().unwrap().current_working_dir.clone();
        let file_path_buf = resolve_path(&current_dir, &file_path);
        let display_path = file_path_buf.display().to_string();

        match fs::read_to_string(&file_path_buf) {
            Ok(original_content) => {
                match apply_diff(&original_content, &diff) {
                    Ok(new_content) => {
                        match fs::write(&file_path_buf, new_content) {
                            Ok(_) => Ok(CallToolResult::from(format!("Successfully applied diff to file: {}", display_path))), // Wrap
                            Err(e) => mcp_attr::bail!("Error writing to file '{}': {}", display_path, e),
                        }
                    },
                    Err(e) => mcp_attr::bail!("Error applying diff: {}", e), // bail! handles conversion
                }
            },
            Err(e) => mcp_attr::bail!("Error reading file '{}': {}", display_path, e), // bail! handles conversion
        }
    }

    /// Write content to a file using the current working directory.
    #[tool] // Rename
    async fn tool_write_file(&self, file_path: String, content: String) -> Result<CallToolResult> { // Revert to CallToolResult
        let current_dir = self.0.lock().unwrap().current_working_dir.clone();
        let file_path_buf = resolve_path(&current_dir, &file_path);
        let display_path = file_path_buf.display().to_string();

        if let Some(parent) = file_path_buf.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    mcp_attr::bail!("Error creating directory structure for '{}': {}", display_path, e); // bail! handles conversion
                }
            }
        }

        match fs::write(&file_path_buf, &content) {
            Ok(_) => Ok(CallToolResult::from(format!("Successfully wrote to file: {}", display_path))), // Wrap
            Err(e) => mcp_attr::bail!("Error writing to file '{}': {}", display_path, e), // bail! handles conversion
        }
    }

    /// Check code for errors after editing. For Rust projects, runs 'cargo check'.
    #[tool] // Rename
    async fn tool_check_code(&self) -> Result<CallToolResult> { // Revert to CallToolResult
        let current_dir = self.0.lock().unwrap().current_working_dir.clone();
        let cargo_toml_path = current_dir.join("Cargo.toml");

        if !cargo_toml_path.exists() {
             mcp_attr::bail!("No Cargo.toml found in '{}'. This doesn't appear to be a Rust project.", current_dir.display()); // bail! handles conversion
        }

        // Execute 'cargo check' by calling the execute_bash tool method
        // The execute_bash function already returns Result<CallToolResult>
        self.tool_execute_bash("cargo check".to_string()).await // Returns Result<CallToolResult>
    }

    /// Parse code using TreeSitter to extract structure.
    #[tool] // Rename
    async fn tool_parse_code(&self, file_path: String, project_path: Option<String>) -> Result<CallToolResult> { // Revert to CallToolResult
        let current_dir = self.0.lock().unwrap().current_working_dir.clone();
        let mut diagnostic_info = String::new(); // Keep diagnostics local

        // Determine project directory logic (simplified)
        // Use as_ref() to borrow project_path without moving it
        let project_dir = project_path.as_ref()
            .map(|p| resolve_path(&current_dir, p))
            .unwrap_or_else(|| current_dir.clone());

        // Resolve file path relative to project_dir if project_path was given, else relative to current_dir
        let file_path_buf = if project_path.is_some() { // Check original Option
             resolve_path(&project_dir, &file_path) // file_path relative to project_dir
        } else {
             resolve_path(&current_dir, &file_path) // file_path relative to current_dir
        };

        diagnostic_info.push_str(&format!("Resolved file path: {}\n", file_path_buf.display()));

        if !file_path_buf.exists() || !file_path_buf.is_file() {
             mcp_attr::bail!("Error: File '{}' not found or is not a file.\n\nDiagnostic Info:\n{}", file_path_buf.display(), diagnostic_info); // bail! handles conversion
        }

        match treesitter::parse_file(&file_path_buf, None) { // Assuming parse_file doesn't need project_path explicitly anymore
            Some(file_info) => {
                match serde_json::to_string_pretty(&file_info) {
                    Ok(json_string) => Ok(CallToolResult::from(json_string)), // Wrap
                    Err(e) => mcp_attr::bail!("Error serializing parse results: {}\n\nDiagnostic Info:\n{}", e, diagnostic_info), // bail! handles conversion
                }
            },
            None => mcp_attr::bail!("Error parsing file: Could not parse '{}'\n\nDiagnostic Info:\n{}", file_path_buf.display(), diagnostic_info), // bail! handles conversion
        }
    }

    /// Read a file's contents, up to a character limit.
    #[tool]
    async fn tool_read_file(&self, file_path: String, max_chars: Option<usize>) -> Result<CallToolResult> { // Revert to CallToolResult
        let current_dir = self.0.lock().unwrap().current_working_dir.clone();
        let file_path_buf = resolve_path(&current_dir, &file_path);
        let display_path = file_path_buf.display().to_string();

        if !file_path_buf.exists() || !file_path_buf.is_file() {
            mcp_attr::bail!("Error: File '{}' not found or is not a file.", display_path); // bail! handles conversion
        }

        match fs::read_to_string(&file_path_buf) {
            Ok(content) => {
                let limit = max_chars.unwrap_or(1000); // Default limit
                let truncated = if content.chars().count() > limit { // Use chars().count() for accurate character limit
                    content.chars().take(limit).collect::<String>() + "\n\n(content truncated due to size limit)"
                } else {
                    content
                };
                Ok(CallToolResult::from(truncated)) // Wrap
            },
            Err(e) => mcp_attr::bail!("Error reading file '{}': {}", display_path, e), // bail! handles conversion
        }
    }
    // --- Crates.io Tool Implementations ---
    // Note: These tools now return Result<Value> or Result<String> directly.
    // Error handling uses mcp_attr::bail! or returns Err(...)

    /// Search for packages on crates.io
    #[tool] // Rename
    async fn tool_search_crates(&self, args: SearchCratesArgs) -> Result<String> {
        let mut query_params = HashMap::new();
        query_params.insert("q".to_string(), args.query.clone());
        if let Some(page) = args.page {
            query_params.insert("page".to_string(), page.to_string());
        }
        if let Some(per_page) = args.per_page {
            query_params.insert("per_page".to_string(), per_page.to_string());
        }
        let options = RequestOptions { params: Some(query_params), ..Default::default() };

        match CratesIoClient::get("crates", Some(options)).await {
            Ok(response) => match response {
                FetchResponse::Json { data, status, .. } => {
                    let json_string = match serde_json::to_string_pretty(&data) {
                        Ok(s) => s,
                        Err(e) => mcp_attr::bail!("Error serializing JSON response: {}", e),
                    };
                    Ok(format!("Status: {}\n\n{}", status, json_string))
                },
                FetchResponse::Text { data, status, .. } => {
                    Ok(format!("Status: {}\n{}", status, data))
                }
            },
            Err(e) => mcp_attr::bail!("Error searching crates: {}", e),
        }
    }

    /// Get detailed information about a specific crate
    #[tool]
    async fn tool_get_crate(&self, args: GetCrateArgs) -> Result<String> {
        let path = format!("crates/{}", args.crate_name);

        match CratesIoClient::get(&path, None).await {
            Ok(response) => match response {
                FetchResponse::Json { data, status, .. } => {
                    let json_string = match serde_json::to_string_pretty(&data) {
                        Ok(s) => s,
                        Err(e) => mcp_attr::bail!("Error serializing JSON response: {}", e),
                    };
                    Ok(format!("Status: {}\n\n{}", status, json_string))
                },
                FetchResponse::Text { data, status, .. } => {
                    Ok(format!("Status: {}\n{}", status, data))
                }
            },
            Err(e) => mcp_attr::bail!("Error getting crate details: {}", e),
        }
    }

    /// Get all versions of a specific crate
    #[tool]
    async fn tool_get_crate_versions(&self, args: GetCrateVersionsArgs) -> Result<String> { // Reverted to CallToolResult
        let path = format!("crates/{}/versions", args.crate_name);

        match CratesIoClient::get(&path, None).await {
            Ok(response) => match response {
                FetchResponse::Json { data, status, .. } => {
                     let json_string = serde_json::to_string_pretty(&data)?;
                    Ok(format!("Status: {}\n\n{}", status, json_string))
                },
                FetchResponse::Text { data, status, .. } => {
                     Ok(format!("Status: {}\n{}", status, data) )
                }
            },
            Err(e) => mcp_attr::bail!("Error getting crate versions: {}", e),
        }
    }

     /// Get dependencies for a specific version of a crate
    #[tool] 
    async fn tool_get_crate_dependencies(&self, args: GetCrateDependenciesArgs) -> Result<String> { // Revert to CallToolResult
        let path = format!("crates/{}/{}/dependencies", args.crate_name, args.version);

        match CratesIoClient::get(&path, None).await {
            Ok(response) => match response {
                FetchResponse::Json { data, status, .. } => {
                     let json_string = serde_json::to_string_pretty(&data)?;

                    Ok(format!("Status: {}\n\n{}", status, json_string))
                },
                FetchResponse::Text { data, status, .. } => {
                     Ok(format!("Status: {}\n{}", status, data))
                }
            },
            Err(e) => mcp_attr::bail!("Error getting crate dependencies: {}", e),
        }
    }

    /// Lookup documentation for a Rust crate from docs.rs
    #[tool]
    async fn tool_lookup_crate_docs(&self, args: LookupCrateDocsArgs) -> Result<CallToolResult> { // Revert to CallToolResult
        let crate_name = args.crate_name.unwrap_or_else(|| "tokio".to_string());
        // Construct URL for the latest version explicitly
        let url = format!("https://docs.rs/{}/latest/{}/", crate_name, crate_name.replace('-', "_")); // Use crate name slug for path


        // Explicitly handle client build error with match
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
        {
            Ok(c) => c,
            Err(e) => mcp_attr::bail!("Failed to build reqwest client: {}", e),
        };

        match client.get(&url).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    let error_text = format!("Error: Could not fetch documentation from {}. HTTP status: {}", url, response.status());
                     mcp_attr::bail_public!(mcp_attr::ErrorCode::INTERNAL_ERROR, "{}", error_text);
                }

                match response.text().await {
                    Ok(html_content) => {
                        let text_content = html2text::from_read(html_content.as_bytes(), 130);

                        const MAX_LENGTH: usize = 8000;
                        let truncated_text = if text_content.chars().count() > MAX_LENGTH { // Use chars().count()
                            format!("{}\n\n[Content truncated. Full documentation available at {}]", text_content.chars().take(MAX_LENGTH).collect::<String>(), url)
                        } else {
                            text_content
                        };

                        Ok(CallToolResult::from(truncated_text)) // Wrap
                    }
                    Err(e) => {
                         mcp_attr::bail!("Error reading documentation content: {}", e)
                    }
                }
            }
            Err(e) => {
                 mcp_attr::bail!("Error fetching documentation from {}: {}", url, e)
            }
        }
    }




}

#[tokio::main]
async fn main() -> Result<()> {

    // Initialize server state
    let server_data = ServerData {
        current_working_dir: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        http_client: reqwest::Client::new(),
    };
    let server = CorrodeMcpServer(Mutex::new(server_data));

    serve_stdio(server).await?;

    Ok(())
}
