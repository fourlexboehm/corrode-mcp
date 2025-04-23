// Import the library crate which brings in public definitions from lib.rs
extern crate corrode_mcp;

// Define our own mcp module for any main-specific logic
// but most functionality will come from the library crate
mod mcp;
use mcp_attr::server::serve_stdio;
use std::env;
use std::path::PathBuf;
use std::sync::Mutex;

// Import server structs from the library crate
// Using the library name as a prefix makes it clear where these come from
use corrode_mcp::{CorrodeMcpServer, ServerData};

#[tokio::main]
// Using Box<dyn std::error::Error> to address the large error type warning
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let server_data = ServerData {
        current_working_dir: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        http_client: reqwest::Client::builder()
            .user_agent("corrode-mcp/0.0.2 (github.com/alexboehm/corrode-mcp)")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new()),
    };
    let server = CorrodeMcpServer(Mutex::new(server_data));

    serve_stdio(server).await?;

    Ok(())
}
