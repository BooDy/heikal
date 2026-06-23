use crate::app::{App, AppView, FeedUiItem};
use crate::shaping::{shape_preformatted_text, shape_text};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(f.size().height.saturating_sub(1)),
            Constraint::Length(1),
        ])
        .split(f.size());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(chunks[0]);

    render_feeds(f, app, main_chunks[0]);

    if app.feeds.is_empty() {
        render_welcome(f, app, main_chunks[1]);
    } else {
        match app.current_view {
            AppView::Feeds => {
                render_landing_page(f, app, main_chunks[1]);
            }
            AppView::Articles => {
                render_articles(f, app, main_chunks[1]);
            }
            AppView::Reader => {
                render_reader(f, app, main_chunks[1]);
            }
        }
    }

    render_status_bar(f, app, chunks[1]);

    if app.show_help {
        render_help_modal(f, app, f.size());
    } else if app.show_add_feed {
        render_add_feed_modal(f, app, f.size());
    } else if app.show_edit_feed {
        render_edit_feed_modal(f, app, f.size());
    }
}

fn render_feeds(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .ui_items
        .iter()
        .enumerate()
        .map(|(i, item)| match item {
            FeedUiItem::Category(name) => {
                let icon = if app.collapsed_categories.contains(name) {
                    "▶"
                } else {
                    "▼"
                };
                let style = if i == app.selected_ui_idx && app.current_view == AppView::Feeds {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::LightMagenta)
                        .add_modifier(Modifier::BOLD)
                };
                ListItem::new(format!("{} {}", icon, name)).style(style)
            }
            FeedUiItem::Feed(feed) => {
                let style = if i == app.selected_ui_idx && app.current_view == AppView::Feeds {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else if i == app.selected_ui_idx {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                };
                ListItem::new(format!("  {}", feed.title)).style(style)
            }
        })
        .collect();

    let is_active = app.current_view == AppView::Feeds && !app.show_help;
    let border_color = if is_active {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let feeds_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(" Feeds "),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    f.render_widget(feeds_list, area);
}

fn render_articles(f: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(vec![
        Cell::from(" Title"),
        Cell::from("Date"),
        Cell::from("Read/Unread"),
    ])
    .style(
        Style::default()
            .fg(Color::LightMagenta)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = app
        .articles
        .iter()
        .enumerate()
        .map(|(i, article)| {
            let is_selected = i == app.selected_article_idx;
            let is_active = app.current_view == AppView::Articles && !app.show_help;

            let mut style = if is_selected && is_active {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            if !article.is_read {
                style = style.add_modifier(Modifier::BOLD);
            }

            let title_text = if is_selected {
                format!("> {}", article.title)
            } else if !article.is_read {
                format!("* {}", article.title)
            } else {
                format!("  {}", article.title)
            };

            let date_str = article
                .published
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "N/A".to_string());

            let read_status = if article.is_read { "Read" } else { "Unread" };

            Row::new(vec![
                Cell::from(title_text),
                Cell::from(date_str),
                Cell::from(read_status),
            ])
            .style(style)
        })
        .collect();

    let is_active = app.current_view == AppView::Articles && !app.show_help;
    let border_color = if is_active {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let widths = [
        Constraint::Percentage(70),
        Constraint::Length(17),
        Constraint::Length(12),
    ];

    let articles_table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(" Articles "),
    );

    f.render_widget(articles_table, area);
}

fn is_html(text: &str) -> bool {
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            if let Some(&next_c) = chars.peek() {
                if next_c.is_ascii_alphabetic() || next_c == '/' || next_c == '!' || next_c == '?' {
                    return true;
                }
            }
        }
    }
    false
}

