use crate::mcp::types::*;
use rpc_router::{HandlerResult, IntoHandlerError};
use serde_json::json;

pub async fn prompts_list(
    _request: Option<ListPromptsRequest>,
) -> HandlerResult<ListPromptsResult> {
    //let prompts: Vec<Prompt> = serde_json::from_str(include_str!("./templates/prompts.json")).unwrap();
    let response = ListPromptsResult {
        next_cursor: None,
        prompts: vec![

            Prompt {
                name: "analyze-code".to_string(),
                description: Some("Analyze code for potential improvements".to_string()),
                arguments: Some(vec![PromptArgument {
                    name: "language".to_string(),
                    description: Some("Programming language".to_string()),
                    required: Some(true),
                }]),
            },
            // Crates.io prompts...
            Prompt {
                name: "search_crates".to_string(),
                description: Some("Search for crates on crates.io".to_string()),
                arguments: Some(vec![
                    PromptArgument { name: "query".to_string(), description: Some("Search query string".to_string()), required: Some(true) },
                    PromptArgument { name: "page".to_string(), description: Some("Page number (optional)".to_string()), required: Some(false) },
                    PromptArgument { name: "per_page".to_string(), description: Some("Results per page (optional)".to_string()), required: Some(false) },
                ]),
            },
            Prompt {
                name: "get_crate".to_string(),
                description: Some("Get details for a specific crate".to_string()),
                arguments: Some(vec![PromptArgument { name: "crate_name".to_string(), description: Some("Name of the crate".to_string()), required: Some(true) }]),
            },
            Prompt {
                name: "get_crate_versions".to_string(),
                description: Some("Get versions for a specific crate".to_string()),
                arguments: Some(vec![PromptArgument { name: "crate_name".to_string(), description: Some("Name of the crate".to_string()), required: Some(true) }]),
            },
            Prompt {
                name: "get_crate_dependencies".to_string(),
                description: Some("Get dependencies for a specific crate version".to_string()),
                arguments: Some(vec![
                    PromptArgument { name: "crate_name".to_string(), description: Some("Name of the crate".to_string()), required: Some(true) },
                    PromptArgument { name: "version".to_string(), description: Some("Version of the crate".to_string()), required: Some(true) },
                ]),
            },
            Prompt {
                name: "lookup_crate_docs".to_string(),
                description: Some("Lookup documentation for a Rust crate from docs.rs".to_string()),
                arguments: Some(vec![PromptArgument { name: "crateName".to_string(), description: Some("Name of the Rust crate (defaults to 'tokio')".to_string()), required: Some(false) }]),
            },
            // Other tool prompts...
            Prompt {
                name: "execute_bash".to_string(),
                description: Some("Execute a bash command".to_string()),
                arguments: Some(vec![PromptArgument {
                    name: "command".to_string(),
                    description: Some("The bash command to execute".to_string()),
                    required: Some(true),
                }]),
            },
            Prompt {
                name: "read_file".to_string(),
                description: Some("Read the content of a file".to_string()),
                arguments: Some(vec![
                    PromptArgument {
                        name: "file_path".to_string(),
                        description: Some("Path to the file to read".to_string()),
                        required: Some(true),
                    },
                    PromptArgument {
                        name: "max_chars".to_string(),
                        description: Some("Maximum characters to read (optional, default 1000)".to_string()),
                        required: Some(false),
                    },
                ]),
            },
            Prompt {
                name: "edit_file".to_string(),
                description: Some("Edit a file using a diff".to_string()),
                arguments: Some(vec![
                    PromptArgument {
                        name: "file_path".to_string(),
                        description: Some("Path to the file to edit".to_string()),
                        required: Some(true),
                    },
                    PromptArgument {
                        name: "diff".to_string(),
                        description: Some("The diff content (unified format)".to_string()),
                        required: Some(true),
                    },
                ]),
            },
            Prompt {
                name: "write_file".to_string(),
                description: Some("Write content to a file".to_string()),
                arguments: Some(vec![
                    PromptArgument {
                        name: "file_path".to_string(),
                        description: Some("Path to the file to write".to_string()),
                        required: Some(true),
                    },
                    PromptArgument {
                        name: "content".to_string(),
                        description: Some("The content to write".to_string()),
                        required: Some(true),
                    },
                ]),
            },
            Prompt {
                name: "check".to_string(),
                description: Some("Check Rust code for errors using 'cargo check'".to_string()),
                arguments: None, // No arguments needed
            },
            Prompt {
                name: "parse_code".to_string(),
                description: Some("Parse code structure using TreeSitter".to_string()),
                arguments: Some(vec![
                    PromptArgument {
                        name: "file_path".to_string(),
                        description: Some("Path to the file to parse".to_string()),
                        required: Some(true),
                    },
                    PromptArgument {
                        name: "project_path".to_string(),
                        description: Some("Optional project root path".to_string()),
                        required: Some(false),
                    },
                ]),
            },
        ],
    };
    Ok(response)
}

