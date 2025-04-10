use std::path::{Path, PathBuf};
use mcp_attr::Result;
use mcp_attr::server::{mcp_server, McpServer};
use mcp_attr::schema::{GetPromptResult, CallToolResult};
use std::sync::Mutex;
use std::collections::HashMap;
use serde::Deserialize;
use schemars::JsonSchema;
use reqwest;
use crate::mcp::crates_io::{CratesIoClient, RequestOptions, FetchResponse};
use crate::mcp::function_signatures;
use crate::mcp::patch::{parse_hunks, find_candidates, rebuild_hunks, rebuild_patch};
use std::fs;
use std::process::Command;
use crate::mcp::prompts::{CODE_CHANGE_WORKFLOW, MCP_TOOLS_GUIDE};


pub mod mcp;


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
struct ListFunctionSignaturesArgs {
    /// Optional specific file to check
    file_path: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct LookupCrateDocsArgs {
    #[serde(rename = "crateName")]
    crate_name: Option<String>,
}


pub struct ServerData {
    pub current_working_dir: PathBuf,
    pub http_client: reqwest::Client,
}

pub struct CorrodeMcpServer(pub Mutex<ServerData>);

#[mcp_server]
impl McpServer for CorrodeMcpServer {
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

    /// Prompt the user for the directory to change to.
    #[prompt]
    async fn cd(
        &self,
        /// The target directory path
        target_directory: String,
    ) -> Result<GetPromptResult> {
        let prompt_text = format!("Please enter the full path to the project directory you want to change to, starting from: {}", target_directory);
        Ok(GetPromptResult::from(prompt_text))
    }

    /// Get the code change workflow guidance
    #[prompt]
    async fn code_change_workflow(
        &self,
        /// Optional aspect of the workflow to focus on
        _aspect: Option<String>,
    ) -> Result<GetPromptResult> {
        let workflow = CODE_CHANGE_WORKFLOW;
        
        // Return the workflow as a prompt
        Ok(GetPromptResult::from(workflow))
    }

    /// Get comprehensive MCP tools usage guide
    #[prompt]
    async fn mcp_tools_guide(
        &self,
        /// Optional specific tool to get guidance for
        _tool: Option<String>,
    ) -> Result<GetPromptResult> {
        let guide = MCP_TOOLS_GUIDE;
        
        // If a specific tool was requested, try to find that section
        // For now, we'll just return the full guide
        // In a future enhancement, this could extract just the relevant section
        
        // Return the guide as a prompt
        Ok(GetPromptResult::from(guide))
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

    // --- Tool Implementations ---

    /// Execute a command using bash shell. Handles 'cd' to change server's working directory.
    #[tool] 
    async fn execute_bash(&self, command: String) -> Result<CallToolResult> { 
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
            let current_dir_path = server_state.current_working_dir.clone(); 

            // Check if command is a cd command and update working directory if it is
            if let Some(new_dir) = handle_cd_command(&current_dir_path, cmd) {
                // Try to actually change to this directory to verify it exists
                if new_dir.exists() && new_dir.is_dir() {
                    // Update the server state's CWD
                    server_state.current_working_dir = new_dir.clone();
                    result.push_str(&format!("Changed directory to: {}\n", new_dir.display()));
                } else {
                    // Enhanced error message for CD failures with more context
                    let error_message = format!(
                        "Directory change failed:\n- Command: '{}'\n- Target: {}\n- Current directory: {}\n- Error: The specified directory does not exist or is not accessible",
                        cmd,
                        new_dir.display(),
                        current_dir_path.display()
                    );
                    
                    result.push_str(&format!("{}\n", error_message));
                    
                    // Stop executing further commands if cd fails
                    // Use bail! which converts to the appropriate error type for Result<CallToolResult>
                    mcp_attr::bail!("{}", error_message);
                }

                // If this is a pure cd command, we're done with this part of the sequence
                if cmd == "cd" || (cmd.starts_with("cd ") && !cmd.contains("&&") && !cmd.contains(';')) {
                     continue;
                }
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

                    // If a command fails, stop executing and return the accumulated output + error
                    if cmd_is_error {
                         // Include both stdout and stderr in the error message for better debugging
                         let error_message = format!("Command '{}' failed with exit code {}.\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
                             cmd,
                             exit_status,
                             stdout,
                             stderr
                         );
                         
                         // Use bail! which converts to the appropriate error type for Result<CallToolResult>
                         result.push_str(&format!("{}", error_message));
                    }
                },
                Err(e) => {
                    // Enhanced error message for command execution failure
                    let error_details = format!(
                        "Failed to execute command '{}':\n- Error: {}\n- Working Directory: {}\n- Note: This typically happens when the command or shell is not found, or due to permissions issues",
                        cmd,
                        e,
                        current_dir_path.display()
                    );
                    
                    result.push_str(&format!("{}\n", error_details));
                    
                    // Use bail! which converts to the appropriate error type for Result<CallToolResult>
                    mcp_attr::bail!("{}", error_details);
                }
            }
        }

