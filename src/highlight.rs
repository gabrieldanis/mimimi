//! Syntax highlighting using `syntect`, converting to ratatui styled spans.

use std::path::Path;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{self, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::diff::{DiffLine, DiffLineKind};

/// Highlight a set of [`DiffLine`]s using syntect, returning ratatui [`Line`]s.
///
/// Each line gets:
/// - A gutter showing the line number
/// - A diff marker background (green for additions, red for removals)
/// - Syntax-highlighted content based on the file extension
pub fn highlight_diff_lines<'a>(file_path: &str, diff_lines: &[DiffLine]) -> Vec<Line<'a>> {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = &ts.themes["base16-ocean.dark"];

    let syntax = ss
        .find_syntax_by_extension(
            Path::new(file_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("txt"),
        )
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    // Build the full text so syntect can do multi-line stateful highlighting.
    let full_text: String = diff_lines
        .iter()
        .map(|dl| format!("{}\n", dl.content))
        .collect();

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut result = Vec::with_capacity(diff_lines.len());

    for (diff_line, text_line) in diff_lines.iter().zip(LinesWithEndings::from(&full_text)) {
        let bg_color = match diff_line.kind {
            DiffLineKind::Add => Some(Color::Rgb(30, 60, 30)),
            DiffLineKind::Remove => Some(Color::Rgb(60, 30, 30)),
            DiffLineKind::Context => None,
        };

        let marker = match diff_line.kind {
            DiffLineKind::Add => "+",
            DiffLineKind::Remove => "-",
            DiffLineKind::Context => " ",
        };

        // Line number gutter
        let line_num_str = match (diff_line.new_line, diff_line.old_line) {
            (Some(n), _) => format!("{:>4} ", n),
            (_, Some(n)) => format!("{:>4} ", n),
            _ => "     ".to_string(),
        };

        let mut spans: Vec<Span<'a>> = Vec::new();

        // Gutter span
        spans.push(Span::styled(
            line_num_str,
            Style::default().fg(Color::DarkGray),
        ));

        // Diff marker
        let marker_style = match diff_line.kind {
            DiffLineKind::Add => Style::default().fg(Color::Green),
            DiffLineKind::Remove => Style::default().fg(Color::Red),
            DiffLineKind::Context => Style::default().fg(Color::DarkGray),
        };
        spans.push(Span::styled(format!("{marker} "), marker_style));

        // Syntax-highlighted content
        let highlighted = highlighter
            .highlight_line(text_line, &ss)
            .unwrap_or_default();

        for (style, text) in highlighted {
            let text = text.trim_end_matches('\n');
            if text.is_empty() {
                continue;
            }
            let mut ratatui_style = syntect_style_to_ratatui(style);
            if let Some(bg) = bg_color {
                ratatui_style = ratatui_style.bg(bg);
            }
            spans.push(Span::styled(text.to_string(), ratatui_style));
        }

        // If the line has a background color, pad the rest with it.
        if let Some(bg) = bg_color {
            spans.push(Span::styled(" ", Style::default().bg(bg)));
        }

        result.push(Line::from(spans));
    }

    result
}

/// Convert a syntect [`highlighting::Style`] to a ratatui [`Style`].
fn syntect_style_to_ratatui(style: highlighting::Style) -> Style {
    let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
    let mut s = Style::default().fg(fg);
    if style.font_style.contains(highlighting::FontStyle::BOLD) {
        s = s.add_modifier(Modifier::BOLD);
    }
    if style.font_style.contains(highlighting::FontStyle::ITALIC) {
        s = s.add_modifier(Modifier::ITALIC);
    }
    if style
        .font_style
        .contains(highlighting::FontStyle::UNDERLINE)
    {
        s = s.add_modifier(Modifier::UNDERLINED);
    }
    s
}
