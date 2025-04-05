use std::collections::HashMap;
use corrode_mcp::mcp::crates_io::{CratesIoClient, RequestOptions, FetchResponse};

#[tokio::main]
async fn main() {
    println!("Testing crates.io API client...");
    
    // Search for crates
    let mut params = HashMap::new();
    params.insert("q".to_string(), "serde".to_string());
    params.insert("per_page".to_string(), "5".to_string());
    
    let options = RequestOptions {
        params: Some(params),
        ..Default::default()
    };
    
    println!("Searching for 'serde'...");
    match CratesIoClient::get("crates", Some(options)).await {
        Ok(response) => match response {
            FetchResponse::Json { data, status, .. } => {
                println!("Status: {}", status);
                println!("Data: {}", serde_json::to_string_pretty(&data).unwrap());
            }
            FetchResponse::Text { data, status, .. } => {
                println!("Status: {}", status);
                println!("Data: {}", data);
            }
        },
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    
    // Get details for a specific crate
    println!("\nGetting details for 'serde'...");
    match CratesIoClient::get("crates/serde", None).await {
        Ok(response) => match response {
            FetchResponse::Json { data, status, .. } => {
                println!("Status: {}", status);
                println!("Data: {}", serde_json::to_string_pretty(&data).unwrap());
            }
            FetchResponse::Text { data, status, .. } => {
                println!("Status: {}", status);
                println!("Data: {}", data);
            }
        },
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}