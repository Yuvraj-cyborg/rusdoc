use owo_colors::OwoColorize;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};

use crate::cli::OutputFormat;
use crate::doc::DocItem;
use crate::resolver::item_kind_label;

pub fn render_item(item: &DocItem<'_>, format: OutputFormat) -> String {
    match format {
        OutputFormat::Rich => render_rich(item),
        OutputFormat::Plain => render_plain(item),
        OutputFormat::Json => render_json(item),
    }
}

fn render_rich(item: &DocItem<'_>) -> String {
    let mut out = String::new();
    let width = term_width();

    // Header
    let kind = item_kind_label(item.inner);
    let path = item.path.join("::");
    out.push_str(&format!(" {} {}\n", kind.dimmed(), path.bold().cyan()));
    out.push_str(&format!(" {}\n", "─".repeat(width.saturating_sub(2).min(path.len() + kind.len() + 2))));
    out.push('\n');

    if let Some(dep) = &item.deprecation {
        let note = dep.note.as_deref().unwrap_or("deprecated");
        out.push_str(&format!("  {} {}\n\n", "DEPRECATED:".yellow().bold(), note.yellow()));
    }

    if let Some(docs) = &item.docs {
        out.push_str(&render_markdown_rich(docs, width));
    } else {
        out.push_str(&format!("  {}\n", "(no documentation)".dimmed()));
    }

    out.push('\n');
    out
}

/// Accumulate inline content into a buffer, then flush as a wrapped paragraph.
/// This avoids the choppy per-event wrapping problem.
fn render_markdown_rich(markdown: &str, width: usize) -> String {
    // Strip reference-link definitions (e.g. [link text]: url) from the end,
    // and clean up intra-doc link references like [text][crate::foo]
    let cleaned = clean_rustdoc_links(markdown);
    let parser = Parser::new(&cleaned);
    let mut out = String::new();

    // Accumulates plain text for an entire block,
    // then gets wrapped and flushed all at once.
    let mut para = String::new();
    let mut in_code_block = false;
    let mut code_buf = String::new();
    let mut in_heading = false;
    let mut heading_buf = String::new();
    let mut _in_list_item = false;
    let wrap_width = width.saturating_sub(4); // 2 indent + 2 margin

    let flush_para = |para: &mut String, out: &mut String, prefix: &str| {
        if para.is_empty() {
            return;
        }
        let text = std::mem::take(para);
        let wrapped = textwrap::fill(text.trim(), wrap_width);
        for line in wrapped.lines() {
            out.push_str(prefix);
            out.push_str(line);
            out.push('\n');
        }
    };

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_)))
            | Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)) => {
                flush_para(&mut para, &mut out, "  ");
                in_code_block = true;
                code_buf.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                out.push('\n');
                for line in code_buf.lines() {
                    out.push_str(&format!("    {}\n", line.green()));
                }
                out.push('\n');
                code_buf.clear();
            }

            Event::Start(Tag::Heading { .. }) => {
                flush_para(&mut para, &mut out, "  ");
                in_heading = true;
                heading_buf.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                out.push('\n');
                out.push_str(&format!("  {}\n\n", heading_buf.trim().bold()));
                heading_buf.clear();
            }

            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                flush_para(&mut para, &mut out, "  ");
                out.push('\n');
            }

            Event::Start(Tag::Item) => {
                flush_para(&mut para, &mut out, "  ");
                _in_list_item = true;
                para.push_str("- ");
            }
            Event::End(TagEnd::Item) => {
                flush_para(&mut para, &mut out, "  ");
                _in_list_item = false;
            }
            Event::Start(Tag::List(_)) => {}
            Event::End(TagEnd::List(_)) => {
                out.push('\n');
            }

            // Inline: bold/italic/links (just pass through)
            Event::Start(Tag::Strong) => para.push_str("\x1b[1m"),
            Event::End(TagEnd::Strong) => para.push_str("\x1b[22m"),
            Event::Start(Tag::Emphasis) => para.push_str("\x1b[3m"),
            Event::End(TagEnd::Emphasis) => para.push_str("\x1b[23m"),
            Event::Start(Tag::Link { dest_url, .. }) => {
                // Just show the link text, drop the URL
                let _ = dest_url;
            }
            Event::End(TagEnd::Link) => {}

            Event::Text(text) => {
                if in_code_block {
                    code_buf.push_str(&text);
                } else if in_heading {
                    heading_buf.push_str(&text);
                } else {
                    para.push_str(&text);
                }
            }
            Event::Code(code) => {
                if in_heading {
                    heading_buf.push_str(&code);
                } else {
                    para.push_str(&format!("{}", code.cyan()));
                }
            }
            Event::SoftBreak => {
                if !para.is_empty() {
                    para.push(' ');
                }
            }
            Event::HardBreak => {
                flush_para(&mut para, &mut out, "  ");
            }
            _ => {}
        }
    }

    // Flush anything remaining
    flush_para(&mut para, &mut out, "  ");

    out
}

