// No imports needed for constants

/// The workflow prompt to guide AI assistants in making code changes
pub const CODE_CHANGE_WORKFLOW: &str = r#"
# MCP Code Change Workflow

## Basic Change Loop
1. READ: Understand the current code with `read_file`
2. MODIFY: Make the requested changes with `edit_file` or `write_file`
3. VERIFY: Run `check_code` to ensure changes compile
4. ITERATE: If verification fails, fix issues and return to step 3
5. COMPLETE: When verification passes, report success

## Dependency Addition Sub-Loop
When adding a new dependency, insert this between steps 1 and 2:
   a. RESEARCH: Use `tool_search_crates` to find the package
   b. VERSION: Use `get_crate_versions` to identify the latest stable version
   c. Then continue with the main loop

IMPORTANT: 
- Always complete the verification step for every code change
- For dependency changes, always check compatibility with existing dependencies
- Document any non-obvious decisions in your response
"#;

/// Comprehensive guide for using MCP tools with detailed instructions
pub const MCP_TOOLS_GUIDE: &str = r#"
# Corrode MCP Tool Usage Guide

You are an AI assistant with access to Rust-specific MCP tools. 
When making changes to Rust code, follow the workflow below.

## Basic Change Loop
1. READ: Understand the current code with `read_file`
2. MODIFY: Make the requested changes with `edit_file` or `write_file`
3. VERIFY: Run `check_code` to ensure changes compile
4. ITERATE: If verification fails, fix issues and return to step 3
5. COMPLETE: When verification passes, report success

## Dependency Addition Sub-Loop
When adding a new dependency, insert this between steps 1 and 2:
   a. RESEARCH: Use `tool_search_crates` to find the package
   b. VERSION: Use `get_crate_versions` to identify the latest stable version
   c. Then continue with the main loop

## Available MCP Tools:

1. `read_file`: Read content from a file
   - Usage: `read_file({ "file_path": "path/to/file" })`
   - Best practice: Check if file exists before reading

2. `write_file`: Write or overwrite content to a file
   - Usage: `write_file({ "file_path": "path/to/file", "content": "file content" })`
   - Always verify writes with check_code if modifying Rust code

3. `edit_file`: Modify a file using unified diffs
   - Usage: `edit_file({ "file_path": "path/to/file", "diff": "@@ ... @@" })`
   - Prefer for small, targeted changes to maintain context

4. `check_code`: Verify that Rust code compiles correctly
   - Usage: `check_code({})`
   - Always run after any code modifications

5. `execute_bash`: Run shell commands with proper context
   - Usage: `execute_bash({ "command": "ls -la" })`
   - Use to run cargo commands, navigate directories, etc.

6. `tool_search_crates`: Search for packages on crates.io
   - Usage: `tool_search_crates({ "query": "tokio", "page": 1, "per_page": 10 })`
   - Use to find potential dependencies

7. `get_crate`: Get detailed information about a specific crate
   - Usage: `get_crate({ "crate_name": "tokio" })`
   - Check for popularity, description, and repository links

8. `tool_get_crate_versions`: Get all versions of a crate
   - Usage: `tool_get_crate_versions({ "crate_name": "tokio" })`
   - Always use when adding new dependencies

9. `get_crate_dependencies`: Get dependencies for a specific crate version
   - Usage: `get_crate_dependencies({ "crate_name": "tokio", "version": "1.25.0" })`
   - Check for compatibility issues with existing dependencies

10. `list_function_signatures`: List function signatures in the project
    - Usage: `list_function_signatures({ "file_path": null })`
    - Use to understand the project structure
"#;
