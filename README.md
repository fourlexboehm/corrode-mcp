# Corrode MCP Server

Corrode Code Model Context Protocol (MCP) Rust Server.

Model Context Protocol (MCP) is an open protocol that enables seamless integration between LLM applications
and external data sources and tools. Whether you're building an AI-powered IDE, enhancing a chat interface,
or creating custom AI workflows, MCP provides a standardized way to connect LLMs with the context they need.

This project provides an MCP server implementation in Rust for code-related tasks.

## Key Features

The Corrode MCP Server offers powerful capabilities for Rust developers:

### Rust-Specific Tools

- **Crates.io Integration**: Seamlessly search, explore, and manage Rust crates directly from your AI interface.
  - Search through available crates with detailed metadata
  - Retrieve specific crate information with comprehensive details
  - View all available versions of a crate to ensure compatibility
  - Examine crate dependencies to better understand project requirements

- **Code Analysis**: Analyze Rust code with intelligent tooling.
  - Check Rust code for compilation errors with integrated `cargo check`
  - Identify function signatures throughout your project
  - Examine Rust code structure and dependencies

### General Development Tools

- **File Operations**: Efficiently work with your codebase.
  - Read and write files with proper encoding
  - Apply changes through unified diffs
  - Navigate the file system with intuitive commands

- **Shell Command Execution**: Execute shell commands with full context handling.
  - Run `cargo` commands with proper environment setup
  - Manage directory navigation with automatic context tracking
  - Execute complex shell operations directly from your AI interface

# Installation

## From Crates.io (Recommended)

1. Ensure you have Rust and Cargo installed.
2. Install the server using Cargo:
   ```bash
   cargo install corrode-mcp
   ```
   This will download the crate from crates.io, build it, and install the `corrode-mcp` binary to your Cargo bin directory (usually `~/.cargo/bin/`). Ensure this directory is in your system's PATH.

## From Source

1. Clone the repository:
   ```bash
   git clone <repository_url> # TODO: Add repository URL
   cd corrode-mcp
   ```
2. Build and install using Cargo:
   ```bash
   cargo install --path .
   ```

# How to use MCP CLI server in Claude Desktop?

1. Ensure `corrode-mcp` is installed (see Installation section) and available in your system's PATH.
2. Edit `claude_desktop_config.json`: Claude Desktop -> `Settings` -> `Developer` -> `Edit Config`
3. Add the following configuration under the `mcpServers` key (or merge it if `mcpServers` already exists):

```json
{
  "mcpServers": {
    "corrode-mcp": {
      "command": "corrode-mcp",
      "args": ["--mcp"]
    }
  }
}
```

If you want to check MCP log, please use `tail -n 20 -f ~/Library/Logs/Claude/mcp*.log`.

# Usage Examples

Here are some practical ways to leverage Corrode MCP with your Rust projects:

## Exploring Crates
```
# Search for async runtime crates
> Search for crates related to async runtime

# Get detailed information about a specific version
> What are the features available in tokio 1.44.1?

# Check dependencies
> Show me the dependencies for serde_json 1.0
```

## Code Analysis
```
# Check for errors in your project
> Are there any compilation errors in my current project?

# Code structure understanding
> List all function signatures in the src directory

# Cargo operations
> Update my dependencies to the latest versions
```

## Rust Project Management
```
# Create new components
> Create a new module for handling HTTP requests

# Explore project structure
> Draw an architecture diagram of this Rust project

# Quality improvements
> Analyze this code for potential performance improvements
```

# References

* MCP Specification: https://spec.modelcontextprotocol.io/
* Model Context Protocol (MCP): https://modelcontextprotocol.io/introduction
* rpc-router: json-rpc routing library - https://github.com/jeremychone/rust-rpc-router/
* Zed context_server: https://github.com/zed-industries/zed/tree/main/crates/context_server
