use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::detect_term::AdaptiveColors;

#[derive(Debug, Clone, Copy)]
pub enum PopupMode {
    Navigation,
}

impl PopupMode {
    pub fn title(&self) -> &str {
        match self {
            PopupMode::Navigation => " Keyboard Shortcuts ",
        }
    }
}

#[derive(Debug, Clone)]
struct Shortcut {
    key: &'static str,
    description: &'static str,
    category: ShortcutCategory,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ShortcutCategory {
    Navigation,
    ModeSwitching,
    General,
}

impl ShortcutCategory {
    fn title(&self) -> &str {
        match self {
            ShortcutCategory::Navigation => " Navigation ",
            ShortcutCategory::ModeSwitching => " Mode Switching ",
            ShortcutCategory::General => " General ",
        }
    }

    fn color(&self) -> ratatui::style::Color {
        match self {
            ShortcutCategory::Navigation => AdaptiveColors::cyan(),
            ShortcutCategory::ModeSwitching => AdaptiveColors::orange(),
            ShortcutCategory::General => AdaptiveColors::yellow(),
        }
    }
}

const ALL_SHORTCUTS: &[Shortcut] = &[
    // Navigation
    Shortcut { key: "1", description: "Task Board", category: ShortcutCategory::Navigation },
    Shortcut { key: "2", description: "Plan View", category: ShortcutCategory::Navigation },
    Shortcut { key: "3", description: "Agents View", category: ShortcutCategory::Navigation },
    Shortcut { key: "4", description: "Diff Review", category: ShortcutCategory::Navigation },
    Shortcut { key: "5", description: "Logs View", category: ShortcutCategory::Navigation },
    Shortcut { key: "6", description: "Memory Trace", category: ShortcutCategory::Navigation },
    Shortcut { key: "j/k", description: "Scroll up/down", category: ShortcutCategory::Navigation },
    Shortcut { key: "g/G", description: "Go to top/bottom", category: ShortcutCategory::Navigation },
    Shortcut { key: "/", description: "Search", category: ShortcutCategory::Navigation },

    // Mode Switching
    Shortcut { key: "F1", description: "Rush Mode", category: ShortcutCategory::ModeSwitching },
    Shortcut { key: "F2", description: "Smart Mode", category: ShortcutCategory::ModeSwitching },
    Shortcut { key: "F3", description: "Deep Mode", category: ShortcutCategory::ModeSwitching },

    // General
    Shortcut { key: ":", description: "Command Palette", category: ShortcutCategory::General },
    Shortcut { key: "?", description: "Open Shortcuts", category: ShortcutCategory::General },
    Shortcut { key: "Enter", description: "Submit Prompt", category: ShortcutCategory::General },
    Shortcut { key: "Esc", description: "Back/Close", category: ShortcutCategory::General },
    Shortcut { key: "Ctrl+Q", description: "Quit", category: ShortcutCategory::General },
];

pub struct NavigationPopup {
    active: bool,
    mode: PopupMode,
    search_query: String,
}

impl NavigationPopup {
    pub fn new() -> Self {
        Self {
            active: false,
            mode: PopupMode::Navigation,
            search_query: String::new(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn activate(&mut self) {
        self.active = true;
        self.search_query.clear();
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.search_query.clear();
    }

    pub fn handle_input(&mut self, c: char) {
        self.search_query.push(c);
    }

    pub fn handle_backspace(&mut self) {
        self.search_query.pop();
    }

    pub fn get_search_query(&self) -> &str {
        &self.search_query
    }

    fn filter_shortcuts(&self) -> Vec<&Shortcut> {
        let query = self.search_query.to_lowercase();
        if query.is_empty() {
            return ALL_SHORTCUTS.iter().collect();
        }

        ALL_SHORTCUTS
            .iter()
            .filter(|s| {
                s.key.to_lowercase().contains(&query)
                    || s.description.to_lowercase().contains(&query)
            })
            .collect()
    }

    fn render_shortcut_line(shortcut: &Shortcut) -> Line<'_> {
        let key_color = match shortcut.category {
            ShortcutCategory::Navigation => AdaptiveColors::green(),
            ShortcutCategory::ModeSwitching => AdaptiveColors::orange(),
            ShortcutCategory::General => AdaptiveColors::yellow(),
        };

        Line::from(vec![
            Span::styled(
                format!(" {:<8} ", shortcut.key),
                Style::default().fg(key_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<20} ", shortcut.description),
                Style::default().fg(AdaptiveColors::text()),
            ),
        ])
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        if !self.active {
            return;
        }

        // Create centered popup area - wider and taller
        let popup_width = 70;
        let popup_height = 22;

        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;

        let popup_area = Rect {
            x: area.x + x,
            y: area.y + y,
            width: popup_width,
            height: popup_height,
        };

        // Create main block with border
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(AdaptiveColors::cyan()))
            .title(self.mode.title())
            .title_style(
                Style::default()
                    .fg(AdaptiveColors::yellow())
                    .add_modifier(Modifier::BOLD),
            );

        f.render_widget(block, popup_area);

        // Create inner area for content
        let inner = Block::default().borders(Borders::NONE).inner(popup_area);

        // Create layout for content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Search input area
                Constraint::Length(1),  // Separator
                Constraint::Min(1),     // Content
                Constraint::Length(1),  // Help text
            ])
            .split(inner);

