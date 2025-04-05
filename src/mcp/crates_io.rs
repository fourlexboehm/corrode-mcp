use log::debug;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    pub method: Option<String>,
    pub params: Option<HashMap<String, String>>,
    pub body: Option<serde_json::Value>,
}

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

const BASE_URL: &str = "https://crates.io/api/v1/";

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

pub async fn crates_io_fetch(
    client: &Client,
    path: &str,
    options: RequestOptions,
) -> Result<FetchResponse, reqwest::Error> {
    let method = options.method.unwrap_or_else(|| "GET".to_string());
    let url = build_url(path, options.params);

    debug!("Making request to {}", url);
    debug!("Method: {}", method);

    let request_builder = match method.as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
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
    
    debug!("Received response from {} with status: {}", url, status);

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

pub struct CratesIoClient;

impl CratesIoClient {
    pub async fn get(
        path: &str,
        options: Option<RequestOptions>,
    ) -> Result<FetchResponse, reqwest::Error> {
        let client = get_default_client();
        let mut opts = options.unwrap_or_default();
        opts.method = Some("GET".to_string());
        crates_io_fetch(&client, path, opts).await
    }

    // pub async fn post(
    //     path: &str,
    //     options: Option<RequestOptions>,
    // ) -> Result<FetchResponse, reqwest::Error> {
    //     let client = get_default_client();
    //     let mut opts = options.unwrap_or_default();
    //     opts.method = Some("POST".to_string());
    //     crates_io_fetch(&client, path, opts).await
    // }

    // pub async fn put(
    //     path: &str,
    //     options: Option<RequestOptions>,
    // ) -> Result<FetchResponse, reqwest::Error> {
    //     let client = get_default_client();
    //     let mut opts = options.unwrap_or_default();
    //     opts.method = Some("PUT".to_string());
    //     crates_io_fetch(&client, path, opts).await
    // }

    // pub async fn delete(
    //     path: &str,
    //     options: Option<RequestOptions>,
    // ) -> Result<FetchResponse, reqwest::Error> {
    //     let client = get_default_client();
    //     let mut opts = options.unwrap_or_default();
    //     opts.method = Some("DELETE".to_string());
    //     crates_io_fetch(&client, path, opts).await
    // }

    // pub async fn get_with_client(
    //     client: &Client,
    //     path: &str,
    //     options: Option<RequestOptions>,
    // ) -> Result<FetchResponse, reqwest::Error> {
    //     let mut opts = options.unwrap_or_default();
    //     opts.method = Some("GET".to_string());
    //     crates_io_fetch(client, path, opts).await
    // }
}

// CratesIoClient is the primary interface for accessing the crates.io API

fn get_default_client() -> Client {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("application/json"),
    );
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static("rust-docs-mcp-server/1.0.0"),
    );

    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .default_headers(headers)
        .build()
        .expect("Failed to build HTTP client")
}

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