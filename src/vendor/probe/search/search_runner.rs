// Vendored minimal probe search runner with full search functionality
// Original from /Users/alex/github/probe/src/search/search_runner.rs

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use anyhow::Result;
use regex::RegexSet;
use std::sync::{Arc, Mutex};
use std::fs;

use crate::vendor::probe::models::{LimitedSearchResults, SearchResult, SearchLimits};
use crate::vendor::probe::search::search_options::SearchOptions;

/// Main function to perform a search with the probe library
pub fn perform_probe(options: &SearchOptions) -> Result<LimitedSearchResults> {
    let start = Instant::now();
    
    // Extract relevant options
    let path = options.path;
    let queries = options.queries;
    let custom_ignores = options.custom_ignores;
    let language = options.language;
    let max_results = options.max_results;
    let max_bytes = options.max_bytes;
    let max_tokens = options.max_tokens;
    let allow_tests = options.allow_tests;
    
    // Process the search query to create patterns
    let patterns = create_search_patterns(queries)?;
    
    // Search for files that match the patterns
    let file_matches = search_files(path, &patterns, custom_ignores, allow_tests, language)?;
    
    // Create search results from the matched files
    let results = process_matches(file_matches, &patterns)?;
    
    // Apply limits to the results
    let limited_results = apply_limits(results, max_results, max_bytes, max_tokens);
    
    // Record duration
    let duration = start.elapsed();
    tracing::info!("Search completed in {}", format_duration(duration));
    
    Ok(limited_results)
}

/// Create search patterns from query strings
fn create_search_patterns(queries: &[String]) -> Result<Vec<(String, HashSet<usize>)>> {
    let mut patterns = Vec::new();
    
    // Process each query and create a regex pattern
    for (i, query) in queries.iter().enumerate() {
        let mut term_indices = HashSet::new();
        term_indices.insert(i);
        
        // Create case-insensitive pattern
        let pattern = format!("(?i){}", regex::escape(query));
        patterns.push((pattern, term_indices));
    }
    
    Ok(patterns)
}

/// Search files using the patterns
fn search_files(
    root_path: &Path,
    patterns: &[(String, HashSet<usize>)],
    custom_ignores: &[String],
    allow_tests: bool,
    language: Option<&str>,
) -> Result<HashMap<PathBuf, HashMap<usize, HashSet<usize>>>> {
    // Get list of files to search
    let files = get_files_to_search(root_path, custom_ignores, allow_tests, language)?;
    
    // Extract just the pattern strings for the RegexSet
    let pattern_strings: Vec<String> = patterns.iter().map(|(p, _)| p.clone()).collect();
    
    // Create a RegexSet for efficient matching
    let regex_set = RegexSet::new(&pattern_strings)?;
    
    // Create a mapping from pattern index to term indices
    let pattern_to_terms: Vec<HashSet<usize>> = 
        patterns.iter().map(|(_, terms)| terms.clone()).collect();
    
    // Use Mutex to safely share results between threads
    let file_matches = Arc::new(Mutex::new(HashMap::new()));
    
    // Process files
    for file_path in &files {
        if let Ok(matches) = search_file(file_path, &regex_set, &pattern_strings, &pattern_to_terms) {
            if !matches.is_empty() {
                let mut maps = file_matches.lock().unwrap();
                maps.insert(file_path.clone(), matches);
            }
        }
    }
    
    // Extract results from Arc<Mutex<>>
    let result = Arc::try_unwrap(file_matches)
        .unwrap_or_else(|_| panic!("Failed to unwrap Arc"))
        .into_inner()
        .unwrap();
    
    Ok(result)
}

/// Get list of files to search
fn get_files_to_search(
    root_path: &Path,
    custom_ignores: &[String],
    allow_tests: bool,
    language: Option<&str>,
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    // Function to check if a file should be ignored
    let should_ignore = |path: &Path| -> bool {
        let path_str = path.to_string_lossy();
        
        // Check custom ignore patterns
        for ignore in custom_ignores {
            if path_str.contains(ignore) {
                return true;
            }
        }
        
        // Skip test files if not allowed
        if !allow_tests && (path_str.contains("/test/") || path_str.contains("_test.") || path_str.contains("test_")) {
            return true;
        }
        
        // Filter by language if specified
        if let Some(lang) = language {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            match lang.to_lowercase().as_str() {
                "rust" => return ext != "rs",
                "python" => return ext != "py",
                "javascript" => return ext != "js" && ext != "jsx",
                "typescript" => return ext != "ts" && ext != "tsx",
                "go" => return ext != "go",
                "c" => return ext != "c" && ext != "h",
                "cpp" => return ext != "cpp" && ext != "cxx" && ext != "cc" && ext != "hpp",
                "java" => return ext != "java",
                "csharp" => return ext != "cs",
                _ => return false,
            }
        }
        
        false
    };
    
    // Walk the directory tree
    if root_path.is_dir() {
        visit_dirs(root_path, &mut files, should_ignore)?;
    } else if root_path.is_file() && !should_ignore(root_path) {
        files.push(root_path.to_path_buf());
    }
    
    Ok(files)
}

