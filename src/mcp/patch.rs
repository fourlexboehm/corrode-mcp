use std::{borrow::Cow, str::FromStr};

use anyhow::{Context as _, Result};

/// Represents the range of lines in a hunk header
#[derive(Clone, Debug)]
pub struct HeaderRange {
    /// The line number the patch starts at
    pub start: usize,
    /// The line numbers visible for the patch
    pub range: usize,
}

/// Represents the header of a hunk in a patch
#[derive(Clone, Debug)]
pub struct HunkHeader {
    pub source: HeaderRange,
    #[allow(dead_code)]
    pub dest: HeaderRange,

    // Optional values after fixing the ranges
    pub fixed_source: Option<HeaderRange>,
    pub fixed_dest: Option<HeaderRange>,
}

/// Represents a line in a hunk
#[derive(Clone, Debug, strum_macros::EnumIs)]
pub enum HunkLine {
    Context(String),
    Added(String),
    Removed(String),
}

impl HunkLine {
    pub fn content(&self) -> &str {
        match self {
            HunkLine::Removed(s) | HunkLine::Context(s) | HunkLine::Added(s) => s,
        }
    }

    pub fn as_patch_line(&self) -> Cow<str> {
        match self {
            HunkLine::Context(s) => Cow::Owned(format!(" {s}")),
            HunkLine::Added(s) => Cow::Owned(format!("+{s}")),
            HunkLine::Removed(s) => Cow::Owned(format!("-{s}")),
        }
    }
}

/// Represents a hunk in a patch
#[derive(Clone, Debug)]
pub struct Hunk {
    /// The parsed header of the hunk
    pub header: HunkHeader,

    /// Parsed lines of the hunk
    pub lines: Vec<HunkLine>,

    /// The original full hunk body
    pub body: String,
}

impl<'a> From<&'a Hunk> for Cow<'a, Hunk> {
    fn from(val: &'a Hunk) -> Self {
        Cow::Borrowed(val)
    }
}

impl From<Hunk> for Cow<'_, Hunk> {
    fn from(val: Hunk) -> Self {
        Cow::Owned(val)
    }
}

impl Hunk {
    fn matchable_lines(&self) -> impl Iterator<Item = &HunkLine> {
        self.lines
            .iter()
            .filter(|l| l.is_removed() || l.is_context())
    }

    /// Inserts a line at the given index on matchable lines. Converts the index to the actual
    /// underlying index
    pub fn insert_line_at(&mut self, line: HunkLine, index: usize) {
        self.lines.insert(self.real_index(index), line);
    }

    pub fn real_index(&self, index: usize) -> usize {
        self.lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.is_removed() || l.is_context())
            .nth(index)
            .map_or_else(|| self.lines.len(), |(i, _)| i)
    }

    pub fn matches(&self, line: &str, index: usize, log: bool) -> bool {
        let expected = self
            .matchable_lines()
            .skip(index)
            .map(HunkLine::content)
            .next();

        // let outcome = expected.map(str::trim) == Some(line.trim());
        let outcome = expected == Some(line);

        if log {
            if outcome {
                // Calculate mismatching leading whitespace
                tracing::trace!(line, expected, "Matched line");
            } else {
                tracing::trace!(line, expected, "Did not match line");
            }
        }
        outcome
    }

    pub fn render_updated(&self) -> Result<String> {
        // Extract any context after the second @@ block to add to the new header line
        // i.e. with `@@ -1,2 +2,1 @@ my_function()` we want my_function() to be included
        let header_context = self
            .body
            .lines()
            .next()
            .unwrap_or_default()
            .rsplit("@@")
            .next()
            .unwrap_or_default();

        let source = self
            .header
            .fixed_source
            .as_ref()
            .context("Expected updated source")?;
        let dest = self
            .header
            .fixed_dest
            .as_ref()
            .context("Expected updated dest")?;

        let mut updated = format!(
            "@@ -{},{} +{},{} @@{header_context}\n",
            source.start + 1,
            source.range,
            dest.start + 1,
            dest.range
        );

        for line in &self.lines {
            updated.push_str(&line.as_patch_line());
            updated.push('\n');
        }

        Ok(updated.to_string())
    }
}

/// A hunk that is found in a file
#[derive(Clone, Debug)]
pub struct Candidate<'a> {
    /// The line number in the file we started at
    start: usize,

    /// The current line we are matching against
    current_line: usize,

    hunk: Cow<'a, Hunk>,
}

impl<'a> Candidate<'a> {
    pub fn new(line: usize, hunk: impl Into<Cow<'a, Hunk>>) -> Self {
        Self {
            start: line,
            current_line: 0,
            hunk: hunk.into(),
        }
    }

