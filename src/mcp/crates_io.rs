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
    },
    Text {
        data: String,
        status: u16,
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
        })
    } else {
        Ok(FetchResponse::Text {
            data: response.text().await?,
            status,
        })
    }
}

#[derive(Clone)]
pub struct CratesIoClient {
    client: Client,
}

impl CratesIoClient {

    pub fn with_client(client: Client) -> Self {
        CratesIoClient { client }
    }

    pub async fn get(
        &self,
        path: &str, 
        options: Option<RequestOptions>
    ) -> Result<FetchResponse, reqwest::Error> {
        let mut opts = options.unwrap_or_default();
        opts.method = Some("GET".to_string());
        crates_io_fetch(&self.client, path, opts).await
    }

  
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