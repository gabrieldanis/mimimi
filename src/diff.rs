//! Parser for unified diff output from `glab mr diff`.

use std::collections::HashMap;

/// A single line from a unified diff.
#[derive(Debug, Clone)]
pub struct DiffLine {
    /// The content of the line (without the leading +/-/space marker).
    pub content: String,
    /// Line number in the new file, if applicable.
    pub new_line: Option<usize>,
    /// Line number in the old file, if applicable.
    pub old_line: Option<usize>,
    /// The kind of diff line.
    pub kind: DiffLineKind,
}

/// Whether a diff line is an addition, removal, or context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Add,
    Remove,
    Context,
}

/// Parsed diff data: maps file paths to their diff lines.
#[derive(Debug, Default)]
pub struct ParsedDiff {
    pub files: HashMap<String, Vec<DiffLine>>,
}

/// Parse raw unified diff text (from `glab mr diff`) into a [`ParsedDiff`].
pub fn parse_unified_diff(raw: &str) -> ParsedDiff {
    let mut files: HashMap<String, Vec<DiffLine>> = HashMap::new();
    let mut current_file: Option<String> = None;
    let mut new_line_num: usize = 0;
    let mut old_line_num: usize = 0;

    for line in raw.lines() {
        // Detect new file header: "+++ b/path" or "+++ path"
        if let Some(path) = line.strip_prefix("+++ ") {
            let path = path.strip_prefix("b/").unwrap_or(path);
            current_file = Some(path.to_string());
            files.entry(path.to_string()).or_default();
            continue;
        }

        // Skip "--- " lines
        if line.starts_with("--- ") {
            continue;
        }

        // Parse hunk headers: @@ -old_start,old_count +new_start,new_count @@
        if line.starts_with("@@ ") {
            if let Some((old_start, new_start)) = parse_hunk_header(line) {
                old_line_num = old_start;
                new_line_num = new_start;
            }
            continue;
        }

        let Some(ref file) = current_file else {
            continue;
        };

        let lines = files.entry(file.clone()).or_default();

        if let Some(content) = line.strip_prefix('+') {
            lines.push(DiffLine {
                content: content.to_string(),
                new_line: Some(new_line_num),
                old_line: None,
                kind: DiffLineKind::Add,
            });
            new_line_num += 1;
        } else if let Some(content) = line.strip_prefix('-') {
            lines.push(DiffLine {
                content: content.to_string(),
                new_line: None,
                old_line: Some(old_line_num),
                kind: DiffLineKind::Remove,
            });
            old_line_num += 1;
        } else if let Some(content) = line.strip_prefix(' ') {
            lines.push(DiffLine {
                content: content.to_string(),
                new_line: Some(new_line_num),
                old_line: Some(old_line_num),
                kind: DiffLineKind::Context,
            });
            new_line_num += 1;
            old_line_num += 1;
        }
        // Skip other lines (e.g. "\ No newline at end of file")
    }

    ParsedDiff { files }
}

/// Parse hunk header like `@@ -1,3 +1,5 @@` and return (old_start, new_start).
fn parse_hunk_header(line: &str) -> Option<(usize, usize)> {
    // Format: @@ -old_start[,old_count] +new_start[,new_count] @@
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }
    let old_start = parts[1]
        .strip_prefix('-')?
        .split(',')
        .next()?
        .parse::<usize>()
        .ok()?;
    let new_start = parts[2]
        .strip_prefix('+')?
        .split(',')
        .next()?
        .parse::<usize>()
        .ok()?;
    Some((old_start, new_start))
}

/// Extract lines from a parsed diff around a given new-file line number,
/// with `context` lines above and below. Returns the matching [`DiffLine`]s.
pub fn extract_context(
    diff: &ParsedDiff,
    file_path: &str,
    target_new_line: usize,
    context: usize,
) -> Vec<DiffLine> {
    let Some(lines) = diff.files.get(file_path) else {
        return Vec::new();
    };

    // Find the index of the target line in the diff lines vec.
    let target_idx = lines
        .iter()
        .position(|l| l.new_line == Some(target_new_line));

    let Some(target_idx) = target_idx else {
        return Vec::new();
    };

    let start = target_idx.saturating_sub(context);
    let end = (target_idx + context + 1).min(lines.len());

    lines[start..end].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_diff() {
        let raw = "\
--- a/hello.txt
+++ b/hello.txt
@@ -1,3 +1,4 @@
 line one
-old line two
+new line two
+inserted line
 line three
";
        let parsed = parse_unified_diff(raw);
        let lines = parsed.files.get("hello.txt").unwrap();
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0].kind, DiffLineKind::Context);
        assert_eq!(lines[0].content, "line one");
        assert_eq!(lines[1].kind, DiffLineKind::Remove);
        assert_eq!(lines[2].kind, DiffLineKind::Add);
        assert_eq!(lines[3].kind, DiffLineKind::Add);
        assert_eq!(lines[4].kind, DiffLineKind::Context);
        assert_eq!(lines[4].content, "line three");
    }

    #[test]
    fn test_extract_context_lines() {
        let raw = "\
--- a/code.js
+++ b/code.js
@@ -0,0 +1,6 @@
+console.log('hi there!');
+
+const myVar = \"test\";
+
+
+console.log('what?', myVar);
";
        let parsed = parse_unified_diff(raw);
        let ctx = extract_context(&parsed, "code.js", 3, 2);
        // Should get lines 1..5 (target=3, context=2 above/below)
        assert_eq!(ctx.len(), 5);
        assert_eq!(ctx[2].new_line, Some(3));
    }

    #[test]
    fn test_parse_no_prefix_paths() {
        // glab mr diff sometimes outputs without a/ b/ prefixes
        let raw = "\
--- NewIdea.md
+++ NewIdea.md
@@ -0,0 +1,2 @@
+hello
+world
";
        let parsed = parse_unified_diff(raw);
        assert!(parsed.files.contains_key("NewIdea.md"));
        assert_eq!(parsed.files["NewIdea.md"].len(), 2);
    }
}