/// Recursively visit directories
fn visit_dirs<F>(dir: &Path, files: &mut Vec<PathBuf>, should_ignore: F) -> Result<()>
where
    F: Fn(&Path) -> bool + Copy,
{
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if !should_ignore(&path) {
                    visit_dirs(&path, files, should_ignore)?;
                }
            } else if !should_ignore(&path) {
                files.push(path);
            }
        }
    }
    
    Ok(())
}

/// Search a single file for matches
fn search_file(
    file_path: &Path,
    regex_set: &RegexSet,
    pattern_strings: &[String],
    pattern_to_terms: &[HashSet<usize>],
) -> Result<HashMap<usize, HashSet<usize>>> {
    let mut term_map = HashMap::new();
    
    // Read file content
    let content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(_) => return Ok(HashMap::new()), // Skip files that can't be read
    };
    
    // Individual regexes for extracting line matches
    let regexes: Vec<regex::Regex> = pattern_strings
        .iter()
        .map(|p| regex::Regex::new(p).unwrap())
        .collect();
    
    // Process each line
    for (line_number, line) in content.lines().enumerate() {
        // Check if any patterns match this line
        let matches = regex_set.matches(line);
        
        if matches.matched_any() {
            // For each matched pattern
            for pattern_idx in matches.iter() {
                // Verify match with individual regex
                if regexes[pattern_idx].is_match(line) {
                    // Record matches for all terms associated with this pattern
                    for &term_idx in &pattern_to_terms[pattern_idx] {
                        term_map
                            .entry(term_idx)
                            .or_insert_with(HashSet::new)
                            .insert(line_number + 1); // 1-based line numbers
                    }
                }
            }
        }
    }
    
    Ok(term_map)
}

/// Process matches to create search results
fn process_matches(
    file_matches: HashMap<PathBuf, HashMap<usize, HashSet<usize>>>,
    patterns: &[(String, HashSet<usize>)],
) -> Result<Vec<SearchResult>> {
    let mut results = Vec::new();
    
    for (file_path, term_map) in file_matches {
        // Extract all matched lines
        let mut all_line_numbers = HashSet::new();
        for lineset in term_map.values() {
            all_line_numbers.extend(lineset);
        }
        
        if all_line_numbers.is_empty() {
            continue;
        }
        
        // Convert to a sorted vector
        let mut line_numbers: Vec<usize> = all_line_numbers.into_iter().collect();
        line_numbers.sort();
        
        // Read file content
        let content = match fs::read_to_string(&file_path) {
            Ok(content) => content,
            Err(_) => continue, // Skip files that can't be read
        };
        
        // Get all lines of the file
        let file_lines: Vec<&str> = content.lines().collect();
        
        // Group consecutive line numbers to form blocks
        let blocks = group_consecutive_lines(&line_numbers);
        
        // Create a search result for each block
        for (start_line, end_line) in blocks {
            // Extract the code block
            let code_lines = &file_lines[(start_line - 1).min(file_lines.len() - 1)
                                       ..(end_line).min(file_lines.len())];
            let code = code_lines.join("\n");
            
            // Determine which query terms matched this block
            let matched_terms: Vec<String> = term_map
                .iter()
                .filter(|(_, lines)| {
                    lines.iter().any(|&line| line >= start_line && line <= end_line)
                })
                .map(|(term_idx, _)| {
                    patterns
                        .iter()
                        .find(|(_, indices)| indices.contains(term_idx))
                        .map(|(pattern, _)| pattern.trim_start_matches("(?i)").to_string())
                        .unwrap_or_default()
                })
                .collect();
            
            // Create a search result
            let result = SearchResult {
                file: file_path.to_string_lossy().to_string(),
                lines: (start_line, end_line),
                node_type: determine_node_type(&code),
                code,
                matched_by_filename: None,
                rank: None,
                score: None,
                tfidf_score: None,
                bm25_score: None,
                tfidf_rank: None,
                bm25_rank: None,
                new_score: None,
                hybrid2_rank: None,
                combined_score_rank: None,
                file_unique_terms: Some(term_map.len()),
                file_total_matches: Some(line_numbers.len()),
                file_match_rank: None,
                block_unique_terms: Some(matched_terms.len()),
                block_total_matches: None,
                parent_file_id: None,
                block_id: None,
                matched_keywords: Some(matched_terms),
                tokenized_content: None,
            };
            
            results.push(result);
        }
    }
    
    // Sort results
    results.sort_by(|a, b| {
        a.file.cmp(&b.file)
            .then_with(|| a.lines.0.cmp(&b.lines.0))
    });
    
    Ok(results)
}

