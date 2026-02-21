# Glow Rust - Markdown Rendering

This document covers the markdown rendering system using `pulldown-cmark` and `syntect`.

## Overview

The markdown renderer converts markdown text to styled terminal output using ANSI escape codes. The key challenges are:

1. Parsing markdown efficiently
2. Applying consistent styling
3. Syntax highlighting for code blocks
4. Handling word wrapping and line numbers
5. Supporting both light and dark terminal themes

## Renderer Architecture

```
Markdown Text
     │
     ▼
┌─────────────────┐
│ pulldown-cmark  │  Parse markdown to events
│    Parser       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Event Handler   │  Process events, build styled text
│                 │
└────────┬────────┘
         │
    ┌────┴────┐
    ▼         ▼
┌───────┐  ┌───────────┐
│ Text  │  │  Code     │
│Style  │  │ Highlight │
│ Apply │  │ (syntect) │
└───┬───┘  └─────┬─────┘
    │            │
    └─────┬──────┘
          ▼
┌─────────────────┐
│  Text<'static>  │  ratatui Text with styled lines
│                 │
└─────────────────┘
```

## Core Implementation

### Main Renderer

```rust
// src/markdown/renderer.rs

use pulldown_cmark::{Parser, Event, Tag, TagEnd, HeadingLevel, CodeBlockKind};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use std::collections::VecDeque;

/// Markdown to terminal renderer
pub struct MarkdownRenderer {
    config: RenderConfig,
    width: u16,
    theme: Theme,
    syntax_highlighter: SyntaxHighlighter,
}

/// Renderer configuration
#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub show_line_numbers: bool,
    pub preserve_newlines: bool,
    pub max_width: u16,
    pub line_number_width: usize,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            show_line_numbers: false,
            preserve_newlines: false,
            max_width: 120,
            line_number_width: 4,
        }
    }
}

/// Color theme for rendering
#[derive(Debug, Clone)]
pub struct Theme {
    // Heading styles
    pub h1: Style,
    pub h2: Style,
    pub h3: Style,
    pub h4: Style,
    pub h5: Style,
    pub h6: Style,
    
    // Text styles
    pub paragraph: Style,
    pub link: Style,
    pub inline_code: Style,
    pub block_quote: Style,
    
    // List styles
    pub list_marker: Style,
    
    // Code block styles
    pub code_block_bg: Color,
    pub line_number: Style,
}

impl Theme {
    /// Dark theme for dark terminals
    pub fn dark() -> Self {
        Self {
            h1: Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            h2: Style::default().fg(Color::Green),
            h3: Style::default().fg(Color::Cyan),
            h4: Style::default().fg(Color::Cyan),
            h5: Style::default().fg(Color::Blue),
            h6: Style::default().fg(Color::Blue),
            paragraph: Style::default(),
            link: Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED),
            inline_code: Style::default().fg(Color::Yellow).bg(Color::DarkGray),
            block_quote: Style::default().fg(Color::DarkGray),
            list_marker: Style::default().fg(Color::Green),
            code_block_bg: Color::Reset,
            line_number: Style::default().fg(Color::DarkGray),
        }
    }
    
    /// Light theme for light terminals
    pub fn light() -> Self {
        Self {
            h1: Style::default().fg(Color::Rgb(0, 100, 0)).add_modifier(Modifier::BOLD),
            h2: Style::default().fg(Color::Rgb(0, 100, 0)),
            h3: Style::default().fg(Color::Rgb(0, 128, 128)),
            h4: Style::default().fg(Color::Rgb(0, 128, 128)),
            h5: Style::default().fg(Color::Blue),
            h6: Style::default().fg(Color::Blue),
            paragraph: Style::default(),
            link: Style::default().fg(Color::Cyan).add_modifier(Modifier::UNDERLINED),
            inline_code: Style::default().fg(Color::Rgb(180, 90, 0)).bg(Color::Rgb(240, 240, 240)),
            block_quote: Style::default().fg(Color::DarkGray),
            list_marker: Style::default().fg(Color::Green),
            code_block_bg: Color::Rgb(245, 245, 245),
            line_number: Style::default().fg(Color::DarkGray),
        }
    }
}

impl MarkdownRenderer {
    pub fn new(config: RenderConfig, width: u16, dark_mode: bool) -> Self {
        let theme = if dark_mode { Theme::dark() } else { Theme::light() };
        let syntax_highlighter = SyntaxHighlighter::new(dark_mode);
        
        Self {
            config,
            width: width.min(config.max_width),
            theme,
            syntax_highlighter,
        }
    }
    
    /// Render markdown to styled Text
    pub fn render(&mut self, markdown: &str) -> Text<'static> {
        let parser = Parser::new(markdown);
        let mut builder = TextBuilder::new(&self.theme, self.width as usize);
        
        for event in parser {
            self.handle_event(&mut builder, event);
        }
        
        let mut text = builder.build();
        
        // Add line numbers if configured
        if self.config.show_line_numbers {
            text = self.add_line_numbers(text);
        }
        
        text
    }
    
    fn handle_event(&mut self, builder: &mut TextBuilder, event: Event) {
        match event {
            Event::Start(tag) => self.handle_start(builder, tag),
            Event::End(tag) => self.handle_end(builder, tag),
            Event::Text(text) => builder.add_text(&text),
            Event::Code(code) => builder.add_inline_code(&code),
            Event::Html(html) => builder.add_text(&html),
            Event::SoftBreak => builder.add_soft_break(),
            Event::HardBreak => builder.add_hard_break(),
            Event::Rule => builder.add_rule(),
            Event::FootnoteReference(_) => {} // Not supported
            Event::TaskListMarker(_) => {}    // Not supported in text
        }
    }
    
    fn handle_start(&mut self, builder: &mut TextBuilder, tag: Tag) {
        match tag {
            Tag::Paragraph => builder.start_paragraph(),
            Tag::Heading { level, .. } => builder.start_heading(level),
            Tag::BlockQuote => builder.start_block_quote(),
            Tag::CodeBlock(kind) => builder.start_code_block(kind),
            Tag::List(_) => builder.start_list(),
            Tag::Item => builder.start_list_item(),
            Tag::Emphasis => builder.start_emphasis(),
            Tag::Strong => builder.start_strong(),
            Tag::Strikethrough => builder.start_strikethrough(),
            Tag::Link { dest_url, .. } => builder.start_link(&dest_url),
            Tag::Image { .. } => {} // Images not rendered
            Tag::Table(_) => {}     // Tables as text
            Tag::TableHead => {}
            Tag::TableRow => {}
            Tag::TableCell => {}
            Tag::HtmlBlock => {}
            Tag::MetadataBlock(_) => {}
        }
    }
    
    fn handle_end(&mut self, builder: &mut TextBuilder, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => builder.end_paragraph(),
            TagEnd::Heading(_) => builder.end_heading(),
            TagEnd::BlockQuote => builder.end_block_quote(),
            TagEnd::CodeBlock => builder.end_code_block(&mut self.syntax_highlighter),
            TagEnd::List(_) => builder.end_list(),
            TagEnd::Item => builder.end_list_item(),
            TagEnd::Emphasis => builder.end_emphasis(),
            TagEnd::Strong => builder.end_strong(),
            TagEnd::Strikethrough => builder.end_strikethrough(),
            TagEnd::Link => builder.end_link(),
            TagEnd::Image => {}
            TagEnd::Table => {}
            TagEnd::TableHead => {}
            TagEnd::TableRow => {}
            TagEnd::TableCell => {}
            TagEnd::HtmlBlock => {}
            TagEnd::MetadataBlock(_) => {}
        }
    }
    
    fn add_line_numbers(&self, text: Text) -> Text<'static> {
        let numbered_lines: Vec<Line> = text
            .lines
            .into_iter()
            .enumerate()
            .map(|(i, line)| {
                let line_num = Span::styled(
                    format!("{:width$}", i + 1, width = self.config.line_number_width),
                    self.theme.line_number,
                );
                let mut spans = vec![line_num];
                spans.extend(line.spans);
                Line::from(spans)
            })
            .collect();
        
        Text::from(numbered_lines)
    }
}
```