pub async fn prompts_get(request: GetPromptRequest) -> HandlerResult<PromptResult> {
    let args = request.arguments.unwrap_or_default(); // Use unwrap_or_default for safety

    let response = match request.name.as_str() {

        "search_crates" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("unknown query");
            PromptResult {
                description: "Search crates.io".to_string(),
                messages: Some(vec![PromptMessage {
                    role: "user".to_string(),
                    content: PromptMessageContent {
                        type_name: "text".to_string(),
                        text: format!("Search crates.io for '{}'. Summarize the top results.", query),
                    },
                }]),
            }
        }
        "get_crate" => {
            let crate_name = args.get("crate_name").and_then(|v| v.as_str()).unwrap_or("unknown crate");
            PromptResult {
                description: "Get crate details".to_string(),
                messages: Some(vec![PromptMessage {
                    role: "user".to_string(),
                    content: PromptMessageContent {
                        type_name: "text".to_string(),
                        text: format!("Provide a summary of the crate '{}' based on its details.", crate_name),
                    },
                }]),
            }
        }
        "get_crate_versions" => {
            let crate_name = args.get("crate_name").and_then(|v| v.as_str()).unwrap_or("unknown crate");
            PromptResult {
                description: "Get crate versions".to_string(),
                messages: Some(vec![PromptMessage {
                    role: "user".to_string(),
                    content: PromptMessageContent {
                        type_name: "text".to_string(),
                        text: format!("List the recent versions of the crate '{}'.", crate_name),
                    },
                }]),
            }
        }
        "get_crate_dependencies" => {
            let crate_name = args.get("crate_name").and_then(|v| v.as_str()).unwrap_or("unknown crate");
            let version = args.get("version").and_then(|v| v.as_str()).unwrap_or("latest");
             PromptResult {
                description: "Get crate dependencies".to_string(),
                messages: Some(vec![PromptMessage {
                    role: "user".to_string(),
                    content: PromptMessageContent {
                        type_name: "text".to_string(),
                        text: format!("List the main dependencies for crate '{}' version {}.", crate_name, version),
                    },
                }]),
            }
        }
        "lookup_crate_docs" => {
            let crate_name = args.get("crateName").and_then(|v| v.as_str()).unwrap_or("tokio");
            PromptResult {
                description: "Lookup crate documentation".to_string(),
                messages: Some(vec![PromptMessage {
                    role: "user".to_string(),
                    content: PromptMessageContent {
                        type_name: "text".to_string(),
                        text: format!(
                            "Please analyze and summarize the documentation for the Rust crate '{}'. Focus on:\n1. The main purpose and features of the crate\n2. Key types and functions\n3. Common usage patterns\n4. Any important notes or warnings\n5. VERY IMPORTANT: Latest Version\n\nDocumentation content will follow.",
                            crate_name
                        ),
                    },
                }]),
            }
        }
        "execute_bash" => {
            let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("echo 'No command provided'");
            PromptResult {
                description: "Execute bash command".to_string(),
                messages: Some(vec![PromptMessage {
                    role: "user".to_string(),
                    content: PromptMessageContent {
                        type_name: "text".to_string(),
                        text: format!("Execute the following bash command and report the output:\n```bash\n{}\n```", command),
                    },
                }]),
            }
        }
        "read_file" => {
            let file_path = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("unknown_file.txt");
            PromptResult {
                description: "Read file content".to_string(),
                messages: Some(vec![PromptMessage {
                    role: "user".to_string(),
                    content: PromptMessageContent {
                        type_name: "text".to_string(),
                        text: format!("Read the content of the file '{}' and provide it.", file_path),
                    },
                }]),
            }
        }
        "edit_file" => {
            let file_path = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("unknown_file.txt");
            let diff = args.get("diff").and_then(|v| v.as_str()).unwrap_or("No diff provided");
            PromptResult {
                description: "Edit file with diff".to_string(),
                messages: Some(vec![PromptMessage {
                    role: "user".to_string(),
                    content: PromptMessageContent {
                        type_name: "text".to_string(),
                        text: format!("Apply the following diff to the file '{}':\n```diff\n{}\n```", file_path, diff),
                    },
                }]),
            }
        }
        "write_file" => {
            let file_path = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("output.txt");
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("No content provided");
            PromptResult {
                description: "Write content to file".to_string(),
                messages: Some(vec![PromptMessage {
                    role: "user".to_string(),
                    content: PromptMessageContent {
                        type_name: "text".to_string(),
                        text: format!("Write the following content to the file '{}':\n```\n{}\n```", file_path, content),
                    },
                }]),
            }
        }
        "check" => {
             PromptResult {
                description: "Check Rust code".to_string(),
                messages: Some(vec![PromptMessage {
                    role: "user".to_string(),
                    content: PromptMessageContent {
                        type_name: "text".to_string(),
                        text: "Run 'cargo check' on the current Rust project and report any errors or warnings.".to_string(),
                    },
                }]),
            }
        }
         "parse_code" => {
            let file_path = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("unknown_file.rs");
            PromptResult {
                description: "Parse code structure".to_string(),
                messages: Some(vec![PromptMessage {
                    role: "user".to_string(),
                    content: PromptMessageContent {
                        type_name: "text".to_string(),
                        text: format!("Parse the code structure (functions, classes, etc.) of the file '{}' using TreeSitter.", file_path),
                    },
                }]),
            }
        }
        _ => {
            return Err(json!({"code": -32602, "message": "Prompt not found"}).into_handler_error())
        }
    };
    Ok(response)
}
