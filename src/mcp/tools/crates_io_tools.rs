use crate::mcp::crates_io::{CratesIoClient, RequestOptions, FetchResponse};
use crate::mcp::types::*;
use rpc_router::{HandlerResult, RpcParams};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use reqwest;
use html2text;

// Search crates parameters
#[derive(Debug, Clone, Serialize, Deserialize, RpcParams)]
pub struct SearchCratesRequest {
    pub query: String,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

// Get crate details parameters
#[derive(Debug, Clone, Serialize, Deserialize, RpcParams)]
pub struct GetCrateRequest {
    pub crate_name: String,
}

// Get crate versions parameters
#[derive(Debug, Clone, Serialize, Deserialize, RpcParams)]
pub struct GetCrateVersionsRequest {
    pub crate_name: String,
}

// Get crate dependencies parameters
#[derive(Debug, Clone, Serialize, Deserialize, RpcParams)]
pub struct GetCrateDependenciesRequest {
    pub crate_name: String,
    pub version: String,
}

// Search for crates.io packages
pub async fn search_crates(request: SearchCratesRequest) -> HandlerResult<CallToolResult> {
    let mut query_params = HashMap::new();
    query_params.insert("q".to_string(), request.query.clone());
    
    if let Some(page) = request.page {
        query_params.insert("page".to_string(), page.to_string());
    }
    
    if let Some(per_page) = request.per_page {
        query_params.insert("per_page".to_string(), per_page.to_string());
    }
    
    let options = RequestOptions {
        params: Some(query_params),
        ..RequestOptions::default()
    };
    
    match CratesIoClient::get("crates", Some(options)).await {
        Ok(response) => {
            match response {
                FetchResponse::Json { data, status, .. } => {
                    let json_string = serde_json::to_string_pretty(&data)
                        .unwrap_or_else(|_| "Error converting JSON data to string".to_string());
                    
                    Ok(CallToolResult {
                        content: vec![
                            CallToolResultContent::Text {
                                text: format!("Status: {}\n\n{}", status, json_string)
                            },
                        ],
                        is_error: false,
                    })
                },
                FetchResponse::Text { data, status, .. } => {
                    Ok(CallToolResult {
                        content: vec![
                            CallToolResultContent::Text { text: format!("Status: {}\n{}", status, data) },
                        ],
                        is_error: false,
                    })
                }
            }
        },
        Err(e) => {
            Ok(CallToolResult {
                content: vec![
                    CallToolResultContent::Text { text: format!("Error searching crates: {}", e) },
                ],
                is_error: true,
            })
        },
    }
}

// Get crate details
pub async fn get_crate(request: GetCrateRequest) -> HandlerResult<CallToolResult> {
    let path = format!("crates/{}", request.crate_name);
    
    match CratesIoClient::get(&path, None).await {
        Ok(response) => {
            match response {
                FetchResponse::Json { data, status, .. } => {
                    let json_string = serde_json::to_string_pretty(&data)
                        .unwrap_or_else(|_| "Error converting JSON data to string".to_string());
                    
                    Ok(CallToolResult {
                        content: vec![
                            CallToolResultContent::Text {
                                text: format!("Status: {}\n\n{}", status, json_string)
                            },
                        ],
                        is_error: false,
                    })
                },
                FetchResponse::Text { data, status, .. } => {
                    Ok(CallToolResult {
                        content: vec![
                            CallToolResultContent::Text { text: format!("Status: {}\n{}", status, data) },
                        ],
                        is_error: false,
                    })
                }
            }
        },
        Err(e) => {
            Ok(CallToolResult {
                content: vec![
                    CallToolResultContent::Text { text: format!("Error getting crate details: {}", e) },
                ],
                is_error: true,
            })
        },
    }
}

// Get crate versions
pub async fn get_crate_versions(request: GetCrateVersionsRequest) -> HandlerResult<CallToolResult> {
    let path = format!("crates/{}/versions", request.crate_name);
    
    match CratesIoClient::get(&path, None).await {
        Ok(response) => {
            match response {
                FetchResponse::Json { data, status, .. } => {
                    let json_string = serde_json::to_string_pretty(&data)
                        .unwrap_or_else(|_| "Error converting JSON data to string".to_string());
                    
                    Ok(CallToolResult {
                        content: vec![
                            CallToolResultContent::Text {
                                text: format!("Status: {}\n\n{}", status, json_string)
                            },
                        ],
                        is_error: false,
                    })
                },
                FetchResponse::Text { data, status, .. } => {
                    Ok(CallToolResult {
                        content: vec![
                            CallToolResultContent::Text { text: format!("Status: {}\n{}", status, data) },
                        ],
                        is_error: false,
                    })
                }
            }
        },
        Err(e) => {
            Ok(CallToolResult {
                content: vec![
                    CallToolResultContent::Text { text: format!("Error getting crate versions: {}", e) },
                ],
                is_error: true,
            })
        },
    }
}

// Lookup crate docs parameters
#[derive(Debug, Clone, Serialize, Deserialize, RpcParams)]
pub struct LookupCrateDocsRequest {
    #[serde(rename = "crateName")]
    pub crate_name: Option<String>,
}

// Lookup documentation for a Rust crate from docs.rs
pub async fn lookup_crate_docs(request: LookupCrateDocsRequest) -> HandlerResult<CallToolResult> {
    let crate_name = request.crate_name.unwrap_or_else(|| "tokio".to_string());
    let url = format!("https://docs.rs/{}/latest/{}/index.html", crate_name, crate_name);

    log::debug!("Fetching documentation for crate: {}", crate_name);
    log::debug!("Making request to: {}", url);

    // Create a client with a timeout
    let client_result = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20)) // 20 second timeout
        .build();

    let client = match client_result {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to build reqwest client: {}", e);
            let error_text = format!("Error: Failed to initialize HTTP client. {}", e);
            return Ok(CallToolResult {
                content: vec![CallToolResultContent::Text { text: error_text }],
                is_error: true,
            });
        }
    };

    match client.get(&url).send().await { // Use the configured client
        Ok(response) => {
            log::debug!("Received response with status: {}", response.status());
            if !response.status().is_success() {
                let error_text = format!("Error: Could not fetch documentation. HTTP status: {}", response.status());
                 return Ok(CallToolResult {
                    content: vec![CallToolResultContent::Text { text: error_text }],
                    is_error: true,
                });
            }

            match response.text().await {
                Ok(html_content) => {
                    log::debug!("Successfully fetched HTML content.");
                    // Convert HTML to text, ignoring links and images
                    let text_content = html2text::from_read(html_content.as_bytes(), 130);

                    // Truncate if necessary
                    const MAX_LENGTH: usize = 8000;
                    let truncated_text = if text_content.len() > MAX_LENGTH {
                        format!("{}\n\n[Content truncated. Full documentation available at {}]", &text_content[..MAX_LENGTH], url)
                    } else {
                        text_content
                    };

                    log::debug!("Successfully processed docs for {}", crate_name);
                    Ok(CallToolResult {
                        content: vec![CallToolResultContent::Text { text: truncated_text }],
                        is_error: false,
                    })
                }
                Err(e) => {
                    log::error!("Error reading response body: {}", e);
                    let error_text = format!("Error: Could not read documentation content. {}", e);
                    Ok(CallToolResult {
                        content: vec![CallToolResultContent::Text { text: error_text }],
                        is_error: true,
                    })
                }
            }
        }
        Err(e) => {
            log::error!("Error fetching documentation: {}", e);
            let error_text = format!("Error: Could not fetch documentation. {}", e);
            Ok(CallToolResult {
                content: vec![CallToolResultContent::Text { text: error_text }],
                is_error: true,
            })
        }
    }
}

// Get crate dependencies
pub async fn get_crate_dependencies(request: GetCrateDependenciesRequest) -> HandlerResult<CallToolResult> {
    let path = format!("crates/{}/{}/dependencies", request.crate_name, request.version);
    
    match CratesIoClient::get(&path, None).await {
        Ok(response) => {
            match response {
                FetchResponse::Json { data, status, .. } => {
                    let json_string = serde_json::to_string_pretty(&data)
                        .unwrap_or_else(|_| "Error converting JSON data to string".to_string());
                    
                    Ok(CallToolResult {
                        content: vec![
                            CallToolResultContent::Text {
                                text: format!("Status: {}\n\n{}", status, json_string)
                            },
                        ],
                        is_error: false,
                    })
                },
                FetchResponse::Text { data, status, .. } => {
                    Ok(CallToolResult {
                        content: vec![
                            CallToolResultContent::Text { text: format!("Status: {}\n{}", status, data) },
                        ],
                        is_error: false,
                    })
                }
            }
        },
        Err(e) => {
            Ok(CallToolResult {
                content: vec![
                    CallToolResultContent::Text { text: format!("Error getting crate dependencies: {}", e) },
                ],
                is_error: true,
            })
        },
    }
}