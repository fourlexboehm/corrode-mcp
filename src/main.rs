mod mcp;
use mcp_attr::{server::serve_stdio_with, Result};
use mcp_attr::server::serve_stdio;
use std::sync::Mutex;
use std::path::PathBuf;
use std::env;
use reqwest; // Keep reqwest as it's used in http_client builder
use corrode_mcp::{CorrodeMcpServer, ServerData};


#[tokio::main]
async fn main() -> Result<()> {

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