        // Render search input
        let search_text = if self.search_query.is_empty() {
            " Search shortcuts... "
        } else {
            self.search_query.as_str()
        };

        let search_block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(AdaptiveColors::dark_gray()));

        f.render_widget(search_block, chunks[0]);
        let search_inner = Block::default().borders(Borders::NONE).inner(chunks[0]);

        let search_paragraph = Paragraph::new(search_text)
            .alignment(Alignment::Left)
            .style(Style::default().fg(AdaptiveColors::text()));
        f.render_widget(search_paragraph, search_inner);

        // Render separator line
        let separator = Line::from(vec![
            Span::styled("────────────────────────────────────────────────────────────", Style::default().fg(AdaptiveColors::dark_gray())),
        ]);
        let separator_paragraph = Paragraph::new(separator).alignment(Alignment::Center);
        f.render_widget(separator_paragraph, chunks[1]);

        // Render filtered shortcuts
        let filtered = self.filter_shortcuts();
        let mut content: Vec<Line> = Vec::new();

        if filtered.is_empty() {
            content.push(Line::from(vec![
                Span::styled(
                    " No shortcuts found ",
                    Style::default().fg(AdaptiveColors::dark_gray()),
                ),
            ]));
        } else {
            let mut current_category: Option<ShortcutCategory> = None;

            for shortcut in &filtered {
                // Add category header if changed
                if Some(shortcut.category) != current_category {
                    if current_category.is_some() {
                        content.push(Line::from("")); // Add spacing between categories
                    }
                    current_category = Some(shortcut.category);

                    let category = shortcut.category;
                    content.push(Line::from(vec![
                        Span::styled(
                            format!("{} ", category.title()),
                            Style::default()
                                .fg(category.color())
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            "────────────────────────────────────────────────────",
                            Style::default().fg(AdaptiveColors::dark_gray()),
                        ),
                    ]));
                    content.push(Line::from(""));
                }

                content.push(Self::render_shortcut_line(shortcut));
            }
        }

        let content_paragraph = Paragraph::new(content).alignment(Alignment::Left);
        f.render_widget(content_paragraph, chunks[2]);

        // Render help text at bottom
        let help = Line::from(vec![
            Span::styled(" Type to search  ", Style::default().fg(AdaptiveColors::dark_gray())),
            Span::styled("Esc", Style::default().fg(AdaptiveColors::yellow()).add_modifier(Modifier::BOLD)),
            Span::styled(" to close  ", Style::default().fg(AdaptiveColors::dark_gray())),
        ]);

        let help_paragraph = Paragraph::new(help).alignment(Alignment::Center);
        f.render_widget(help_paragraph, chunks[3]);
    }
}
