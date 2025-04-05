use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tree_sitter::{Language, Node, Parser, Query, QueryCursor};
use walkdir::WalkDir;

use super::treesitter::{detect_language, get_line};

#[derive(Serialize, Deserialize)]
pub struct FunctionSignature {
    pub file_path: String,
    pub name: String,
    pub signature: String,
    pub line_number: usize,
    pub parent: Option<String>,
    pub language: String,
}

// Extract the function signature from source code
fn extract_signature(source: &str, node: &Node) -> String {
    // Get the line containing the function
    let start_position = node.start_position();
    let lines: Vec<&str> = source.lines().collect();
    
    if start_position.row < lines.len() {
        let func_line = lines[start_position.row];
        return func_line.trim().to_string();
    }
    
    // Return empty string if we can't get the line
    String::new()
}

// Get a query string for extracting function information for the given language
fn get_function_query_for_language(language: Language) -> Option<String> {
    // Rust functions and methods
    if language == unsafe { super::treesitter::tree_sitter_rust() } {
        Some(r#"
        (function_item name: (identifier) @function)
        (impl_item
          trait? @trait
          name: (type_identifier) @impl
          body: (declaration_list
            (associated_function
              name: (identifier) @method)))
        "#.to_string())
    } 
    // JavaScript/TypeScript functions and methods
    else if language == unsafe { super::treesitter::tree_sitter_javascript() } || 
            language == unsafe { super::treesitter::tree_sitter_typescript() } {
        Some(r#"
        (function_declaration name: (identifier) @function)
        (method_definition name: (property_identifier) @method)
        (arrow_function) @arrow_function
        "#.to_string())
    } 
    // Python functions and methods
    else if language == unsafe { super::treesitter::tree_sitter_python() } {
        Some(r#"
        (function_definition name: (identifier) @function)
        (class_definition 
          body: (block 
            (function_definition name: (identifier) @method)))
        "#.to_string())
    } 
    // Go functions and methods
    else if language == unsafe { super::treesitter::tree_sitter_go() } {
        Some(r#"
        (function_declaration name: (identifier) @function)
        (method_declaration name: (field_identifier) @method)
        "#.to_string())
    } 
    // C/C++ functions and methods
    else if language == unsafe { super::treesitter::tree_sitter_c() } || 
            language == unsafe { super::treesitter::tree_sitter_cpp() } {
        Some(r#"
        (function_definition declarator: (function_declarator declarator: (identifier) @function))
        (method_definition name: (field_identifier) @method)
        "#.to_string())
    } 
    else {
        None
    }
}

// Extract function signatures from a file
pub fn extract_function_signatures(file_path: &Path, language_override: Option<&str>) -> Vec<FunctionSignature> {
    let mut signatures = Vec::new();
    
    // Read the file content
    if let Ok(source) = fs::read_to_string(file_path) {
        if let Some((lang, lang_name)) = detect_language(file_path, language_override) {
            let mut parser = Parser::new();
            if parser.set_language(lang).is_err() {
                return signatures;
            }
            
            if let Some(tree) = parser.parse(&source, None) {
                let root_node = tree.root_node();
                
                // Create query to find functions
                if let Some(query_string) = get_function_query_for_language(lang) {
                    if let Ok(query) = Query::new(lang, &query_string) {
                        let mut cursor = QueryCursor::new();
                        let matches = cursor.matches(&query, root_node, source.as_bytes());
                        
                        for m in matches {
                            for capture in m.captures {
                                let capture_name = query.capture_names()[capture.index as usize].to_string();
                                
                                if capture_name == "function" || capture_name == "method" {
                                    let name = capture.node.utf8_text(source.as_bytes()).unwrap_or("unknown").to_string();
                                    let line_number = get_line(&capture.node, &source);
                                    let signature = extract_signature(&source, &capture.node);
                                    
                                    // Get parent for methods
                                    let parent = if capture_name == "method" {
                                        // Try to find parent class/struct name
                                        let parent_node = capture.node.parent()
                                            .and_then(|n| n.parent())
                                            .and_then(|n| n.parent());
                                            
                                        parent_node.and_then(|n| {
                                            for i in 0..n.child_count() {
                                                if let Some(child) = n.child(i) {
                                                    if child.kind() == "type_identifier" || 
                                                       child.kind() == "identifier" {
                                                        return child.utf8_text(source.as_bytes()).ok().map(String::from);
                                                    }
                                                }
                                            }
                                            None
                                        })
                                    } else {
                                        None
                                    };
                                    
                                    signatures.push(FunctionSignature {
                                        file_path: file_path.to_string_lossy().to_string(),
                                        name,
                                        signature,
                                        line_number,
                                        parent,
                                        language: lang_name.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    signatures
}

// Extract function signatures from all files in a project
pub fn extract_project_signatures(project_dir: &Path) -> Vec<FunctionSignature> {
    let mut all_signatures = Vec::new();
    
    // Walk through all files in the project
    let walker = WalkDir::new(project_dir).follow_links(true);
    let valid_extensions = vec!["rs", "js", "ts", "py", "go", "c", "h", "cpp", "hpp"];
    
    for entry_result in walker {
        if let Ok(entry) = entry_result {
            if !entry.file_type().is_file() {
                continue;
            }
            
            let file_path = entry.path();
            
            // Skip files in target directory or node_modules
            if file_path.to_string_lossy().contains("/target/") || 
               file_path.to_string_lossy().contains("/node_modules/") {
                continue;
            }
            
            // Check if file has a valid extension
            let is_valid_extension = if let Some(ext) = file_path.extension() {
                if let Some(ext_str) = ext.to_str() {
                    let lowercase_ext = ext_str.to_lowercase();
                    valid_extensions.contains(&lowercase_ext.as_str())
                } else {
                    false
                }
            } else {
                false
            };
            
            if is_valid_extension {
                let signatures = extract_function_signatures(file_path, None);
                all_signatures.extend(signatures);
            }
        }
    }
    
    all_signatures
}
