// Vendored probe search options from /Users/alex/github/probe/src/search/search_options.rs

use std::path::Path;

/// Options for configuring a search operation
pub struct SearchOptions<'a> {
    /// Root path to search in
    pub path: &'a Path,

    /// Search query strings (multiple queries are joined with OR)
    pub queries: &'a [String],

    /// If true, only return file names without searching content
    pub files_only: bool,

    /// Custom ignore patterns (e.g., ["vendor/", "node_modules/"])
    pub custom_ignores: &'a [String],

    /// If true, exclude files that match by filename
    pub exclude_filenames: bool,

    /// The reranker to use for result ranking (default, tfidf, bm25, freq, hybrid)
    pub reranker: &'a str,

    /// If true, search for frequencies of terms
    pub frequency_search: bool,

    /// If true, require exact matches (case-sensitive)
    pub exact: bool,

    /// Optional language filter (e.g., "rust", "python")
    pub language: Option<&'a str>,

    /// Maximum number of results to return
    pub max_results: Option<usize>,

    /// Maximum number of bytes in results
    pub max_bytes: Option<usize>,

    /// Maximum number of tokens in results
    pub max_tokens: Option<usize>,

    /// If true, include test files in search
    pub allow_tests: bool,

    /// If true, don't merge adjacent matching blocks
    pub no_merge: bool,

    /// Distance threshold for merging blocks
    pub merge_threshold: usize,

    /// If true, don't actually perform the search
    pub dry_run: bool,

    /// Optional session ID for caching
    pub session: Option<&'a str>,

    /// Timeout in seconds (0 = no timeout)
    pub timeout: u64,
}

impl<'a> Default for SearchOptions<'a> {
    fn default() -> Self {
        Self {
            path: Path::new("."),
            queries: &[],
            files_only: false,
            custom_ignores: &[],
            exclude_filenames: false,
            reranker: "default",
            frequency_search: false,
            exact: false,
            language: None,
            max_results: Some(100),
            max_bytes: None,
            max_tokens: None,
            allow_tests: true,
            no_merge: false,
            merge_threshold: 5,
            dry_run: false,
            session: None,
            timeout: 30,
        }
    }
}
