// Vendored minimal probe library
// Original from /Users/alex/github/probe
// This is a fully-functional implementation of the core search functionality

pub mod models;
pub mod search;

// Re-export commonly used types for convenience
pub use models::{LimitedSearchResults, SearchResult};
pub use search::{perform_probe, SearchOptions};