fn render_reader(f: &mut Frame, app: &mut App, area: Rect) {
    if let Some(article) = app.articles.get(app.selected_article_idx) {
        let width = area.width.saturating_sub(2) as usize;

        let shaped_lines = if let Some((cached_width, cached_lines)) = &app.shaped_article_cache {
            if *cached_width == width {
                cached_lines.clone()
            } else {
                let content = article
                    .content
                    .as_ref()
                    .or(article.description.as_ref())
                    .map(|s| s.as_str())
                    .unwrap_or("No content");
                let lines = if is_html(content) {
                    let plain_text = html2text::config::plain()
                        .allow_width_overflow()
                        .string_from_read(content.as_bytes(), width)
                        .unwrap_or_else(|_| content.to_string());
                    shape_preformatted_text(app.shaper.as_ref(), &plain_text, width)
                } else {
                    shape_text(app.shaper.as_ref(), content, width)
                };
                app.shaped_article_cache = Some((width, lines.clone()));
                lines
            }
        } else {
            let content = article
                .content
                .as_ref()
                .or(article.description.as_ref())
                .map(|s| s.as_str())
                .unwrap_or("No content");
            let lines = if is_html(content) {
                let plain_text = html2text::config::plain()
                    .allow_width_overflow()
                    .string_from_read(content.as_bytes(), width)
                    .unwrap_or_else(|_| content.to_string());
                shape_preformatted_text(app.shaper.as_ref(), &plain_text, width)
            } else {
                shape_text(app.shaper.as_ref(), content, width)
            };
            app.shaped_article_cache = Some((width, lines.clone()));
            lines
        };

        let text: Vec<Line> = shaped_lines.into_iter().map(Line::from).collect();

        let is_active = app.current_view == AppView::Reader && !app.show_help;
        let border_color = if is_active {
            Color::Cyan
        } else {
            Color::DarkGray
        };

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .title(format!(" {} ", article.title)),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, area);
    }
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status_style = Style::default().bg(Color::Blue).fg(Color::White);

    let keys_text = if app.show_help {
        "Press any key to close Help"
    } else if app.show_add_feed {
        "Type URL | [Enter] Confirm | [Esc] Cancel"
    } else if app.show_edit_feed {
        "Type value | [Enter] Next/Confirm | [Esc] Cancel"
    } else {
        "[j/k] Navigate | [Enter] Select | [a] Add Feed | [e] Edit Feed | [r] Sync | [?] Help | [q] Quit"
    };

    let clean_status = app
        .status_message
        .replace('\n', " ")
        .replace('\r', "")
        .replace('\t', " ");

    let status_text = format!(" 📡 Status: {}  |  {}", clean_status, keys_text);
    let status = Paragraph::new(status_text).style(status_style);
    f.render_widget(status, area);
}