### Text Builder

```rust
// src/markdown/builder.rs

use pulldown_cmark::{HeadingLevel, CodeBlockKind};

/// Builds styled text from markdown events
struct TextBuilder<'a> {
    theme: &'a Theme,
    max_width: usize,
    lines: Vec<Line<'static>>,
    current_line: Vec<Span<'static>>,
    current_style: Style,
    
    // Stack of active styles (for nested formatting)
    style_stack: Vec<Style>,
    
    // State for current block
    in_paragraph: bool,
    in_heading: bool,
    in_block_quote: bool,
    in_list: bool,
    list_depth: usize,
    in_code_block: bool,
    code_block_lang: String,
    code_block_content: String,
    code_block_lines: Vec<String>,
}

impl<'a> TextBuilder<'a> {
    fn new(theme: &'a Theme, max_width: usize) -> Self {
        Self {
            theme,
            max_width,
            lines: Vec::new(),
            current_line: Vec::new(),
            current_style: Style::default(),
            style_stack: Vec::new(),
            in_paragraph: false,
            in_heading: false,
            in_block_quote: false,
            in_list: false,
            list_depth: 0,
            in_code_block: false,
            code_block_lang: String::new(),
            code_block_content: String::new(),
            code_block_lines: Vec::new(),
        }
    }
    
    // ... implementation methods for each event type
    
    fn add_text(&mut self, text: &str) {
        if self.in_code_block {
            self.code_block_content.push_str(text);
        } else {
            // Word wrap and add text
            let wrapped = self.wrap_text(text);
            for (i, segment) in wrapped.into_iter().enumerate() {
                if i > 0 {
                    self.flush_line();
                }
                self.current_line.push(Span::styled(segment, self.current_style));
            }
        }
    }
    
    fn add_inline_code(&mut self, code: &str) {
        let style = self.theme.inline_code;
        self.current_line.push(Span::styled(code.to_string(), style));
    }
    
    fn start_heading(&mut self, level: HeadingLevel) {
        self.flush_line();
        self.in_heading = true;
        let (prefix, style) = match level {
            HeadingLevel::H1 => ("# ", self.theme.h1),
            HeadingLevel::H2 => ("## ", self.theme.h2),
            HeadingLevel::H3 => ("### ", self.theme.h3),
            HeadingLevel::H4 => ("#### ", self.theme.h4),
            HeadingLevel::H5 => ("##### ", self.theme.h5),
            HeadingLevel::H6 => ("###### ", self.theme.h6),
        };
        self.current_style = style;
        self.current_line.push(Span::styled(prefix.to_string(), style));
    }
    
    fn end_heading(&mut self) {
        self.flush_line();
        self.lines.push(Line::default()); // Blank line after heading
        self.in_heading = false;
        self.current_style = Style::default();
    }
    
    fn start_paragraph(&mut self) {
        self.in_paragraph = true;
    }
    
    fn end_paragraph(&mut self) {
        self.flush_line();
        self.lines.push(Line::default()); // Blank line between paragraphs
        self.in_paragraph = false;
    }
    
    fn start_code_block(&mut self, kind: CodeBlockKind) {
        self.flush_line();
        self.in_code_block = true;
        self.code_block_content.clear();
        self.code_block_lang = match kind {
            CodeBlockKind::Fenced(lang) => lang.to_string(),
            CodeBlockKind::Indented => String::new(),
        };
    }
    
    fn end_code_block(&mut self, highlighter: &mut SyntaxHighlighter) {
        // Apply syntax highlighting
        let highlighted = highlighter.highlight(
            &self.code_block_content,
            &self.code_block_lang,
        );
        
        for line in highlighted.lines() {
            self.lines.push(Line::from(line.clone()));
        }
        
        self.lines.push(Line::default()); // Blank line after code
        self.in_code_block = false;
    }
    
    fn start_list(&mut self) {
        self.list_depth += 1;
        if !self.in_list {
            self.flush_line();
        }
        self.in_list = true;
    }
    
    fn end_list(&mut self) {
        self.list_depth -= 1;
        if self.list_depth == 0 {
            self.in_list = false;
            self.lines.push(Line::default());
        }
    }
    
    fn start_list_item(&mut self) {
        let indent = "  ".repeat(self.list_depth.saturating_sub(1));
        let marker = "• ";
        self.current_line.push(Span::raw(indent));
        self.current_line.push(Span::styled(marker, self.theme.list_marker));
    }
    
    fn start_emphasis(&mut self) {
        self.style_stack.push(self.current_style);
        self.current_style = self.current_style.add_modifier(Modifier::ITALIC);
    }
    
    fn end_emphasis(&mut self) {
        if let Some(style) = self.style_stack.pop() {
            self.current_style = style;
        }
    }
    
    fn start_strong(&mut self) {
        self.style_stack.push(self.current_style);
        self.current_style = self.current_style.add_modifier(Modifier::BOLD);
    }
    
    fn end_strong(&mut self) {
        if let Some(style) = self.style_stack.pop() {
            self.current_style = style;
        }
    }
    
    fn start_link(&mut self, url: &str) {
        // Store URL for later display
        self.current_line.push(Span::styled(
            "[",
            self.theme.link,
        ));
    }
    
    fn end_link(&mut self) {
        self.current_line.push(Span::styled(
            "]",
            self.theme.link,
        ));
    }
    
    fn add_soft_break(&mut self) {
        // Soft break = space (word continue)
        self.current_line.push(Span::raw(" "));
    }
    
    fn add_hard_break(&mut self) {
        // Hard break = new line
        self.flush_line();
    }
    
    fn add_rule(&mut self) {
        self.flush_line();
        let rule = "─".repeat(self.max_width);
        self.lines.push(Line::styled(
            rule,
            Style::default().fg(Color::DarkGray),
        ));
        self.lines.push(Line::default());
    }
    
    fn flush_line(&mut self) {
        if !self.current_line.is_empty() {
            self.lines.push(Line::from(std::mem::take(&mut self.current_line)));
        }
    }
    
    fn wrap_text(&self, text: &str) -> Vec<String> {
        use textwrap::{wrap, Options};
        
        let options = Options::new(self.max_width);
        wrap(text, options)
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }
    
    fn build(mut self) -> Text<'static> {
        self.flush_line();
        Text::from(self.lines)
    }
}
```