    /// Number difference in visible lines between the source and destination for the next hunk
    ///
    /// If lines were added, the following hunk will start at an increased line number, if lines
    /// were removed, the following hunk will start at a decreased line number
    #[allow(clippy::cast_possible_wrap)]
    pub fn offset(&self) -> isize {
        self.hunk.lines.iter().filter(|l| l.is_added()).count() as isize
            - self.hunk.lines.iter().filter(|l| l.is_removed()).count() as isize
    }

    pub fn next_line_matches(&self, line: &str) -> bool {
        self.hunk.matches(line, self.current_line, true)
    }

    pub fn is_complete(&self) -> bool {
        // We increment one over the current line, so if we are at the end of the hunk, we are done
        self.current_line == self.hunk.matchable_lines().count()
    }

    pub fn updated_source_header(&self) -> HeaderRange {
        let source_lines = self
            .hunk
            .lines
            .iter()
            .filter(|l| l.is_removed() || l.is_context())
            .count();

        let source_start = self.start;

        HeaderRange {
            start: source_start,
            range: source_lines,
        }
    }

    pub fn updated_dest_header(&self, offset: isize) -> HeaderRange {
        let dest_lines = self
            .hunk
            .lines
            .iter()
            .filter(|l| l.is_added() || l.is_context())
            .count();

        // The offset is the sum off removed and added lines by preceding hunks
        let dest_start = self.start.saturating_add_signed(offset);

        HeaderRange {
            start: dest_start,
            range: dest_lines,
        }
    }
}

impl FromStr for Hunk {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let header: HunkHeader = s.parse()?;
        let lines = s
            .lines()
            .skip(1)
            .map(FromStr::from_str)
            .collect::<Result<Vec<HunkLine>>>()?;

        Ok(Hunk {
            header,
            lines,
            body: s.into(),
        })
    }
}

impl FromStr for HunkLine {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(line) = s.strip_prefix('+') {
            Ok(HunkLine::Added(line.into()))
        } else if let Some(line) = s.strip_prefix('-') {
            Ok(HunkLine::Removed(line.into()))
        } else {
            let s = s.strip_prefix(' ').unwrap_or(s);
            Ok(HunkLine::Context(s.into()))
        }
    }
}

impl FromStr for HunkHeader {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("@@") {
            anyhow::bail!("Hunk header must start with @@");
        }

        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() < 4 {
            anyhow::bail!("Invalid hunk header format");
        }

        let old_range = parts[1].split(',').collect::<Vec<&str>>();
        let new_range = parts[2].split(',').collect::<Vec<&str>>();

        if old_range.len() != 2 || new_range.len() != 2 {
            anyhow::bail!("Invalid range format in hunk header");
        }

        let old_lines = HeaderRange {
            start: old_range[0]
                .replace('-', "")
                .parse()
                .context("Invalid old start line")?,
            range: old_range[1].parse().context("Invalid old range")?,
        };

        let new_lines = HeaderRange {
            start: new_range[0]
                .replace('+', "")
                .parse()
                .context("Invalid new start line")?,
            range: new_range[1].parse().context("Invalid new range")?,
        };

        Ok(HunkHeader {
            source: old_lines,
            dest: new_lines,
            fixed_source: None,
            fixed_dest: None,
        })
    }
}

/// Parses the hunks from a patch
pub fn parse_hunks(patch: &str) -> Result<Vec<Hunk>> {
    let mut hunks = Vec::new();
    let mut current_hunk_lines = Vec::new();

    for line in patch.lines() {
        if line.starts_with("@@") {
            if !current_hunk_lines.is_empty() {
                let hunk = Hunk::from_str(&current_hunk_lines.join("\n"))?;
                hunks.push(hunk);
            }

            current_hunk_lines = vec![line];
        } else if !current_hunk_lines.is_empty() {
            current_hunk_lines.push(line);
        }
    }

    if !current_hunk_lines.is_empty() {
        let hunk = Hunk::from_str(&current_hunk_lines.join("\n"))?;
        hunks.push(hunk);
    }

    Ok(hunks)
}

