//! Markdown for AI messages: inline (**bold**, `code`), blocks (# Header, - list, ``` code, ---).
//!
//! No external crate. Used by [crate::messages::ai_message] to style AI text.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::layouts::{text_muted_style, text_style};
use crate::theme::LocusPalette;
use crate::utils::{wrap_lines, LEFT_PADDING};

// --- Inline (existing) ---

/// Parse a single line for inline markdown: **bold** and `code`. Returns styled spans.
pub fn parse_inline_markdown(line: &str, palette: &LocusPalette) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut i = 0;
    let bytes = line.as_bytes();
    let normal = text_style(palette.text);
    let bold = text_style(palette.text).add_modifier(Modifier::BOLD);
    let code_style = Style::default()
        .fg(crate::layouts::rgb_to_color(palette.accent))
        .bg(crate::layouts::rgb_to_color(palette.element_background));

    while i < bytes.len() {
        if bytes.get(i) == Some(&b'`') {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end] != b'`' {
                end += 1;
            }
            if end <= bytes.len() {
                let s = std::str::from_utf8(&bytes[start..end]).unwrap_or("");
                spans.push(Span::styled(s.to_string(), code_style));
                i = if end < bytes.len() { end + 1 } else { bytes.len() };
                continue;
            }
        }
        if i + 2 <= bytes.len() && bytes[i..i + 2] == [b'*', b'*'] {
            let start = i + 2;
            let mut end = start;
            while end + 2 <= bytes.len() && bytes[end..end + 2] != [b'*', b'*'] {
                end += 1;
            }
            if end + 2 <= bytes.len() {
                let s = std::str::from_utf8(&bytes[start..end]).unwrap_or("");
                spans.push(Span::styled(s.to_string(), bold));
                i = end + 2;
                continue;
            }
            // Unclosed bold: treat ** and rest as normal text
            spans.push(Span::styled(line[i..].to_string(), normal));
            break;
        }
        let mut next = i;
        while next < bytes.len() {
            if bytes[next] == b'`' {
                break;
            }
            if next + 2 <= bytes.len() && bytes[next..next + 2] == [b'*', b'*'] {
                break;
            }
            next += 1;
        }
        let s = std::str::from_utf8(&bytes[i..next]).unwrap_or("");
        if !s.is_empty() {
            spans.push(Span::styled(s.to_string(), normal));
        }
        i = next;
    }
    if spans.is_empty() {
        spans.push(Span::styled(line.to_string(), normal));
    }
    spans
}

pub fn has_inline_markdown(line: &str) -> bool {
    line.contains('`') || line.contains("**")
}

// --- Block parsing ---

/// Block-level markdown element.
#[derive(Debug, Clone)]
pub enum Block {
    Paragraph(String),
    Header(String),
    ListItem(String),
    CodeBlock {
        lang: Option<String>,
        code: String,
    },
    HorizontalRule,
}

/// Parse full message text into blocks (code blocks, headers, list items, rules, paragraphs).
pub fn parse_blocks(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = text.split('\n').collect();
    let mut i = 0;
    let mut paragraph_acc: Vec<&str> = Vec::new();

    let flush_paragraph = |acc: &mut Vec<&str>, out: &mut Vec<Block>| {
        if !acc.is_empty() {
            let s = acc.join("\n").trim().to_string();
            if !s.is_empty() {
                out.push(Block::Paragraph(s));
            }
            acc.clear();
        }
    };

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if let Some(stripped) = trimmed.strip_prefix("```") {
            flush_paragraph(&mut paragraph_acc, &mut blocks);
            let lang = stripped.trim().to_string();
            let lang = if lang.is_empty() { None } else { Some(lang) };
            let mut code = String::new();
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                if !code.is_empty() {
                    code.push('\n');
                }
                code.push_str(lines[i]);
                i += 1;
            }
            if i < lines.len() {
                i += 1; // skip closing ```
            }
            blocks.push(Block::CodeBlock { lang, code });
            continue;
        }

        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            flush_paragraph(&mut paragraph_acc, &mut blocks);
            blocks.push(Block::HorizontalRule);
            i += 1;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix('#') {
            flush_paragraph(&mut paragraph_acc, &mut blocks);
            let header = rest.trim_start_matches('#').trim().to_string();
            if !header.is_empty() {
                blocks.push(Block::Header(header));
            }
            i += 1;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix('-').filter(|_| {
            trimmed.starts_with("- ") || (trimmed.len() >= 2 && trimmed.as_bytes()[1].is_ascii_whitespace())
        }) {
            flush_paragraph(&mut paragraph_acc, &mut blocks);
            blocks.push(Block::ListItem(rest.trim().to_string()));
            i += 1;
            continue;
        }

        if trimmed.is_empty() {
            flush_paragraph(&mut paragraph_acc, &mut blocks);
            i += 1;
            continue;
        }

        paragraph_acc.push(line);
        i += 1;
    }
    flush_paragraph(&mut paragraph_acc, &mut blocks);
    blocks
}

