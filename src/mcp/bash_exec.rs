use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Represents the type of error that occurred during command execution.
#[derive(Debug, PartialEq)]
pub enum CommandErrorType {
    /// The command executable was not found.
    CommandNotFound,
    /// The user lacks permission to execute the command.
    PermissionDenied,
    /// The specified path does not exist or is not accessible.
    PathNotFound,
    /// The command execution failed with a non-zero exit code.
    ExecutionFailure,
    /// The shell itself encountered a problem.
    ShellError,
    /// Other general errors.
    Other,
}

/// Captures the complete context of a command execution for debugging.
pub struct CommandExecutionContext {
    /// The original command string.
    pub command: String,
    /// The working directory where the command is executed.
    pub working_directory: PathBuf,
    /// Whether this is a compound command (contains && or ;).
    pub is_compound_command: bool,
    /// The individual parts of the command.
    pub command_parts: Vec<String>,
    /// Environment variables at execution time.
    pub environment_variables: HashMap<String, String>,
    /// Timestamp when execution started.
    pub timestamp: SystemTime,
}

impl CommandExecutionContext {
    /// Creates a new CommandExecutionContext for a command.
    pub fn new(command: &str, working_directory: &Path) -> Self {
        // Parse command to check if it's compound
        let is_compound = command.contains("&&") || command.contains(';');
        
        // Split into parts for analysis
        let command_parts = if command.trim().contains(' ') {
            command
                .trim()
                .split_whitespace()
                .map(|s| s.to_string())
                .collect()
        } else {
            vec![command.trim().to_string()]
        };
        
        // Capture current environment variables
        let env_vars = std::env::vars().collect();
        
        Self {
            command: command.to_string(),
            working_directory: working_directory.to_path_buf(),
            is_compound_command: is_compound,
            command_parts,
            environment_variables: env_vars,
            timestamp: SystemTime::now(),
        }
    }
    
    /// Returns the command's executable name (first part of the command).
    pub fn executable(&self) -> &str {
        if let Some(first_part) = self.command_parts.first() {
            first_part
        } else {
            ""
        }
    }
}

/// Analyzes a command error and classifies it.
pub fn classify_error(output: &std::io::Error, _context: &CommandExecutionContext) -> CommandErrorType {
    let error_kind = output.kind();
    let error_string = output.to_string().to_lowercase();
    
    // Check for common error patterns
    if error_kind == std::io::ErrorKind::NotFound || error_string.contains("not found") 
        || error_string.contains("command not found") {
        CommandErrorType::CommandNotFound
    } else if error_kind == std::io::ErrorKind::PermissionDenied 
        || error_string.contains("permission denied") {
        CommandErrorType::PermissionDenied
    } else if error_string.contains("no such file") || error_string.contains("no such directory") 
        || error_string.contains("does not exist") {
        CommandErrorType::PathNotFound
    } else if error_string.contains("shell") {
        CommandErrorType::ShellError
    } else {
        CommandErrorType::Other
    }
}

/// Generates a suggestion for recovering from common command errors.
pub fn get_recovery_suggestion(error_type: &CommandErrorType, context: &CommandExecutionContext) -> String {
    match error_type {
        CommandErrorType::CommandNotFound => {
            // Generate suggestions based on the command that was not found
            let executable = context.executable();
            if executable.is_empty() {
                return "Try specifying a valid command.".to_string();
            }
            
            // Common package managers by platform
            if cfg!(target_os = "macos") {
                format!("Try installing '{}' with a package manager like Homebrew: 'brew install {}'", 
                       executable, executable)
            } else if cfg!(target_os = "linux") {
                format!("Try installing '{}' with your system's package manager (apt, yum, dnf, etc.)", 
                       executable)
            } else {
                format!("Ensure '{}' is installed and available in your PATH.", executable)
            }
        },
        CommandErrorType::PermissionDenied => {
            // For permission errors, suggest chmod or sudo
            format!("Try adjusting file permissions with 'chmod +x {}' or running with elevated privileges using 'sudo'.", 
                   context.executable())
        },
        CommandErrorType::PathNotFound => {
            // For path errors, suggest checking paths
            "Verify that the specified file or directory path exists and is accessible.".to_string()
        },
        CommandErrorType::ExecutionFailure => {
            // For execution failures, suggest checking arguments
            "Check command arguments and verify that they are correct for the desired operation.".to_string()
        },
        CommandErrorType::ShellError => {
            // For shell errors, provide general shell troubleshooting
            "The shell encountered an error. Try simplifying the command or checking for syntax errors.".to_string()
        },
        CommandErrorType::Other => {
            // For other errors, provide a general suggestion
            "This appears to be an unusual error. Check system resources and command syntax.".to_string()
        }
    }
}

/// Formats a command execution error message with detailed context.
pub fn format_error_message(
    context: &CommandExecutionContext,
    error: &std::io::Error,
    error_type: &CommandErrorType,
) -> String {
    let suggestion = get_recovery_suggestion(error_type, context);
    
    format!(
        "Command Execution Failed:
- Command: '{}'
- Working Directory: {}
- Error Type: {:?}
- Error Details: {}
- Suggestion: {}",
        context.command,
        context.working_directory.display(),
        error_type,
        error,
        suggestion
    )
}

/// Formats an execution failure error message with exit code and output.
pub fn format_execution_failure(
    context: &CommandExecutionContext,
    exit_code: i32,
    stdout: &str,
    stderr: &str,
) -> String {
    format!(
        "Command Execution Failed:
- Command: '{}'
- Working Directory: {}
- Error Type: ExecutionFailure
- Exit Code: {}
- Standard Output: {}
- Standard Error: {}
- Suggestion: Check command arguments and verify that the inputs are valid.",
        context.command,
        context.working_directory.display(),
        exit_code,
        if stdout.is_empty() { "[Empty]" } else { stdout },
        if stderr.is_empty() { "[Empty]" } else { stderr },
    )
}