        // Drop the lock explicitly before returning Ok
        drop(server_state);

        // If all commands succeeded
        // Wrap the final string result in CallToolResult
        Ok(CallToolResult::from(result))
    }

    /// Replace content with a Unified format git patch.
    ///
    /// Use this tool to make multiple edits in a file.
    #[tool]
    /// Here is an example of a Unified format git patch:
    ///
    /// ```patch
    /// --- a/src/evaluations/patch.rs
    /// +++ b/src/evaluations/patch.rs
    /// @@ -43,6 +43,6 @@ fn prompt() -> String {
    ///             self._content_consumed = True
    /// 
    /// -        Apply only these fixes, do not make any other changes to the code. The file is long and the modifications are small.
    /// +        Apply only these fixes, do not make any other changes to the code. The file is long and the modifications are small. Start by reading the file.
    ///     \"}.to_string()
    /// }
    /// 
    /// ```
    async fn patch_file(&self,
        /// Full path of the file
        file_name: String,
        /// Unified format git patch to apply
        patch: String) -> Result<CallToolResult> {
        // Get the current working directory
        let current_dir = self.0.lock().unwrap().current_working_dir.clone();
        let file_path_buf = resolve_path(&current_dir, &file_name);
        let display_path = file_path_buf.display().to_string();

        // Read the original content
        let mut old_content = match fs::read_to_string(&file_path_buf) {
            Ok(content) => content,
            Err(e) => mcp_attr::bail!("Failed to read file {}: {}", display_path, e),
        };

        // Patches are very strict on the last line being a newline
        if !old_content.ends_with('\n') {
            old_content.push('\n');
        }

        // Parse the patch hunks
        let old_hunks = match parse_hunks(&patch) {
            Ok(hunks) => hunks,
            Err(e) => mcp_attr::bail!("Failed to parse patch: {}", e),
        };

        // Find candidates for each hunk in the file
        let candidates = find_candidates(&old_content, &old_hunks);
        
        // Rebuild the hunks with corrected line numbers
        let new_hunks = rebuild_hunks(&candidates);

        // Rebuild the patch with correct line numbers
        let updated_patch = match rebuild_patch(&patch, &new_hunks) {
            Ok(patch) => patch,
            Err(e) => mcp_attr::bail!("Failed to render fixed patch: {}", e),
        };

        // Parse the patch using diffy
        let diffy_patch = match diffy::Patch::from_str(&updated_patch) {
            Ok(patch) => patch,
            Err(e) => mcp_attr::bail!("Failed to parse patch: {}", e),
        };

        // Apply the patch
        let patched = match diffy::apply(&old_content, &diffy_patch) {
            Ok(patched) => patched,
            Err(e) => mcp_attr::bail!("Failed to apply patch: {}", e),
        };

        // Write the patched content to the file
        match fs::write(&file_path_buf, &patched) {
            Ok(_) => {
                if new_hunks.len() != old_hunks.len() {
                    let failed = old_hunks
                        .iter()
                        .filter(|h| !new_hunks.iter().any(|h2| h2.body == h.body))
                        .collect::<Vec<_>>();
    
                    return Ok(CallToolResult::from(format!(
                        "Failed to apply all hunks. {} hunks failed to apply.\n\nThe following hunks failed to apply as their context lines could not be matched to the file, no changes were applied:\n\n---\n{}\n---\n\nMake sure all lines are correct. Are you also sure that the changes have not been applied already?",
                        failed.len(),
                        failed.iter().map(|h| h.body.as_str()).collect::<Vec<_>>().join("\n")
                    )));
                }
    
                Ok(CallToolResult::from(format!("Patch applied successfully to {}", display_path)))
            },
            Err(e) => mcp_attr::bail!("Error writing to file '{}': {}", display_path, e),
        }
    }