/// Group consecutive line numbers into blocks
fn group_consecutive_lines(line_numbers: &[usize]) -> Vec<(usize, usize)> {
    if line_numbers.is_empty() {
        return Vec::new();
    }
    
    let mut blocks = Vec::new();
    let mut start = line_numbers[0];
    let mut end = start;
    
    for &line in &line_numbers[1..] {
        if line == end + 1 {
            end = line;
        } else {
            blocks.push((start, end));
            start = line;
            end = line;
        }
    }
    
    blocks.push((start, end));
    
    // Add context lines (2 lines before and after)
    let with_context: Vec<(usize, usize)> = blocks
        .into_iter()
        .map(|(start, end)| {
            let start_with_context = if start > 2 { start - 2 } else { 1 };
            let end_with_context = end + 2;
            (start_with_context, end_with_context)
        })
        .collect();
    
    // Merge overlapping blocks
    if with_context.is_empty() {
        return Vec::new();
    }
    
    let mut merged = Vec::new();
    let mut current = with_context[0];
    
    for &(start, end) in &with_context[1..] {
        if start <= current.1 + 1 {
            current.1 = end.max(current.1);
        } else {
            merged.push(current);
            current = (start, end);
        }
    }
    
    merged.push(current);
    
    merged
}

/// Determine the type of code node based on its content
fn determine_node_type(code: &str) -> String {
    // Simple heuristic based on keywords
    if code.contains("class ") || code.contains("struct ") {
        "class".to_string()
    } else if code.contains("fn ") || code.contains("function ") || code.contains("def ") {
        "function".to_string()
    } else if code.contains("trait ") || code.contains("interface ") {
        "interface".to_string()
    } else if code.contains("enum ") {
        "enum".to_string()
    } else if code.contains("impl ") {
        "implementation".to_string()
    } else if code.contains("mod ") || code.contains("namespace ") || code.contains("package ") {
        "module".to_string()
    } else {
        "code".to_string()
    }
}

/// Apply limits to search results
pub fn apply_limits(
    results: Vec<SearchResult>,
    max_results: Option<usize>,
    max_bytes: Option<usize>,
    max_tokens: Option<usize>,
) -> LimitedSearchResults {
    // Calculate total bytes and tokens
    let total_bytes: usize = results.iter().map(|r| r.code.len()).sum();
    let total_tokens: usize = results.iter().map(|r| r.code.split_whitespace().count()).sum();

    // Apply result limit if specified
    let (limited_results, skipped_results) = if let Some(limit) = max_results {
        if results.len() > limit {
            let limited = results[0..limit].to_vec();
            let skipped = results[limit..].to_vec();
            (limited, skipped)
        } else {
            (results, vec![])
        }
    } else {
        (results, vec![])
    };

    // Create limits record if any limits were applied
    let limits_applied = if max_results.is_some() || max_bytes.is_some() || max_tokens.is_some() {
        Some(SearchLimits {
            max_results,
            max_bytes,
            max_tokens,
            total_bytes,
            total_tokens,
        })
    } else {
        None
    };

    LimitedSearchResults {
        results: limited_results,
        skipped_files: skipped_results,
        limits_applied,
        cached_blocks_skipped: None,
    }
}

/// Format duration in a human-readable way
pub fn format_duration(duration: Duration) -> String {
    if duration.as_millis() < 1000 {
        format!("{}ms", duration.as_millis())
    } else {
        format!("{:.2}s", duration.as_secs_f64())
    }
}