### Syntax Highlighting

```rust
// src/markdown/syntax.rs

use syntect::{
    highlighting::{Theme, ThemeSet},
    parsing::SyntaxSet,
    html::highlighted_html_for_string,
    easy::HighlightLines,
};

/// Syntax highlighter for code blocks
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme: &'static Theme,
}

impl SyntaxHighlighter {
    pub fn new(dark_mode: bool) -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        
        let theme = if dark_mode {
            &theme_set.themes["base16-mocha.dark"]
        } else {
            &theme_set.themes["base16-ocean.light"]
        };
        
        Self { syntax_set, theme }
    }
    
    /// Highlight code and return styled spans for each line
    pub fn highlight(&mut self, code: &str, lang: &str) -> Vec<Vec<Span<'static>>> {
        let syntax = self.syntax_set
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        
        let mut highlighter = HighlightLines::new(syntax, self.theme);
        let mut result = Vec::new();
        
        for line in code.lines() {
            let spans: Vec<Span> = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default()
                .into_iter()
                .map(|(style, text)| {
                    Span::styled(
                        text.to_string(),
                        Style::default()
                            .fg(convert_color(style.foreground))
                            .bg(convert_color(style.background)),
                    )
                })
                .collect();
            
            result.push(spans);
        }
        
        result
    }
}

/// Convert syntect color to ratatui color
fn convert_color(color: syntect::highlighting::Color) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}
```