fn render_plain(item: &DocItem<'_>) -> String {
    let mut out = String::new();
    let width = term_width();

    let kind = item_kind_label(item.inner);
    let path = item.path.join("::");
    out.push_str(&format!(" {kind} {path}\n"));
    out.push_str(&format!(" {}\n", "-".repeat(width.saturating_sub(2).min(path.len() + kind.len() + 2))));
    out.push('\n');

    if let Some(dep) = &item.deprecation {
        let note = dep.note.as_deref().unwrap_or("deprecated");
        out.push_str(&format!("  DEPRECATED: {note}\n\n"));
    }

    if let Some(docs) = &item.docs {
        let wrapped = textwrap::fill(docs, width.saturating_sub(4));
        for line in wrapped.lines() {
            out.push_str("  ");
            out.push_str(line);
            out.push('\n');
        }
    } else {
        out.push_str("  (no documentation)\n");
    }

    out.push('\n');
    out
}

fn render_json(item: &DocItem<'_>) -> String {
    let value = serde_json::json!({
        "name": item.name,
        "path": item.path.join("::"),
        "kind": item_kind_label(item.inner),
        "docs": item.docs,
        "deprecated": item.deprecation.is_some(),
    });

    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string())
}

fn term_width() -> usize {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(80)
}

/// Clean up rustdoc-specific markdown before rendering:
/// - `[text][crate::path]` → `text`
/// - `[`Type`]` → `Type`
/// - Strip link reference definitions at the end
fn clean_rustdoc_links(md: &str) -> String {
    let mut result = String::with_capacity(md.len());
    let mut lines_to_skip = std::collections::HashSet::new();

    // Find reference definition lines like `[name]: url`
    for (i, line) in md.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.contains("]: ") {
            lines_to_skip.insert(i);
        }
    }

    for (i, line) in md.lines().enumerate() {
        if lines_to_skip.contains(&i) {
            continue;
        }
        let cleaned = clean_line(line);
        result.push_str(&cleaned);
        result.push('\n');
    }

    result
}

fn clean_line(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '[' {
            // Try to match `[text][ref]` or `[text](url)` or `[`code`]`
            if let Some((text, end)) = try_parse_link(&chars, i) {
                result.push_str(&text);
                i = end;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Try to parse a markdown link starting at position `start`.
/// Returns (display_text, end_position) if successful.
fn try_parse_link(chars: &[char], start: usize) -> Option<(String, usize)> {
    let len = chars.len();

    let mut depth = 0;
    let mut close_bracket = None;
    for j in start..len {
        match chars[j] {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    close_bracket = Some(j);
                    break;
                }
            }
            _ => {}
        }
    }
    let cb = close_bracket?;
    let inner: String = chars[start + 1..cb].iter().collect();

    let after = cb + 1;
    if after < len {
        match chars[after] {
            // `[text](url)` → show "text"
            '(' => {
                let mut paren_end = None;
                for j in after..len {
                    if chars[j] == ')' {
                        paren_end = Some(j);
                        break;
                    }
                }
                if let Some(pe) = paren_end {
                    return Some((inner, pe + 1));
                }
            }
            // `[text][ref]` → show "text"
            '[' => {
                let mut ref_end = None;
                for j in after..len {
                    if chars[j] == ']' {
                        ref_end = Some(j);
                        break;
                    }
                }
                if let Some(re) = ref_end {
                    return Some((inner, re + 1));
                }
            }
            _ => {}
        }
    }

    // Bare `[text]` with no following `(` or `[` — likely an unresolved
    // intra-doc link like `[`Type`]` or `[section name]`. Strip brackets.
    let display = if inner.starts_with('`') && inner.ends_with('`') && inner.len() > 1 {
        inner[1..inner.len() - 1].to_string()
    } else {
        inner
    };
    Some((display, cb + 1))
}