/// True if text contains block-level markdown we parse.
pub fn has_block_markdown(text: &str) -> bool {
    text.contains("```") || text.trim_start().starts_with('#') || text.contains("\n---\n")
        || text.lines().any(|l| {
            let t = l.trim();
            t.starts_with("- ") || (t.len() >= 2 && t.starts_with('-') && t.as_bytes()[1].is_ascii_whitespace())
        })
}

// --- Code block syntax highlighting (T18) ---

fn keywords_for_lang(lang: &str) -> &'static [&'static str] {
    match lang.to_lowercase().as_str() {
        "rust" => &["fn", "let", "mut", "impl", "pub", "use", "mod", "struct", "enum", "if", "else", "match", "for", "in", "while", "return", "async", "await", "self", "Self", "true", "false"],
        "python" => &["def", "class", "if", "else", "elif", "for", "in", "while", "return", "import", "from", "True", "False", "None", "and", "or", "not", "with", "async", "await"],
        "javascript" | "js" => &["function", "const", "let", "var", "return", "if", "else", "for", "while", "async", "await", "true", "false", "null", "undefined", "class", "extends", "import", "export"],
        "typescript" | "ts" => &["function", "const", "let", "var", "return", "if", "else", "for", "while", "async", "await", "true", "false", "null", "undefined", "class", "extends", "import", "export", "interface", "type", "enum"],
        _ => &[],
    }
}

fn highlight_code_line(line: &str, lang: &str, palette: &LocusPalette) -> Vec<Span<'static>> {
    let keywords = keywords_for_lang(lang);
    let accent = text_style(palette.accent);
    let success = text_style(palette.success);
    let muted = text_muted_style(palette.text_muted);
    let warning = text_style(palette.warning);
    let normal = text_style(palette.editor_foreground);

    let mut spans = Vec::new();
    let mut i = 0;
    let bytes = line.as_bytes();

    while i < bytes.len() {
        // Line comment // or #
        if (i + 2 <= bytes.len() && &bytes[i..i + 2] == b"//")
            || (bytes[i] == b'#' && (i == 0 || bytes.get(i.wrapping_sub(1)) == Some(&b' ')))
        {
            spans.push(Span::styled(line[i..].to_string(), muted));
            break;
        }
        // String double-quoted
        if bytes[i] == b'"' {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
            spans.push(Span::styled(line[start..i].to_string(), success));
            continue;
        }
        // String single-quoted
        if bytes[i] == b'\'' {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != b'\'' {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
            spans.push(Span::styled(line[start..i].to_string(), success));
            continue;
        }
        // Word (keyword or number or identifier)
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &line[start..i];
            let is_keyword = keywords.contains(&word);
            let style = if is_keyword { accent } else { normal };
            spans.push(Span::styled(word.to_string(), style));
            continue;
        }
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            spans.push(Span::styled(line[start..i].to_string(), warning));
            continue;
        }
        spans.push(Span::styled(line[i..i + 1].to_string(), normal));
        i += 1;
    }
    if spans.is_empty() {
        spans.push(Span::styled(line.to_string(), normal));
    }
    spans
}

