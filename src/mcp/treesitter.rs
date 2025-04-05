#![allow(dead_code)]
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tree_sitter::{Language, Node, Parser, Query, QueryCursor};

extern crate tree_sitter_rust as rust;
extern crate tree_sitter_javascript as javascript;
extern crate tree_sitter_python as python;
extern crate tree_sitter_typescript as typescript;
extern crate tree_sitter_go as go;
extern crate tree_sitter_c as c;
extern crate tree_sitter_cpp as cpp;

#[derive(Serialize, Deserialize)]
pub struct ProjectStructure {
    pub files: HashMap<String, FileInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub language: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub functions: Vec<FunctionInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<ClassInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub structs: Vec<StructInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub enums: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<FunctionInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct StructInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<FunctionInfo>,
}

// Map file extension to language
pub fn detect_language(file_path: &Path, language_override: Option<&str>) -> Option<(Language, String)> {
    if let Some(lang) = language_override {
        return match lang {
            "rust" => Some((unsafe { tree_sitter_rust() }, "rust".to_string())),
            "javascript" => Some((unsafe { tree_sitter_javascript() }, "javascript".to_string())),
            "typescript" => Some((unsafe { tree_sitter_typescript() }, "typescript".to_string())),
            "python" => Some((unsafe { tree_sitter_python() }, "python".to_string())),
            "go" => Some((unsafe { tree_sitter_go() }, "go".to_string())),
            "c" => Some((unsafe { tree_sitter_c() }, "c".to_string())),
            "cpp" => Some((unsafe { tree_sitter_cpp() }, "cpp".to_string())),
            _ => None,
        };
    }

    if let Some(extension) = file_path.extension() {
        match extension.to_str() {
            Some("rs") => Some((unsafe { tree_sitter_rust() }, "rust".to_string())),
            Some("js") => Some((unsafe { tree_sitter_javascript() }, "javascript".to_string())),
            Some("ts") => Some((unsafe { tree_sitter_typescript() }, "typescript".to_string())),
            Some("py") => Some((unsafe { tree_sitter_python() }, "python".to_string())),
            Some("go") => Some((unsafe { tree_sitter_go() }, "go".to_string())),
            Some("c") => Some((unsafe { tree_sitter_c() }, "c".to_string())),
            Some("h") => Some((unsafe { tree_sitter_c() }, "c".to_string())),
            Some("cpp") => Some((unsafe { tree_sitter_cpp() }, "cpp".to_string())),
            Some("hpp") => Some((unsafe { tree_sitter_cpp() }, "cpp".to_string())),
            _ => None,
        }
    } else {
        None
    }
}

fn get_query_for_language(language: Language) -> Option<String> {
    if language == unsafe { tree_sitter_rust() } {
        Some(
            r#"
            (function_item name: (identifier) @function) 
            (impl_item name: (type_identifier) @impl) 
            (struct_item name: (type_identifier) @struct) 
            (enum_item name: (type_identifier) @enum) 
            (trait_item name: (type_identifier) @trait) 
            (function_item name: (identifier) @method) 
            (use_declaration) @import"#
                .to_string(),
        )
    } else if language == unsafe { tree_sitter_javascript() } || language == unsafe { tree_sitter_typescript() } {
        Some(
            r#"
            (function_declaration name: (identifier) @function)
            (method_definition name: (property_identifier) @method)
            (class_declaration name: (identifier) @class)
            (import_statement) @import
        "#
            .to_string(),
        )
    } else if language == unsafe { tree_sitter_python() } {
        Some(
            r#"
            (function_definition name: (identifier) @function)
            (class_definition name: (identifier) @class)
            (import_statement) @import
        "#
            .to_string(),
        )
    } else if language == unsafe { tree_sitter_go() } {
        Some(
            r#"
            (function_declaration name: (identifier) @function)
            (method_declaration name: (field_identifier) @method receiver: (parameter_list) @receiver)
            (type_declaration (type_spec name: (type_identifier) @type))
            (import_declaration) @import
        "#
            .to_string(),
        )
    } else if language == unsafe { tree_sitter_c() } || language == unsafe { tree_sitter_cpp() } {
        Some(
            r#"
            (function_definition declarator: (function_declarator declarator: (identifier) @function))
            (struct_specifier name: (type_identifier) @struct)
            (class_specifier name: (type_identifier) @class)
            (enum_specifier name: (type_identifier) @enum)
            (include_directive) @import
        "#
            .to_string(),
        )
    } else {
        None
    }
}