/// For each hunks, finds potential candidates in the file
///
/// llms are dumb and cannot count
///
/// However, with a patch we can reasonably fix the headers
/// by searching in the neighboring lines of the original hunk header
pub fn find_candidates<'a>(content: &str, hunks: &'a [Hunk]) -> Vec<Candidate<'a>> {
    let mut candidates = Vec::new();

    for (line_n, line) in content.lines().enumerate() {
        // 1. Check if a hunk matches the line, then create a candidate if it does
        if let Some(hunk) = hunks.iter().find(|h| h.matches(line, 0, false)) {
            tracing::trace!(line, "Found hunk match; creating new candidate");
            candidates.push(Candidate::new(line_n, hunk));
        }

        // 2. For each active candidate, check if the next line matches. If it does, increment the
        // the index of the candidate. Otherwise, remove the candidate
        let mut new_candidates = Vec::new();
        candidates.retain_mut(|c| {
            if c.is_complete() {
                true
            } else if c.next_line_matches(line) {
                tracing::trace!(line, "Candidate matched line");
                c.current_line += 1;
                true
            } else if line.trim().is_empty() {
                tracing::trace!(line, "Current line is empty; keeping candidate around");
                // We create a new candidate with a whitespace line added at the index of this
                // candidate. This helps with LLMs misjudging whitespace in the context
                let mut new_hunk: Hunk = c.hunk.clone().into_owned();
                new_hunk.insert_line_at(HunkLine::Context(line.into()), c.current_line);
                let mut new_candidate = Candidate::new(c.start, new_hunk);
                new_candidate.current_line = c.current_line + 1;

                new_candidates.push(new_candidate);
                false
            } else if c
                .hunk
                .lines.iter()
                .skip(c.hunk.real_index(c.current_line + 1))
                .all(HunkLine::is_context)
            {
                // If the following remaining lines, including this one, are context only, accept
                // the current AI overlords incompetence and add a finished candidate without the
                // remaining lines.
                tracing::trace!(line, "Mismatch; remaining is context only, adding finished candidate without the remaining lines");
                let real_index = c.hunk.real_index(c.current_line);
                let mut new_hunk = c.hunk.clone().into_owned();
                new_hunk.lines = new_hunk
                    .lines
                    .iter()
                    .take(real_index)
                    .cloned()
                    .collect();

                let mut new_candidate = Candidate::new(c.start, new_hunk);
                new_candidate.current_line = c.current_line;
                new_candidates.push(new_candidate);
                false
            } else {
                tracing::trace!(line, "Removing candidate");
                false
            }
        });
        candidates.append(&mut new_candidates);
    }

    candidates
}

/// Takes a list of candidates and rebuits the hunk headers
///
/// Filters out duplicates. The resulting hunks should result in a valid patch.
pub fn rebuild_hunks(candidates: &[Candidate<'_>]) -> Vec<Hunk> {
    // Assume that the candidates are sorted by the start line
    // Then we can just iterate over the candidates and update the ranges

    let mut current_offset: isize = 0;
    let mut hunks: Vec<Hunk> = Vec::new();

    for candidate in candidates {
        let source_header = candidate.updated_source_header();

        let dest_header = candidate.updated_dest_header(current_offset);
        current_offset += candidate.offset();

        // Could probably continue the cow, but at this point the number of hunks should be small
        let mut hunk = candidate.hunk.clone().into_owned();
        hunk.header.fixed_source = Some(source_header);
        hunk.header.fixed_dest = Some(dest_header);

        // Filter duplicates. A hunk is a duplicate if the hunk body is the same. If a duplicate
        // is detected, prefer the one with the fixed_source closest to the original source line
        // If so, we swap it with the existing hunk.

        if let Some(existing) = hunks.iter_mut().find(|h| *h.body == hunk.body) {
            let (Some(existing_source), Some(new_source)) =
                (&existing.header.fixed_source, &hunk.header.fixed_source)
            else {
                tracing::warn!("Potential bad duplicate when rebuilding patch; could be a bug, please check the edit");
                continue;
            };

            #[allow(clippy::cast_possible_wrap)]
            if ((existing_source.start as isize)
                .saturating_sub_unsigned(existing.header.source.start))
            .abs()
                < ((new_source.start as isize).saturating_sub_unsigned(hunk.header.source.start))
                    .abs()
            {
                continue;
            }
            *existing = hunk;
        } else {
            hunks.push(hunk);
        }
    }

    hunks
}

/// Takes the file lines from the original patch if possible, then rebuilds the patch
pub fn rebuild_patch(original: &str, hunks: &[Hunk]) -> Result<String> {
    let mut new_patch = original.lines().take(2).collect::<Vec<_>>().join("\n");
    new_patch.push('\n');

    debug_assert!(
        !new_patch.is_empty(),
        "Original file lines in patch tools are empty"
    );

    for hunk in hunks {
        new_patch.push_str(&hunk.render_updated()?);
    }

    Ok(new_patch)
}