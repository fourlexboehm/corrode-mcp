use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct FunctionSignature {
    pub file_path: String,
    pub name: String,
    pub signature: String,
    pub line_number: usize,
    pub parent: Option<String>,
    pub language: String,
}