    /// Write content to a file using the current working directory. use this to write new files or completely overwrite existing files.
    #[tool]
    async fn write_file(&self, file_path: String, content: String) -> Result<CallToolResult> {
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
    /// Use this after making edits to verify your changes compile correctly.
    #[tool]
    async fn check_code(&self) -> Result<CallToolResult> { 
        let current_dir = self.0.lock().unwrap().current_working_dir.clone();
        let cargo_toml_path = current_dir.join("Cargo.toml");

        if !cargo_toml_path.exists() {
             mcp_attr::bail!("No Cargo.toml found in '{}'. This doesn't appear to be a Rust project.", current_dir.display()); // bail! handles conversion
        }

        self.execute_bash("cargo check".to_string()).await // Returns Result<CallToolResult>
    }

    /// Reads file content.
    ///
    /// Returns the content of a file at the specified path.
    /// Provides the complete file content without truncation.
    #[tool]
    async fn read_file(&self, file_path: String) -> Result<CallToolResult> {
        let current_dir = self.0.lock().unwrap().current_working_dir.clone();
        let file_path_buf = resolve_path(&current_dir, &file_path);
        let display_path = file_path_buf.display().to_string();

        match fs::read_to_string(&file_path_buf) {
            Ok(content) => {
                // Return the full content without any truncation
                Ok(CallToolResult::from(content)) // Wrap
            },
            Err(e) => mcp_attr::bail!("Error reading file '{}': {}", display_path, e), // bail! handles conversion
        }
    }
    // --- Crates.io Tool Implementations ---
    // Note: These tools now return Result<Value> or Result<String> directly.
    // Error handling uses mcp_attr::bail! or returns Err(...)
    // #[resource("crates.io://{query}/{page}/{per_page}")]

    /// Search for packages on crates.io
    #[tool]
    async fn tool_search_crates(&self, args: SearchCratesArgs) -> Result<String> {
        let mut query_params = HashMap::new();
        query_params.insert("q".to_string(), args.query.clone());
        
        // Create a crates.io client in a separate scope to ensure MutexGuard is dropped
        let crates_client = {
            let server_data = self.0.lock().unwrap();
            CratesIoClient::with_client(server_data.http_client.clone())
        }; // server_data is dropped here when the block ends
        
        if let Some(page) = args.page {
            query_params.insert("page".to_string(), page.to_string());
        }
        if let Some(per_page) = args.per_page {
            query_params.insert("per_page".to_string(), per_page.to_string());
        }
        let options = RequestOptions { params: Some(query_params), ..Default::default() };
        
        match crates_client.get("crates", Some(options)).await {
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

    /// Get detailed information about a specific crate, use this to find more about a crate
    #[tool]
    async fn get_crate(&self, args: GetCrateArgs) -> Result<String> {
        // Scope the mutex guard to ensure it's dropped before any await points
        let (crates_client, path) = {
            let server_data = self.0.lock().unwrap();
            let client = CratesIoClient::with_client(server_data.http_client.clone());
            let path_str = format!("crates/{}", args.crate_name);
            (client, path_str)
        };
        
        match crates_client.get(&path, None).await {
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

    /// Get all versions of a specific crate, use this before adding a dependency to ensure you're using the latest version
    #[tool]
    async fn get_crate_versions(&self, args: GetCrateVersionsArgs) -> Result<String> {
        // Scope the mutex guard to ensure it's dropped before any await points
        let (crates_client, path) = {
            let server_data = self.0.lock().unwrap();
            let client = CratesIoClient::with_client(server_data.http_client.clone());
            let path_str = format!("crates/{}/versions", args.crate_name);
            (client, path_str)
        };
        
        match crates_client.get(&path, None).await {
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
    async fn get_crate_dependencies(&self, args: GetCrateDependenciesArgs) -> Result<String> {
        // Scope the mutex guard to ensure it's dropped before any await points
        let (crates_client, path) = {
            let server_data = self.0.lock().unwrap();
            let client = CratesIoClient::with_client(server_data.http_client.clone());
            let path_str = format!("crates/{}/{}/dependencies", args.crate_name, args.version);
            (client, path_str)
        };
        
        match crates_client.get(&path, None).await {
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

    /// Lookup documentation for a Rust crate from docs.rs, use this if you're having problems with a crates APIs
    #[tool]
    async fn lookup_crate_docs(&self, args: LookupCrateDocsArgs) -> Result<CallToolResult> {
        let crate_name = args.crate_name.unwrap_or_else(|| "tokio".to_string());
        let url = format!("https://docs.rs/{}/latest/{}/", crate_name, crate_name.replace('-', "_"));

        // Get client but release lock before any async operations
        let client = {
            let server_state = self.0.lock().unwrap();
            server_state.http_client.clone()
        };
        
        match client.get(&url).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    let error_text = format!("Error: Could not fetch documentation from {}. HTTP status: {}", url, response.status());
                     mcp_attr::bail_public!(mcp_attr::ErrorCode::INTERNAL_ERROR, "{}", error_text);
                }

                match response.text().await {
                    Ok(html_content) => {
                        // Convert HTML to text
                        let html_result = html2text::from_read(html_content.as_bytes(), 130);
                        if let Err(e) = &html_result {
                            mcp_attr::bail!("Error converting HTML to text: {}", e);
                        }
                        let text_content = html_result.unwrap();

                        // Truncate if too long
                        const MAX_LENGTH: usize = 8000;
                        let truncated_text = if text_content.chars().count() > MAX_LENGTH {
                            format!("{}\n\n[Content truncated. Full documentation available at {}]",
                                text_content.chars().take(MAX_LENGTH).collect::<String>(), url)
                        } else {
                            text_content
                        };
                        Ok(CallToolResult::from(truncated_text))
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

    /// List function signatures found in the current project directory.
    #[tool]
    async fn list_function_signatures(&self, args: Option<ListFunctionSignaturesArgs>) -> Result<CallToolResult> {
        let current_dir = self.0.lock().unwrap().current_working_dir.clone();
        
        // Output diagnostic info
        let mut result_string = format!("Current working directory: {}\n\n", current_dir.display());
        
        let signatures = if let Some(args) = args {
            if let Some(file_path) = args.file_path {
                let file_path_buf = resolve_path(&current_dir, &file_path);
                result_string.push_str(&format!("Checking specific file: {}\n\n", file_path_buf.display()));
                
                if !file_path_buf.exists() {
                    return Ok(CallToolResult::from(format!(
                        "Error: File '{}' does not exist.",
                        file_path_buf.display()
                    )));
                }
                
                function_signatures::extract_function_signatures(&file_path_buf, None)
            } else {
                result_string.push_str("Scanning entire project directory\n\n");
                function_signatures::extract_project_signatures(&current_dir)
            }
        } else {
            result_string.push_str("Scanning entire project directory\n\n");
            function_signatures::extract_project_signatures(&current_dir)
        };

        if signatures.is_empty() {
            result_string.push_str("No function signatures found.");
            return Ok(CallToolResult::from(result_string));
        }

        // Format the signatures into a string
        result_string.push_str(&format!("Found {} function signatures:\n\n", signatures.len()));
        
        for sig in signatures {
            // Format: path/to/file.rs:line_number - signature
            let formatted_line = format!(
                "{}:{}: {}\n",
                sig.file_path,
                sig.line_number,
                sig.signature.trim() // Trim whitespace from the signature line
            );
            result_string.push_str(&formatted_line);
        }

        Ok(CallToolResult::from(result_string))
    }

}
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