## Usage

```rust
// In your application
fn render_document(content: &str, config: &Config, width: u16) -> Text<'static> {
    let render_config = RenderConfig {
        show_line_numbers: config.show_line_numbers,
        preserve_newlines: config.preserve_newlines,
        max_width: config.glamour_max_width,
        line_number_width: 4,
    };
    
    let mut renderer = MarkdownRenderer::new(
        render_config,
        width,
        is_dark_terminal(),
    );
    
    renderer.render(content)
}

fn is_dark_terminal() -> bool {
    // Check terminal color scheme
    std::env::var("COLOR_SCHEME")
        .map(|s| s == "dark")
        .unwrap_or(true) // Default to dark
}
```

## Frontmatter Handling

```rust
// src/markdown/frontmatter.rs

use regex::Regex;

/// YAML frontmatter pattern
static FRONTMATTER_PATTERN: &str = r"(?m)^---\r?\n(\s*\r?\n)?";

/// Remove YAML frontmatter from markdown content
pub fn remove_frontmatter(content: &str) -> String {
    let re = Regex::new(FRONTMATTER_PATTERN).unwrap();
    
    let matches: Vec<_> = re.find_iter(content).collect();
    
    if matches.len() >= 2 {
        // Content after second ---
        content[matches[1].end()..].to_string()
    } else {
        content.to_string()
    }
}

/// Extract frontmatter as key-value pairs
pub fn extract_frontmatter(content: &str) -> Option<Vec<(String, String)>> {
    let re = Regex::new(FRONTMATTER_PATTERN).unwrap();
    
    let matches: Vec<_> = re.find_iter(content).collect();
    
    if matches.len() >= 2 {
        let frontmatter = &content[matches[0].end()..matches[1].start()];
        
        // Simple YAML parsing (key: value)
        let pairs = frontmatter
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    Some((
                        parts[0].trim().to_string(),
                        parts[1].trim().to_string(),
                    ))
                } else {
                    None
                }
            })
            .collect();
        
        Some(pairs)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_remove_frontmatter() {
        let input = r#"---
title: Test
date: 2024-01-01
---

# Content

This is the content."#;
        
        let result = remove_frontmatter(input);
        assert!(result.starts_with("# Content"));
    }
    
    #[test]
    fn test_no_frontmatter() {
        let input = "# Just content\n\nNo frontmatter here.";
        let result = remove_frontmatter(input);
        assert_eq!(result, input);
    }
}
```

## Performance Tips

1. **Cache rendered content**: Store the rendered `Text` in the pager model
2. **Lazy rendering**: Only render when content changes
3. **Limit width**: Cap width at 120 characters for readability
4. **Reuse SyntaxHighlighter**: Keep the highlighter instance to reuse loaded syntaxes
