# Corrode MCP Server
====================

Corrode Code Model Context Protocol (MCP) Rust Server.

Model Context Protocol (MCP) is an open protocol that enables seamless integration between LLM applications
and external data sources and tools. Whether you’re building an AI-powered IDE, enhancing a chat interface,
or creating custom AI workflows, MCP provides a standardized way to connect LLMs with the context they need.

This project provides an MCP server implementation in Rust for code-related tasks.

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

# CLI options

* `--mcp`: Enable MCP server
* `--resources`: display resources
* `--prompts`: display prompts
* `--tools`: display tools

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


# References

* MCP Specification: https://spec.modelcontextprotocol.io/
* Model Context Protocol (MCP): https://modelcontextprotocol.io/introduction
* rpc-router: json-rpc routing library - https://github.com/jeremychone/rust-rpc-router/
* Zed context_server: https://github.com/zed-industries/zed/tree/main/crates/context_server
