use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use anyhow::Result;
use rmcp::{
    Error as McpError, RoleServer, ServerHandler, model::*, schemars, service::RequestContext, tool,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

#[derive(Default)]
struct RequestOptions {
    params: Option<HashMap<String, String>>,
}

pub mod mcp;
pub mod vendor;

// --- Argument Structs for Tools (derive Deserialize and JsonSchema) ---

#[derive(Deserialize, JsonSchema)]
pub struct SearchCratesArgs {
    query: String,
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetCrateArgs {
    crate_name: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetCrateVersionsArgs {
    crate_name: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetCrateDependenciesArgs {
    crate_name: String,
    version: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct ListFunctionSignaturesArgs {}

#[derive(Deserialize, JsonSchema)]
pub struct LookupCrateDocsArgs {
    #[serde(rename = "crateName")]
    crate_name: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ProbeSearchArgs {
    /// The search query to run
    query: String,
    /// Optional file patterns to filter (e.g. ["*.rs", "*.toml"])
    file_patterns: Option<Vec<String>>,
    /// Maximum number of results to return
    max_results: Option<usize>,
    /// Optional language filter (e.g. "rust", "python")
    language: Option<String>,
}

pub struct ServerData {
    pub current_working_dir: PathBuf,
    pub http_client: reqwest::Client,
}

// Manual implementation of Clone for CorrodeMcpServer
impl Clone for CorrodeMcpServer {
    fn clone(&self) -> Self {
        // Cloning the Arc inside the Mutex
        CorrodeMcpServer(Mutex::new(ServerData {
            current_working_dir: self.0.lock().unwrap().current_working_dir.clone(),
            http_client: self.0.lock().unwrap().http_client.clone(),
        }))
    }
}

pub struct CorrodeMcpServer(pub Mutex<ServerData>);

#[tool(tool_box)]
impl CorrodeMcpServer {
    /// Execute a command using bash shell. Handles 'cd' to change server's working directory.
    #[tool(
        description = "Execute a command using bash shell. Handles 'cd' to change server's working directory."
    )]
    async fn execute_bash(
        &self,
        #[tool(param)] command: String,
    ) -> Result<CallToolResult, McpError> {
        // Debug flag - can be made configurable in the future
        let debug_enabled = false;

        let mut result = String::new();
        // Flag to track if any command has failed

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

            // Create execution context for debugging and error reporting
            let execution_context =
                mcp::bash_exec::CommandExecutionContext::new(cmd, &current_dir_path);

            // Check if command is a cd command and update working directory if it is
            if let Some(new_dir) = handle_cd_command(&current_dir_path, cmd) {
                // Log path resolution details if debug is enabled
                // Try to actually change to this directory to verify it exists
                if new_dir.exists() && new_dir.is_dir() {
                    // Update the server state's CWD
                    server_state.current_working_dir = new_dir.clone();
                    result.push_str(&format!("Changed directory to: {}\n", new_dir.display()));
                } else {
                    // Simple error message for now - avoid complex error handling that might cause MCP errors
                    let error_message = format!(
                        "Directory not found: The directory '{}' does not exist or is not accessible.\nWorking directory: {}\nSuggestion: Verify the path exists before accessing it.",
                        new_dir.display(),
                        current_dir_path.display()
                    );
                    result.push_str(&format!("\n{}\n", error_message));

                    // Return early but don't use bail! - include the error in the result
                    return Ok(CallToolResult::success(vec![Content::text(result)]));
                }

                // If this is a pure cd command, we're done with this part of the sequence
                if cmd == "cd"
                    || (cmd.starts_with("cd ") && !cmd.contains("&&") && !cmd.contains(';'))
                {
                    continue;
                }
            }

            // Add debug information if enabled
            if debug_enabled {
                result.push_str(&format!("\n[DEBUG] Executing command: '{}'\n", cmd));
                result.push_str(&format!(
                    "[DEBUG] Working directory: {}\n",
                    current_dir_path.display()
                ));

                if execution_context.is_compound_command {
                    result.push_str("[DEBUG] This is a compound command that will be executed as a single unit by bash\n");
                }

                result.push_str(&format!(
                    "[DEBUG] Command executable: '{}'\n",
                    execution_context.executable()
                ));

                if !execution_context.command_parts.is_empty() {
                    result.push_str(&format!(
                        "[DEBUG] Command arguments: {}\n",
                        execution_context.command_parts[1..].join(" ")
                    ));
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

                    // Format exit code based on success/failure
                    if cmd_is_error {
                        result.push_str(&format!("Exit code: {} (ERROR)\n", exit_status));
                    } else {
                        result.push_str(&format!("Exit code: {}\n", exit_status));
                    }
                    if !stdout.is_empty() {
                        result.push_str(&format!("\nStandard output:\n{}", stdout));
                    }

                    if !stderr.is_empty() {
                        result.push_str(&format!("\nStandard error:\n{}\n", stderr));
                    }

                    // If a command fails, add enhanced error info but don't break execution
                    if !output.status.success() {
                        // Add a simplified error message with context
                        let error_message = format!(
                            "Command execution failed: '{}' (exit code: {})\nWorking directory: {}\nSuggestion: Check command arguments and verify inputs are valid.",
                            cmd,
                            exit_status,
                            current_dir_path.display()
                        );

                        result.push_str(&format!("\n{}\n", error_message));
                    }
                }
                Err(e) => {
                    let error_message = format!(
                        "Failed to execute command '{}': {}\nWorking directory: {}\nSuggestion: Check if the command exists and you have permissions to run it.",
                        cmd,
                        e,
                        current_dir_path.display()
                    );

                    result.push_str(&format!("{}\n", error_message));
                    // Break from the loop to stop executing further commands after an error
                    break;
                }
            }
        }

        // Drop the lock explicitly before returning Ok
        drop(server_state);

        // Wrap the final string result in CallToolResult
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    /// Replace content with a Unified format git patch.
    ///
    /// Use this tool to make multiple edits in a file.
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
    #[tool(
        description = "Replace content with a Unified format git patch.\n\nUse this tool to make multiple edits in a file.\nHere is an example of a Unified format git patch:\n\n```patch\n--- a/src/evaluations/patch.rs\n+++ b/src/evaluations/patch.rs\n@@ -43,6 +43,6 @@ fn prompt() -> String {\n            self._content_consumed = True\n\n-        Apply only these fixes, do not make any other changes to the code. The file is long and the modifications are small.\n+        Apply only these fixes, do not make any other changes to the code. The file is long and the modifications are small. Start by reading the file.\n    \\\"}.to_string()\n}\n\n```"
    )]
    async fn patch_file(
        &self,
        #[tool(param)]
        #[schemars(description = "Full path of the file")]
        file_name: String,
        #[tool(param)]
        #[schemars(description = "Unified format git patch to apply")]
        patch: String,
    ) -> Result<CallToolResult, McpError> {
        // Get the current working directory
        let current_dir = self.0.lock().unwrap().current_working_dir.clone();
        let file_path_buf = resolve_path(&current_dir, &file_name);
        let display_path = file_path_buf.display().to_string();

        // Read the original content
        let mut old_content = match fs::read_to_string(&file_path_buf) {
            Ok(content) => content,
            Err(e) => {
                let error_message = format!("Failed to read file {}: {}", display_path, e);
                return Ok(CallToolResult::success(vec![Content::text(error_message)]));
            }
        };

        // Patches are very strict on the last line being a newline
        if !old_content.ends_with('\n') {
            old_content.push('\n');
        }

        // Parse the patch hunks
        let old_hunks = match mcp::patch::parse_hunks(&patch) {
            Ok(hunks) => hunks,
            Err(e) => {
                let error_message = format!("Failed to parse patch: {}", e);
                return Ok(CallToolResult::success(vec![Content::text(error_message)]));
            }
        };

        // Find candidates for each hunk in the file
        let candidates = mcp::patch::find_candidates(&old_content, &old_hunks);

        // Rebuild the hunks with corrected line numbers
        let new_hunks = mcp::patch::rebuild_hunks(&candidates);

        // Rebuild the patch with correct line numbers
        let updated_patch = match mcp::patch::rebuild_patch(&patch, &new_hunks) {
            Ok(patch) => patch,
            Err(e) => {
                let error_message = format!("Failed to render fixed patch: {}", e);
                return Ok(CallToolResult::success(vec![Content::text(error_message)]));
            }
        };

        // Parse the patch using diffy
        let diffy_patch = match diffy::Patch::from_str(&updated_patch) {
            Ok(patch) => patch,
            Err(e) => {
                let error_message = format!("Failed to parse patch: {}", e);
                return Ok(CallToolResult::success(vec![Content::text(error_message)]));
            }
        };

        // Apply the patch
        let patched = match diffy::apply(&old_content, &diffy_patch) {
            Ok(patched) => patched,
            Err(e) => {
                let error_message = format!("Failed to apply patch: {}", e);
                return Ok(CallToolResult::success(vec![Content::text(error_message)]));
            }
        };

        // Write the patched content to the file
        match fs::write(&file_path_buf, &patched) {
            Ok(_) => {
                if new_hunks.len() != old_hunks.len() {
                    let failed = old_hunks
                        .iter()
                        .filter(|h| !new_hunks.iter().any(|h2| h2.body == h.body))
                        .collect::<Vec<_>>();

                    let error_message = format!(
                        "Failed to apply all hunks. {} hunks failed to apply.\n\nThe following hunks failed to apply as their context lines could not be matched to the file, no changes were applied:\n\n---\n{}\n---\n\nMake sure all lines are correct. Are you also sure that the changes have not been applied already?",
                        failed.len(),
                        failed
                            .iter()
                            .map(|h| h.body.as_str())
                            .collect::<Vec<_>>()
                            .join("\n")
                    );
                    return Ok(CallToolResult::success(vec![Content::text(error_message)]));
                }

                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Patch applied successfully to {}",
                    display_path
                ))]))
            }
            Err(e) => {
                let error_message = format!("Error writing to file '{}': {}", display_path, e);
                Ok(CallToolResult::success(vec![Content::text(error_message)]))
            }
        }
    }

    /// Write content to a file using the current working directory. use this to write new files or completely overwrite existing files.
    #[tool(
        description = "Write content to a file using the current working directory. use this to write new files or completely overwrite existing files, avoid using this unless you've failed using patch_file or it's a new file."
    )]
    async fn write_file(
        &self,
        #[tool(param)] file_path: String,
        #[tool(param)] content: String,
    ) -> Result<CallToolResult, McpError> {
        let current_dir = self.0.lock().unwrap().current_working_dir.clone();
        let file_path_buf = resolve_path(&current_dir, &file_path);
        let display_path = file_path_buf.display().to_string();

        if let Some(parent) = file_path_buf.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    let error_message = format!(
                        "Error creating directory structure for '{}': {}",
                        display_path, e
                    );
                    return Ok(CallToolResult::success(vec![Content::text(error_message)]));
                }
            }
        }

        match fs::write(&file_path_buf, &content) {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Successfully wrote to file: {}",
                display_path
            ))])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error writing to file '{}': {}",
                display_path, e
            ))])),
        }
    }

    /// Check code for errors after editing. For Rust projects, runs 'cargo check'.
    /// Use this after making edits to verify your changes compile correctly.
    #[tool(
        description = "Check code for errors after editing. For Rust projects, runs 'cargo check'.\nUse this after making edits to verify your changes compile correctly."
    )]
    async fn check_code(&self) -> Result<CallToolResult, McpError> {
        let current_dir = self.0.lock().unwrap().current_working_dir.clone();
        let cargo_toml_path = current_dir.join("Cargo.toml");

        if !cargo_toml_path.exists() {
            let error_message = format!(
                "No Cargo.toml found in '{}'. This doesn't appear to be a Rust project.",
                current_dir.display()
            );
            return Ok(CallToolResult::success(vec![Content::text(error_message)]));
        }

        // Call execute_bash and convert its result
        match self.execute_bash("cargo check".to_string()).await {
            Ok(result) => Ok(result),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error: {}",
                e
            ))])),
        }
    }

    /// Reads file content.
    ///
    /// Returns the content of a file at the specified path.
    /// Provides the complete file content without truncation.
    #[tool(
        description = "Reads file content.\n\nReturns the content of a file at the specified path.\nProvides the complete file content without truncation."
    )]
    async fn read_file(
        &self,
        #[tool(param)] file_path: String,
    ) -> Result<CallToolResult, McpError> {
        let current_dir = self.0.lock().unwrap().current_working_dir.clone();
        let file_path_buf = resolve_path(&current_dir, &file_path);
        let display_path = file_path_buf.display().to_string();

        match fs::read_to_string(&file_path_buf) {
            Ok(content) => Ok(CallToolResult::success(vec![Content::text(content)])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error reading file '{}': {}",
                display_path, e
            ))])),
        }
    }

    /// Read file or directory contents, run this when first opening a new directory/project to get context
    ///
    /// Uses yek library to read and limit content to the specified token count.
    /// Enforces a strict 50k token limit.
    #[tool(
        description = "Read file or directory contents, run this when first opening a new directory/project to get context\n\nUses yek library to read and limit content to the specified token count.\nEnforces a strict 50k token limit."
    )]
    async fn read_all(&self) -> Result<CallToolResult, McpError> {
        // Get the current working directory
        let current_dir = self.0.lock().unwrap().current_working_dir.clone();

        // Use the current directory directly for reading
        match mcp::read_all::read_with_yek(&current_dir).await {
            Ok(content) => Ok(CallToolResult::success(vec![Content::text(content)])),
            Err(e) => {
                let error_message = format!(
                    "Error reading '{}': {}\nSuggestion: Check if the directory exists and is accessible. The token limit is fixed at 50k.",
                    current_dir.display(),
                    e
                );
                Ok(CallToolResult::success(vec![Content::text(error_message)]))
            }
        }
    }

    // --- Crates.io Tool Implementations ---

    /// Search for packages on crates.io
    #[tool(description = "Search for packages on crates.io")]
    async fn tool_search_crates(
        &self,
        #[tool(aggr)] args: SearchCratesArgs,
    ) -> Result<CallToolResult, McpError> {
        let mut query_params = HashMap::new();
        query_params.insert("q".to_string(), args.query.clone());

        // Create a crates.io client in a separate scope to ensure MutexGuard is dropped
        let crates_client = {
            let server_data = self.0.lock().unwrap();
            server_data.http_client.clone()
        }; // server_data is dropped here when the block ends

        if let Some(page) = args.page {
            query_params.insert("page".to_string(), page.to_string());
        }
        if let Some(per_page) = args.per_page {
            query_params.insert("per_page".to_string(), per_page.to_string());
        }
        let options = RequestOptions {
            params: Some(query_params),
        };

        let mut request = crates_client.get("https://crates.io/api/v1/crates");
        if let Some(params) = options.params {
            request = request.query(&params);
        }

        match request.send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                match response.json::<serde_json::Value>().await {
                    Ok(data) => match serde_json::to_string_pretty(&data) {
                        Ok(json_string) => Ok(CallToolResult::success(vec![Content::text(
                            format!("Status: {}\n\n{}", status, json_string),
                        )])),
                        Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                            "Error serializing JSON response: {}",
                            e
                        ))])),
                    },
                    Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                        "Error parsing JSON response: {}",
                        e
                    ))])),
                }
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error searching crates: {}",
                e
            ))])),
        }
    }

    /// Get detailed information about a specific crate, use this to find more about a crate
    #[tool(
        description = "Get detailed information about a specific crate, use this to find more about a crate"
    )]
    async fn get_crate(
        &self,
        #[tool(aggr)] args: GetCrateArgs,
    ) -> Result<CallToolResult, McpError> {
        // Scope the mutex guard to ensure it's dropped before any await points
        let crates_client = {
            let server_data = self.0.lock().unwrap();
            server_data.http_client.clone()
        };

        match crates_client
            .get(format!(
                "https://crates.io/api/v1/crates/{}",
                args.crate_name
            ))
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status().as_u16();
                match response.json::<serde_json::Value>().await {
                    Ok(data) => match serde_json::to_string_pretty(&data) {
                        Ok(json_string) => Ok(CallToolResult::success(vec![Content::text(
                            format!("Status: {}\n\n{}", status, json_string),
                        )])),
                        Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                            "Error serializing JSON response: {}",
                            e
                        ))])),
                    },
                    Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                        "Error parsing JSON response: {}",
                        e
                    ))])),
                }
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error getting crate details: {}",
                e
            ))])),
        }
    }

    /// Get all versions of a specific crate, use this before adding a dependency to ensure you're using the latest version
    #[tool(
        description = "Get all versions of a specific crate, use this before adding a dependency to ensure you're using the latest version"
    )]
    async fn get_crate_versions(
        &self,
        #[tool(aggr)] args: GetCrateVersionsArgs,
    ) -> Result<CallToolResult, McpError> {
        // Scope the mutex guard to ensure it's dropped before any await points
        let crates_client = {
            let server_data = self.0.lock().unwrap();
            server_data.http_client.clone()
        };

        match crates_client
            .get(format!(
                "https://crates.io/api/v1/crates/{}/versions",
                args.crate_name
            ))
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status().as_u16();
                match response.json::<serde_json::Value>().await {
                    Ok(data) => match serde_json::to_string_pretty(&data) {
                        Ok(json_string) => Ok(CallToolResult::success(vec![Content::text(
                            format!("Status: {}\n\n{}", status, json_string),
                        )])),
                        Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                            "Error serializing JSON response: {}",
                            e
                        ))])),
                    },
                    Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                        "Error parsing JSON response: {}",
                        e
                    ))])),
                }
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error getting crate versions: {}",
                e
            ))])),
        }
    }

    /// Get dependencies for a specific version of a crate
    #[tool(description = "Get dependencies for a specific version of a crate")]
    async fn get_crate_dependencies(
        &self,
        #[tool(aggr)] args: GetCrateDependenciesArgs,
    ) -> Result<CallToolResult, McpError> {
        // Scope the mutex guard to ensure it's dropped before any await points
        let crates_client = {
            let server_data = self.0.lock().unwrap();
            server_data.http_client.clone()
        };

        match crates_client
            .get(format!(
                "https://crates.io/api/v1/crates/{}/{}/dependencies",
                args.crate_name, args.version
            ))
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status().as_u16();
                match response.json::<serde_json::Value>().await {
                    Ok(data) => match serde_json::to_string_pretty(&data) {
                        Ok(json_string) => Ok(CallToolResult::success(vec![Content::text(
                            format!("Status: {}\n\n{}", status, json_string),
                        )])),
                        Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                            "Error serializing JSON response: {}",
                            e
                        ))])),
                    },
                    Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                        "Error parsing JSON response: {}",
                        e
                    ))])),
                }
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error getting crate dependencies: {}",
                e
            ))])),
        }
    }

    /// Lookup documentation for a Rust crate from docs.rs, use this if you're having problems with a crates APIs
    #[tool(
        description = "Lookup documentation for a Rust crate from docs.rs, use this if you're having problems with a crates APIs"
    )]
    async fn lookup_crate_docs(
        &self,
        #[tool(aggr)] args: LookupCrateDocsArgs,
    ) -> Result<CallToolResult, McpError> {
        let crate_name = args.crate_name.unwrap_or_else(|| "tokio".to_string());
        let url = format!(
            "https://docs.rs/{}/latest/{}/",
            crate_name,
            crate_name.replace('-', "_")
        );

        // Get client but release lock before any async operations
        let client = {
            let server_state = self.0.lock().unwrap();
            server_state.http_client.clone()
        };

        match client.get(&url).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    let error_text = format!(
                        "Error: Could not fetch documentation from {}. HTTP status: {}",
                        url,
                        response.status()
                    );
                    return Ok(CallToolResult::success(vec![Content::text(error_text)]));
                }

                match response.text().await {
                    Ok(html_content) => {
                        // Convert HTML to text
                        let html_result = html2text::from_read(html_content.as_bytes(), 130);
                        if let Err(e) = &html_result {
                            return Ok(CallToolResult::success(vec![Content::text(format!(
                                "Error converting HTML to text: {}",
                                e
                            ))]));
                        }
                        let text_content = html_result.unwrap();

                        // Truncate if too long
                        const MAX_LENGTH: usize = 8000;
                        let truncated_text = if text_content.chars().count() > MAX_LENGTH {
                            format!(
                                "{}\n\n[Content truncated. Full documentation available at {}]",
                                text_content.chars().take(MAX_LENGTH).collect::<String>(),
                                url
                            )
                        } else {
                            text_content
                        };
                        Ok(CallToolResult::success(vec![Content::text(truncated_text)]))
                    }
                    Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                        "Error reading documentation content: {}",
                        e
                    ))])),
                }
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error fetching documentation from {}: {}",
                url, e
            ))])),
        }
    }

    /// List function signatures found in the current project directory.
    #[tool(description = "List function signatures found in the current project directory.")]
    async fn list_function_signatures(
        &self,
        #[tool(aggr)] _args: ListFunctionSignaturesArgs,
    ) -> Result<CallToolResult, McpError> {
        // [REMOVED: function signature logic due to missing dependencies]
        Ok(CallToolResult::success(vec![Content::text(
            "Function signature extraction temporarily disabled: missing internal implementation."
                .to_string(),
        )]))
    }

    /// Search code in the current project using the probe library
    ///
    /// Performs a code search with powerful filtering and ranking capabilities.
    /// Use this tool to find patterns, functions, and code blocks in your codebase.
    #[tool(
        description = "Search code in the current project using the probe library\n\nPerforms a code search with powerful filtering and ranking capabilities.\nUse this tool to find patterns, functions, and code blocks in your codebase."
    )]
    async fn search_code_tool(
        &self,
        #[tool(aggr)] args: ProbeSearchArgs,
    ) -> Result<CallToolResult, McpError> {
        use self::mcp::probe_search::{format_search_results, search_code};

        // Get the current working directory
        let current_dir = self.0.lock().unwrap().current_working_dir.clone();

        // Set defaults for optional parameters
        let file_patterns = args.file_patterns.unwrap_or_default();
        let max_results = args.max_results.unwrap_or(20);

        // Perform the search using our probe_search module
        match search_code(
            &args.query,
            &current_dir,
            file_patterns,
            max_results,
            args.language,
        )
        .await
        {
            Ok(results) => {
                // Format the results into a readable string
                let formatted_results = format_search_results(&results);

                // If there are no results, provide a helpful message
                if results.results.is_empty() {
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "No results found for query: '{}'. Try broadening your search or using different terms.",
                        args.query
                    ))]))
                } else {
                    Ok(CallToolResult::success(vec![Content::text(
                        formatted_results,
                    )]))
                }
            }
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Error searching code: {}",
                e
            ))])),
        }
    }
}

#[tool(tool_box)]
impl ServerHandler for CorrodeMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This server provides tools for working with Rust code and projects. It can execute commands, read and write files, search code, and interact with crates.io.".to_string()),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParamInner>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        Err(McpError::resource_not_found(
            "resource_not_found",
            Some(json!({
                "uri": uri
            })),
        ))
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParamInner>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            next_cursor: None,
            prompts: vec![],
        })
    }

    async fn get_prompt(
        &self,
        GetPromptRequestParam { name, .. }: GetPromptRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        Err(McpError::invalid_params(
            "prompt not found",
            Some(json!({
                "name": name
            })),
        ))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParamInner>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
        })
    }
}

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