fn render_welcome(f: &mut Frame, _app: &App, area: Rect) {
    let welcome_text = vec![
        Line::from(""),
        Line::from(vec![ratatui::text::Span::styled(
            "   Welcome to Heikal!   ",
            Style::default()
                .fg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("   ────────────────────────"),
        Line::from(""),
        Line::from("   It looks like you haven't subscribed to any RSS feeds yet."),
        Line::from(""),
        Line::from(vec![ratatui::text::Span::styled(
            "   [ How to subscribe to RSS Feeds ]",
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from("   To subscribe, press [a] in this app, or run:"),
        Line::from(vec![ratatui::text::Span::styled(
            "     heikal add <feed_url>",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from("   For example:"),
        Line::from(vec![ratatui::text::Span::styled(
            "     heikal add https://news.ycombinator.com/rss",
            Style::default().fg(Color::Gray),
        )]),
        Line::from(""),
        Line::from("   Once added, press [r] in this app to sync and fetch articles!"),
        Line::from(""),
        Line::from("   ──────────────────────────────────────────────────────────"),
        Line::from("   Press [?] or [m] at any time to open the full Help Menu."),
        Line::from(""),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Getting Started ")
        .title_alignment(ratatui::layout::Alignment::Left);

    let paragraph = Paragraph::new(welcome_text)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn render_landing_page(f: &mut Frame, app: &App, area: Rect) {
    let total_feeds = app.feeds.len();
    let categories: std::collections::HashSet<&str> =
        app.feeds.iter().map(|f| f.category.as_str()).collect();
    let total_categories = categories.len();

    let welcome_text = vec![
        Line::from(""),
        Line::from(vec![ratatui::text::Span::styled(
            "    ⚡ Heikal Dashboard ⚡    ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("    ─────────────────────────────────────"),
        Line::from(""),
        Line::from(vec![ratatui::text::Span::styled(
            "    [ App Statistics ]",
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(format!("      • Total Feeds      : {}", total_feeds)),
        Line::from(format!("      • Categories       : {}", total_categories)),
        Line::from(""),
        Line::from(vec![ratatui::text::Span::styled(
            "    [ Quick Start Guide ]",
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("      1. Navigate through your feeds using [j / k] or [Up / Down]."),
        Line::from("      2. Press [Enter] or [l / Right] to open a feed and see its articles."),
        Line::from("      3. Press [a] to add a new feed, or [e] to edit the selected feed."),
        Line::from("      4. Press [r] to force refresh/sync your feeds from the SQLite database."),
        Line::from(""),
        Line::from(vec![ratatui::text::Span::styled(
            "    [ Keyboard Shortcuts Reference ]",
            Style::default()
                .fg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("      • [j/k] or [Up/Down]  : Navigate Feed Sidebar"),
        Line::from("      • [Enter] or [l/Right]: Open selected Feed / Read article"),
        Line::from("      • [h/Left]            : Go back to feeds sidebar / exit article"),
        Line::from("      • [a]                 : Subscribe to new Feed"),
        Line::from("      • [e]                 : Edit current Feed details"),
        Line::from("      • [o]                 : Open article link in web browser"),
        Line::from("      • [?] / [m]           : Toggle full Help menu"),
        Line::from("      • [q]                 : Quit application"),
        Line::from(""),
        Line::from("    ─────────────────────────────────────────────────────────────────"),
        Line::from(vec![ratatui::text::Span::styled(
            "    Select a feed on the left and press [Enter] to begin reading!",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::ITALIC),
        )]),
        Line::from(""),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Dashboard ")
        .title_alignment(ratatui::layout::Alignment::Left);

    let paragraph = Paragraph::new(welcome_text)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn render_help_modal(f: &mut Frame, _app: &App, area: Rect) {
    let popup_area = centered_rect(70, 75, area);

    f.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(""),
        Line::from(vec![ratatui::text::Span::styled(
            "   Heikal Help & Feed Instructions   ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("   ───────────────────────────────────"),
        Line::from(""),
        Line::from(vec![ratatui::text::Span::styled(
            "   [ Managing RSS Feeds ]",
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("   To subscribe to a new feed, run this command in your terminal:"),
        Line::from(vec![ratatui::text::Span::styled(
            "     heikal add <feed_url>",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("     Example: heikal add https://news.ycombinator.com/rss"),
        Line::from(""),
        Line::from("   To unsubscribe / delete a feed:"),
        Line::from(vec![ratatui::text::Span::styled(
            "     heikal delete <feed_id>",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from("   To list all active feeds and their database IDs:"),
        Line::from(vec![ratatui::text::Span::styled(
            "     heikal list",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![ratatui::text::Span::styled(
            "   [ Navigation & Hotkeys ]",
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("     j / Down  : Scroll / Navigate down"),
        Line::from("     k / Up    : Scroll / Navigate up"),
        Line::from("     h / Left  : Go back to previous panel / exit Reader"),
        Line::from("     l / Right : Open selected feed / article"),
        Line::from("     Enter     : Open selected feed / read article"),
        Line::from("     o         : Open article in system browser"),
        Line::from("     r         : Force background feed sync"),
        Line::from("     a         : Open 'Add Feed' dialog"),
        Line::from("     e         : Open 'Edit Feed' dialog (when selecting a feed)"),
        Line::from("     ? / m     : Toggle this Help Menu"),
        Line::from("     q / Esc   : Close this menu / Quit application"),
        Line::from(""),
        Line::from(vec![ratatui::text::Span::styled(
            "   Press any key to close this menu",
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        )]),
        Line::from(""),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Help & Info ")
        .title_alignment(ratatui::layout::Alignment::Center);

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, popup_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_add_feed_modal(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(60, 35, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Add RSS Feed ")
        .title_alignment(ratatui::layout::Alignment::Center);

    let modal_text = if app.add_feed_step == 0 {
        vec![
            Line::from(""),
            Line::from(vec![ratatui::text::Span::styled(
                "   Subscribe to a New Feed (Step 1/2)",
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("   ──────────────────────────────────"),
            Line::from(""),
            Line::from("   Enter RSS/Atom Feed URL:"),
            Line::from(vec![ratatui::text::Span::styled(
                format!("   > {}█", app.input_feed_url),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![ratatui::text::Span::styled(
                "   [Enter] Next   |   [Esc] Cancel",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(""),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(vec![ratatui::text::Span::styled(
                "   Subscribe to a New Feed (Step 2/2)",
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("   ──────────────────────────────────"),
            Line::from(""),
            Line::from(format!("   URL: {}", app.input_feed_url)),
            Line::from(""),
            Line::from("   Enter Group Category (default: Uncategorized):"),
            Line::from(vec![ratatui::text::Span::styled(
                format!("   > {}█", app.input_category),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![ratatui::text::Span::styled(
                "   [Enter] Confirm   |   [Esc] Cancel",
                Style::default().fg(Color::Gray),
            )]),
            Line::from(""),
        ]
    };

    let paragraph = Paragraph::new(modal_text)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, popup_area);
}

fn render_edit_feed_modal(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(65, 55, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Edit RSS Feed ")
        .title_alignment(ratatui::layout::Alignment::Center);

    let mut modal_text = vec![
        Line::from(""),
        Line::from(vec![ratatui::text::Span::styled(
            format!("   Edit Feed Details (Step {}/3)", app.edit_feed_step + 1),
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("   ───────────────────────────────"),
        Line::from(""),
    ];

    // Field 1: Title
    if app.edit_feed_step == 0 {
        modal_text.push(Line::from(vec![
            ratatui::text::Span::styled(
                "   Title:  ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            ratatui::text::Span::styled(
                format!("> {}█", app.edit_input_title),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    } else {
        modal_text.push(Line::from(vec![
            ratatui::text::Span::styled("   Title:  ", Style::default().fg(Color::Gray)),
            ratatui::text::Span::styled(
                app.edit_input_title.clone(),
                Style::default().fg(Color::White),
            ),
        ]));
    }
    modal_text.push(Line::from(""));

    // Field 2: URL
    if app.edit_feed_step == 1 {
        modal_text.push(Line::from(vec![
            ratatui::text::Span::styled(
                "   URL:    ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            ratatui::text::Span::styled(
                format!("> {}█", app.edit_input_url),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    } else {
        modal_text.push(Line::from(vec![
            ratatui::text::Span::styled("   URL:    ", Style::default().fg(Color::Gray)),
            ratatui::text::Span::styled(
                app.edit_input_url.clone(),
                Style::default().fg(Color::White),
            ),
        ]));
    }
    modal_text.push(Line::from(""));

    // Field 3: Category
    if app.edit_feed_step == 2 {
        modal_text.push(Line::from(vec![
            ratatui::text::Span::styled(
                "   Category: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            ratatui::text::Span::styled(
                format!("> {}█", app.edit_input_category),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    } else {
        modal_text.push(Line::from(vec![
            ratatui::text::Span::styled("   Category: ", Style::default().fg(Color::Gray)),
            ratatui::text::Span::styled(
                if app.edit_input_category.is_empty() {
                    "Uncategorized"
                } else {
                    &app.edit_input_category
                },
                Style::default().fg(Color::White),
            ),
        ]));
    }

    modal_text.push(Line::from(""));
    modal_text.push(Line::from("   ───────────────────────────────"));
    modal_text.push(Line::from(""));
    modal_text.push(Line::from(vec![ratatui::text::Span::styled(
        "   [Enter] Next/Confirm   |   [Esc] Cancel",
        Style::default().fg(Color::Gray),
    )]));
    modal_text.push(Line::from(""));

    let paragraph = Paragraph::new(modal_text)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_html() {
        assert!(is_html("<p>hello</p>"));
        assert!(is_html("<a>"));
        assert!(is_html("hello <!-- comment -->"));
        assert!(is_html("<?xml version=\"1.0\"?>"));
        assert!(!is_html("hello world"));
        assert!(!is_html("hello < world"));
        assert!(!is_html("a < b && b > c"));
    }
}
