use anyhow::Result;
use corrode_mcp::{CorrodeMcpServer, ServerData};
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::{self, EnvFilter};
use std::sync::Mutex;

/// npx @modelcontextprotocol/inspector cargo run
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the tracing subscriber with file and stdout logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
    tracing::info!("Starting Corrode MCP server");
    
    // Create server data with current working directory
    let current_dir = std::env::current_dir()?;
    tracing::info!("Working directory: {}", current_dir.display());
    
    // Create the server instance
    let server = CorrodeMcpServer(Mutex::new(ServerData {
        current_working_dir: current_dir,
        http_client: reqwest::Client::new(),
    }));
    
    // Create the service with stdio transport
    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;
    
    // Wait for the service to complete
    service.waiting().await?;
    Ok(())
}
