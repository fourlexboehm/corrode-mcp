use log::debug;
use reqwest::{header, Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use url::Url;

// Options for requests
#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    pub method: Option<String>,
    pub params: Option<HashMap<String, String>>,
    pub body: Option<serde_json::Value>,
}

// Response types
#[derive(Debug, Clone)]
pub enum FetchResponse {
    Json {
        data: serde_json::Value,
        status: u16,
        headers: reqwest::header::HeaderMap,
    },
    Text {
        data: String,
        status: u16,
        headers: reqwest::header::HeaderMap,
    },
}

// Singleton client for crates.io API
lazy_static::lazy_static! {
    static ref CLIENT: Client = {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("rust-docs-mcp-server/1.0.0"),
        );

        ClientBuilder::new()
            .timeout(Duration::from_secs(10))
            .default_headers(headers)
            .build()
            .expect("Failed to build HTTP client")
    };
}

const BASE_URL: &str = "https://crates.io/api/v1/";

// Helper to build full URL with query params
pub fn build_url(path: &str, params: Option<HashMap<String, String>>) -> String {
    let url_result = Url::parse(BASE_URL).and_then(|base| base.join(path));
    
    match url_result {
        Ok(mut url) => {
            if let Some(params) = params {
                for (key, value) in params {
                    url.query_pairs_mut().append_pair(&key, &value);
                }
            }
            url.to_string()
        }
        Err(e) => {
            eprintln!("Error building URL: {}", e);
            String::new()
        }
    }
}

// Create a configured fetch client for crates.io
pub async fn crates_io_fetch(
    path: &str,
    options: RequestOptions,
) -> Result<FetchResponse, reqwest::Error> {
    let method = options.method.unwrap_or_else(|| "GET".to_string());
    let url = build_url(path, options.params);

    debug!("Making request to {}", url);
    debug!("Method: {}", method);

    let request_builder = match method.as_str() {
        "GET" => CLIENT.get(&url),
        "POST" => CLIENT.post(&url),
        "PUT" => CLIENT.put(&url),
        "DELETE" => CLIENT.delete(&url),
        _ => panic!("Unsupported HTTP method: {}", method),
    };

    let request_builder = if let Some(body) = options.body {
        request_builder.json(&body)
    } else {
        request_builder
    };

    let response = request_builder.send().await?;

    let status = response.status().as_u16();
    let headers = response.headers().clone();
    
    debug!(
        "Received response from {} with status: {}",
        url, status
    );

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if status < 200 || status >= 300 {
        eprintln!("HTTP error! status: {}", status);
    }

    if content_type.contains("application/json") {
        Ok(FetchResponse::Json {
            data: response.json().await?,
            status,
            headers,
        })
    } else {
        Ok(FetchResponse::Text {
            data: response.text().await?,
            status,
            headers,
        })
    }
}

// Default client with convenience methods
pub struct CratesIoClient;

impl CratesIoClient {
    pub async fn get(
        path: &str,
        options: Option<RequestOptions>,
    ) -> Result<FetchResponse, reqwest::Error> {
        let mut opts = options.unwrap_or_default();
        opts.method = Some("GET".to_string());
        crates_io_fetch(path, opts).await
    }

    pub async fn post(
        path: &str,
        options: Option<RequestOptions>,
    ) -> Result<FetchResponse, reqwest::Error> {
        let mut opts = options.unwrap_or_default();
        opts.method = Some("POST".to_string());
        crates_io_fetch(path, opts).await
    }

    pub async fn put(
        path: &str,
        options: Option<RequestOptions>,
    ) -> Result<FetchResponse, reqwest::Error> {
        let mut opts = options.unwrap_or_default();
        opts.method = Some("PUT".to_string());
        crates_io_fetch(path, opts).await
    }

    pub async fn delete(
        path: &str,
        options: Option<RequestOptions>,
    ) -> Result<FetchResponse, reqwest::Error> {
        let mut opts = options.unwrap_or_default();
        opts.method = Some("DELETE".to_string());
        crates_io_fetch(path, opts).await
    }
}

// Re-export for convenience
pub use CratesIoClient as default;

// Example model for a crate
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CrateResponse {
    pub crate_data: CrateData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CrateData {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub downloads: u64,
    pub version_downloads: u64,
    pub versions: Option<Vec<u64>>,
    pub max_version: String,
    pub documentation: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
}

// Example model for crate versions
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionsResponse {
    pub versions: Vec<Version>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Version {
    pub id: String,
    pub num: String,
    pub created_at: String,
    pub updated_at: String,
    pub downloads: u64,
    pub yanked: bool,
    pub license: Option<String>,
}

// Example model for crate search
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResponse {
    pub crates: Vec<CrateSummary>,
    pub meta: SearchMeta,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CrateSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub downloads: u64,
    pub max_version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchMeta {
    pub total: u64,
}