// --- Block rendering to lines ---

const BORDER_H: char = '─';

/// Render blocks to lines. Indent length is used for wrap width and left padding.
/// If first_line_prefix is Some, those spans are inserted after border_span on the first line (e.g. indicator + timestamp).
pub fn render_blocks_to_lines(
    blocks: &[Block],
    palette: &LocusPalette,
    width: usize,
    indent_len: usize,
    border_span: &Span<'static>,
    first_line_prefix: Option<Vec<Span<'static>>>,
) -> Vec<Line<'static>> {
    let wrap_width = width.saturating_sub(indent_len).max(1);
    let mut lines = Vec::new();
    let normal = text_style(palette.text);
    let muted = text_muted_style(palette.text_muted);
    let header_style = text_style(palette.text).add_modifier(Modifier::BOLD);
    let mut is_first = true;

    for block in blocks {
        match block {
            Block::Paragraph(s) => {
                let wrapped = wrap_lines(s, wrap_width);
                for seg in &wrapped {
                    let mut spans = vec![border_span.clone()];
                    if is_first {
                        if let Some(ref prefix) = first_line_prefix {
                            spans.extend(prefix.clone());
                        }
                        is_first = false;
                    }
                    spans.push(Span::raw(LEFT_PADDING));
                    if has_inline_markdown(seg) {
                        spans.extend(parse_inline_markdown(seg, palette));
                    } else {
                        spans.push(Span::styled(seg.clone(), normal));
                    }
                    lines.push(Line::from(spans));
                }
            }
            Block::Header(s) => {
                let mut spans = vec![border_span.clone()];
                if is_first {
                    if let Some(ref prefix) = first_line_prefix {
                        spans.extend(prefix.clone());
                    }
                    is_first = false;
                }
                spans.push(Span::raw(LEFT_PADDING));
                spans.push(Span::styled(s.clone(), header_style));
                lines.push(Line::from(spans));
            }
            Block::ListItem(s) => {
                let wrapped = wrap_lines(s, wrap_width.saturating_sub(2));
                for seg in &wrapped {
                    let mut sp = vec![border_span.clone()];
                    if is_first {
                        if let Some(ref prefix) = first_line_prefix {
                            sp.extend(prefix.clone());
                        }
                        is_first = false;
                    }
                    sp.push(Span::raw(LEFT_PADDING));
                    sp.push(Span::styled("• ", muted));
                    if has_inline_markdown(seg) {
                        sp.extend(parse_inline_markdown(seg, palette));
                    } else {
                        sp.push(Span::styled(seg.clone(), normal));
                    }
                    lines.push(Line::from(sp));
                }
            }
            Block::CodeBlock { lang, code } => {
                let lang_str = lang.as_deref().unwrap_or("").to_string();
                let code_lines: Vec<String> = code.lines().map(|s| s.to_string()).collect();
                let num_w = (code_lines.len().max(1)).to_string().len();
                for (i, code_line) in code_lines.iter().enumerate() {
                    let mut spans = vec![border_span.clone()];
                    if is_first {
                        if let Some(ref prefix) = first_line_prefix {
                            spans.extend(prefix.clone());
                        }
                        is_first = false;
                    }
                    let line_num = (i + 1).to_string();
                    let pad = " ".repeat(num_w.saturating_sub(line_num.len()));
                    spans.push(Span::raw(LEFT_PADDING));
                    spans.push(Span::styled(
                        format!("{}{} │ ", pad, line_num),
                        text_muted_style(palette.editor_line_number),
                    ));
                    spans.extend(highlight_code_line(code_line.as_str(), &lang_str, palette));
                    lines.push(Line::from(spans));
                }
            }
            Block::HorizontalRule => {
                let mut spans = vec![border_span.clone()];
                if is_first {
                    if let Some(ref prefix) = first_line_prefix {
                        spans.extend(prefix.clone());
                    }
                    is_first = false;
                }
                let rule: String = (0..wrap_width).map(|_| BORDER_H).collect();
                spans.push(Span::raw(LEFT_PADDING));
                spans.push(Span::styled(rule, muted));
                lines.push(Line::from(spans));
            }
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bold_parsed() {
        let palette = LocusPalette::locus_dark();
        let spans = parse_inline_markdown("hello **world** ok", &palette);
        assert!(spans.len() >= 2);
    }

    #[test]
    fn code_parsed() {
        let palette = LocusPalette::locus_dark();
        let spans = parse_inline_markdown("use `Option` here", &palette);
        assert!(spans.len() >= 2);
    }

    #[test]
    fn parse_blocks_code_fence() {
        let blocks = parse_blocks("hello\n```rust\nfn x() {}\n```\nworld");
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[0], Block::Paragraph(s) if s == "hello"));
        assert!(matches!(&blocks[1], Block::CodeBlock { lang: Some(l), .. } if l == "rust"));
        assert!(matches!(&blocks[2], Block::Paragraph(s) if s == "world"));
    }

    #[test]
    fn parse_blocks_header() {
        let blocks = parse_blocks("# Title\nbody");
        assert!(matches!(&blocks[0], Block::Header(s) if s == "Title"));
        assert!(matches!(&blocks[1], Block::Paragraph(s) if s == "body"));
    }

    #[test]
    fn parse_blocks_horizontal_rule() {
        let blocks = parse_blocks("above\n---\nbelow");
        assert!(blocks.iter().any(|b| matches!(b, Block::HorizontalRule)));
    }

    #[test]
    fn parse_blocks_list_items() {
        let blocks = parse_blocks("- one\n- two\n- three");
        let list_count = blocks.iter().filter(|b| matches!(b, Block::ListItem(_))).count();
        assert_eq!(list_count, 3);
    }

    #[test]
    fn parse_blocks_empty_code_block() {
        let blocks = parse_blocks("```\n```");
        assert!(matches!(&blocks[0], Block::CodeBlock { code, .. } if code.is_empty()));
    }

    #[test]
    fn parse_blocks_unclosed_code_block() {
        let blocks = parse_blocks("```rust\nfn main() {}");
        assert!(matches!(&blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn inline_markdown_no_markers() {
        let palette = LocusPalette::locus_dark();
        let spans = parse_inline_markdown("plain text here", &palette);
        assert_eq!(spans.len(), 1);
    }

    #[test]
    fn inline_markdown_unclosed_backtick() {
        let palette = LocusPalette::locus_dark();
        let spans = parse_inline_markdown("use `Option", &palette);
        // Should not panic, should render something
        assert!(!spans.is_empty());
    }

    #[test]
    fn inline_markdown_unclosed_bold() {
        let palette = LocusPalette::locus_dark();
        let spans = parse_inline_markdown("this is **bold", &palette);
        assert!(!spans.is_empty());
    }

    #[test]
    fn has_block_markdown_false_for_plain() {
        assert!(!has_block_markdown("just plain text"));
    }

    #[test]
    fn has_block_markdown_true_for_header() {
        assert!(has_block_markdown("# Title"));
    }

    #[test]
    fn highlight_code_line_rust_keyword() {
        let palette = LocusPalette::locus_dark();
        let spans = highlight_code_line("fn main() {}", "rust", &palette);
        assert!(spans.len() > 1); // should split into keyword + rest
    }

    #[test]
    fn highlight_code_line_string() {
        let palette = LocusPalette::locus_dark();
        let spans = highlight_code_line("let s = \"hello\";", "rust", &palette);
        assert!(spans.len() > 1);
    }

    #[test]
    fn highlight_code_line_comment() {
        let palette = LocusPalette::locus_dark();
        let spans = highlight_code_line("// comment", "rust", &palette);
        assert!(!spans.is_empty());
    }

    #[test]
    fn highlight_code_line_unknown_lang() {
        let palette = LocusPalette::locus_dark();
        let spans = highlight_code_line("some code", "brainfuck", &palette);
        assert!(!spans.is_empty()); // should still render without panic
    }
}