// Get the line number from a node's position
pub fn get_line(node: &Node, _source: &str) -> usize {
    let pos = node.start_position();
    // TreeSitter positions are 0-based, add 1 for human readability
    pos.row + 1
}

// Get the end line number of a node
fn get_end_line(node: &Node, _source: &str) -> Option<usize> {
    let pos = node.end_position();
    // TreeSitter positions are 0-based, add 1 for human readability
    Some(pos.row + 1)
}

// Parse a single file and return its structure
pub fn parse_file(file_path: &Path, language_override: Option<&str>) -> Option<FileInfo> {
    // Read the file content
    let source = fs::read_to_string(file_path).ok()?;
    
    // Detect language and set up TreeSitter
    let (lang, lang_name) = detect_language(file_path, language_override)?;
    
    let mut parser = Parser::new();
    if parser.set_language(lang).is_err() {
        return None;
    }
    
    // Parse the source code
    let tree = parser.parse(&source, None)?;
    let root_node = tree.root_node();
    
    // Create the query for extracting code structure
    let query_string = get_query_for_language(lang)?;
    
    // Try to create the query, handling potential errors
    let query = Query::new(lang, &query_string).ok()?;
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, root_node, source.as_bytes());
    
    let mut functions: Vec<FunctionInfo> = Vec::new();
    let mut classes: Vec<ClassInfo> = Vec::new();
    let mut structs: Vec<StructInfo> = Vec::new();
    let mut enums: Vec<String> = Vec::new();
    let mut imports: Vec<String> = Vec::new();
    
    for m in matches {
        for capture in m.captures {
            let capture_name = query.capture_names()[capture.index as usize].to_string();
            let name = capture.node.utf8_text(source.as_bytes()).unwrap_or("unknown").to_string();
            
            match capture_name.as_str() {
                "function" => {
                    functions.push(FunctionInfo {
                        name,
                        start_line: get_line(&capture.node, &source),
                        end_line: get_end_line(&capture.node, &source),
                        parent: None,
                    });
                },
                "class" => {
                    classes.push(ClassInfo {
                        name,
                        start_line: get_line(&capture.node, &source),
                        end_line: get_end_line(&capture.node, &source),
                        methods: Vec::new(),
                    });
                },
                "struct" => {
                    structs.push(StructInfo {
                        name,
                        start_line: get_line(&capture.node, &source),
                        end_line: get_end_line(&capture.node, &source),
                        fields: Vec::new(),
                        methods: Vec::new(),
                    });
                },
                "enum" => {
                    enums.push(name);
                },
                "import" => {
                    imports.push(name);
                },
                _ => {}
            }
        }
    }
    
    let rel_path = file_path.to_string_lossy().to_string();
    
    Some(FileInfo {
        path: rel_path,
        language: lang_name,
        functions,
        classes,
        structs,
        enums,
        imports,
    })
}


// Safely get the language functions
pub unsafe fn tree_sitter_rust() -> Language {
    rust::language()
}
pub unsafe fn tree_sitter_javascript() -> Language {
    javascript::language()
}
// For TypeScript, use a workaround since the function might be named differently
pub unsafe fn tree_sitter_typescript() -> Language {
    // Some crate versions use different function names for TypeScript
    // Fall back to JavaScript if TypeScript fails
    typescript::language_typescript()
}
pub unsafe fn tree_sitter_python() -> Language {
    python::language()
}
pub unsafe fn tree_sitter_go() -> Language {
    go::language()
}
pub unsafe fn tree_sitter_c() -> Language {
    c::language()
}
pub unsafe fn tree_sitter_cpp() -> Language {
    cpp::language()
}