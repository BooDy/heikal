use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use heikal::app::{App, AppView, FeedUiItem};
use heikal::db::{SqliteStorage, Storage};
use heikal::models::Article;
use heikal::sync::sync_loop;
use heikal::ui;
use std::io;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> heikal::error::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Initialize DB
    let storage = Arc::new(SqliteStorage::new("heikal.db")?);

    if args.len() >= 3 && args[1] == "add" {
        let url = &args[2];
        let category = if args.len() >= 4 {
            Some(args[3].as_str())
        } else {
            None
        };
        println!(
            "Adding feed: {} (Category: {})",
            url,
            category.unwrap_or("Uncategorized")
        );
        match storage.add_feed(url, url, None, None, category) {
            Ok(_) => println!("Successfully added feed."),
            Err(e) => println!("Error adding feed: {}", e),
        }
        return Ok(());
    }

    if args.len() >= 3 && args[1] == "delete" {
        let id_str = &args[2];
        match id_str.parse::<i64>() {
            Ok(id) => {
                println!("Deleting feed with ID: {}", id);
                match storage.delete_feed(id) {
                    Ok(_) => println!("Successfully deleted feed."),
                    Err(e) => println!("Error deleting feed: {}", e),
                }
            }
            Err(_) => println!("Invalid ID: {}", id_str),
        }
        return Ok(());
    }

    if args.len() >= 2 && args[1] == "list" {
        let feeds = storage.get_feeds()?;
        println!("{:<5} | {:<20} | {}", "ID", "Title", "URL");
        println!("{}", "-".repeat(40));
        for feed in feeds {
            println!("{:<5} | {:<20} | {}", feed.id, feed.title, feed.url);
        }
        return Ok(());
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Start background sync
    let (status_tx, mut status_rx) = tokio::sync::mpsc::unbounded_channel();
    let sync_storage = storage.clone();
    let sync_status_tx = status_tx.clone();
    tokio::spawn(async move {
        sync_loop(sync_storage, Duration::from_secs(600), sync_status_tx).await;
    });

    // Create app
    let mut app = App::new(storage)?;

    // Run loop
    let res = run_app(&mut terminal, &mut app, &mut status_rx, status_tx.clone()).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    status_rx: &mut tokio::sync::mpsc::UnboundedReceiver<String>,
    status_tx: tokio::sync::mpsc::UnboundedSender<String>,
) -> heikal::error::Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        // Check for status updates
        while let Ok(msg) = status_rx.try_recv() {
            if msg.starts_with("AI_SUMMARY_SUCCESS:") {
                app.summary_content = msg["AI_SUMMARY_SUCCESS:".len()..].to_string();
                app.summary_loading = false;
                app.status_message = String::from("Summary generated successfully");
            } else if msg.starts_with("AI_SUMMARY_ERROR:") {
                app.summary_content = format!("AI Error: {}", &msg["AI_SUMMARY_ERROR:".len()..]);
                app.summary_loading = false;
                app.status_message = String::from("AI summarization failed");
            } else {
                app.status_message = msg;
                if app.status_message == "Sync complete" {
                    app.refresh_feeds()?;
                    app.refresh_articles()?;
                }
            }
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if app.show_help {
                    app.show_help = false;
                } else if app.show_add_feed {
                    match key.code {
                        KeyCode::Enter => {
                            if app.add_feed_step == 0 {
                                let url = app.input_feed_url.trim().to_string();
                                if !url.is_empty() {
                                    app.add_feed_step = 1;
                                    app.input_category.clear();
                                } else {
                                    app.show_add_feed = false;
                                }
                            } else {
                                let url = app.input_feed_url.trim().to_string();
                                let category = app.input_category.trim().to_string();
                                let category_opt = if category.is_empty() {
                                    None
                                } else {
                                    Some(category.as_str())
                                };

                                app.status_message = format!("Adding feed: {}...", url);
                                match app.storage.add_feed(&url, &url, None, None, category_opt) {
                                    Ok(feed_id) => {
                                        app.refresh_feeds()?;
                                        app.refresh_articles()?;
                                        let sync_storage = app.storage.clone();
                                        let status_tx_clone = status_tx.clone();
                                        tokio::spawn(async move {
                                            if let Ok(feeds) = sync_storage.get_feeds() {
                                                if let Some(feed) =
                                                    feeds.iter().find(|f| f.id == feed_id)
                                                {
                                                    let _ = status_tx_clone
                                                        .send(String::from("Syncing new feed..."));
                                                     if let Err(e) = heikal::sync::sync_feed(
                                                         sync_storage.as_ref(),
                                                         feed,
                                                     )
                                                    .await
                                                    {
                                                        let _ = status_tx_clone
                                                            .send(format!("Sync error: {}", e));
                                                    } else {
                                                        let _ = status_tx_clone
                                                            .send(String::from("Sync complete"));
                                                    }
                                                }
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        app.status_message = format!("Error: {}", e);
                                    }
                                }
                                app.show_add_feed = false;
                                app.input_feed_url.clear();
                                app.input_category.clear();
                                app.add_feed_step = 0;
                            }
                        }
                        KeyCode::Esc => {
                            app.show_add_feed = false;
                            app.input_feed_url.clear();
                            app.input_category.clear();
                            app.add_feed_step = 0;
                        }
                        KeyCode::Backspace => {
                            if app.add_feed_step == 0 {
                                app.input_feed_url.pop();
                            } else {
                                app.input_category.pop();
                            }
                        }
                        KeyCode::Char(c) => {
                            if app.add_feed_step == 0 {
                                app.input_feed_url.push(c);
                            } else {
                                app.input_category.push(c);
                            }
                        }
                        _ => {}
                    }
                } else if app.show_edit_feed {
                    match key.code {
                        KeyCode::Enter => {
                            if app.edit_feed_step == 0 {
                                app.edit_feed_step = 1;
                            } else if app.edit_feed_step == 1 {
                                app.edit_feed_step = 2;
                            } else {
                                let title = app.edit_input_title.trim().to_string();
                                let url = app.edit_input_url.trim().to_string();
                                let category = app.edit_input_category.trim().to_string();

                                if !title.is_empty() && !url.is_empty() {
                                    app.status_message = format!("Updating feed: {}...", title);
                                    let cat_str = if category.is_empty() {
                                        "Uncategorized"
                                    } else {
                                        &category
                                    };
                                    match app.storage.update_feed_details(
                                        app.edit_feed_id,
                                        &title,
                                        &url,
                                        cat_str,
                                    ) {
                                        Ok(_) => {
                                            app.refresh_feeds()?;
                                            app.refresh_articles()?;
                                            app.status_message =
                                                String::from("Feed updated successfully");
                                        }
                                        Err(e) => {
                                            app.status_message = format!("Error: {}", e);
                                        }
                                    }
                                } else {
                                    app.status_message =
                                        String::from("Title and URL cannot be empty");
                                }
                                app.show_edit_feed = false;
                                app.edit_feed_step = 0;
                            }
                        }
                        KeyCode::Esc => {
                            app.show_edit_feed = false;
                            app.edit_feed_step = 0;
                        }
                        KeyCode::Backspace => {
                            if app.edit_feed_step == 0 {
                                app.edit_input_title.pop();
                            } else if app.edit_feed_step == 1 {
                                app.edit_input_url.pop();
                            } else {
                                app.edit_input_category.pop();
                            }
                        }
                        KeyCode::Char(c) => {
                            if app.edit_feed_step == 0 {
                                app.edit_input_title.push(c);
                            } else if app.edit_feed_step == 1 {
                                app.edit_input_url.push(c);
                            } else {
                                app.edit_input_category.push(c);
                            }
                        }
                        _ => {}
                    }
                } else if app.show_settings {
                    match key.code {
                        KeyCode::Enter => {
                            if app.settings_step == 0 {
                                app.settings_step = 1;
                            } else if app.settings_step == 1 {
                                app.settings_step = 2;
                            } else if app.settings_step == 2 {
                                app.settings_step = 3;
                            } else {
                                let provider = app.input_provider.trim().to_string();
                                let api_token = app.input_api_token.trim().to_string();
                                let model = app.input_model.trim().to_string();
                                let api_base = app.input_api_base.trim().to_string();

                                if !provider.is_empty() && !model.is_empty() {
                                    let _ = app.storage.set_setting("ai_provider", &provider);
                                    let _ = app.storage.set_setting("ai_api_token", &api_token);
                                    let _ = app.storage.set_setting("ai_model", &model);
                                    let _ = app.storage.set_setting("ai_api_base", &api_base);
                                    app.status_message = String::from("AI configuration saved successfully");
                                } else {
                                    app.status_message = String::from("Provider and Model cannot be empty");
                                }
                                app.show_settings = false;
                                app.settings_step = 0;
                            }
                        }
                        KeyCode::Esc => {
                            app.show_settings = false;
                            app.settings_step = 0;
                        }
                        KeyCode::Backspace => {
                            if app.settings_step == 0 {
                                app.input_provider.pop();
                            } else if app.settings_step == 1 {
                                app.input_api_token.pop();
                            } else if app.settings_step == 2 {
                                app.input_model.pop();
                            } else {
                                app.input_api_base.pop();
                            }
                        }
                        KeyCode::Char(c) => {
                            if app.settings_step == 0 {
                                app.input_provider.push(c);
                            } else if app.settings_step == 1 {
                                app.input_api_token.push(c);
                            } else if app.settings_step == 2 {
                                app.input_model.push(c);
                            } else {
                                app.input_api_base.push(c);
                            }
                        }
                        _ => {}
                    }
                } else if app.show_summary {
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.summary_scroll = app.summary_scroll.saturating_add(1);
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.summary_scroll = app.summary_scroll.saturating_sub(1);
                        }
                        _ => {
                            app.show_summary = false;
                            app.summary_scroll = 0;
                        }
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => {
                            app.quit();
                        }
                        KeyCode::Char('?') | KeyCode::Char('m') => {
                            app.show_help = true;
                        }
                        KeyCode::Char('a') => {
                            app.show_add_feed = true;
                            app.input_feed_url.clear();
                        }
                        KeyCode::Char('c') => {
                            app.show_settings = true;
                            app.settings_step = 0;
                        }
                        KeyCode::Char('s') => {
                            if app.current_view == AppView::Feeds || app.current_view == AppView::Articles {
                                let feed_opt = if app.current_view == AppView::Feeds {
                                    if let Some(FeedUiItem::Feed(feed)) = app.ui_items.get(app.selected_ui_idx) {
                                        Some(feed.clone())
                                    } else {
                                        None
                                    }
                                } else {
                                    app.feeds.get(app.selected_feed_idx).cloned()
                                };

                                if let Some(feed) = feed_opt {
                                    match app.storage.get_articles_by_feed(feed.id) {
                                        Ok(articles) => {
                                            let unread: Vec<Article> = articles.into_iter().filter(|a| !a.is_read).collect();
                                            if unread.is_empty() {
                                                app.status_message = String::from("No unread articles in this feed to summarize.");
                                            } else {
                                                app.show_summary = true;
                                                app.summary_loading = true;
                                                app.summary_feed_title = feed.title.clone();
                                                app.summary_content.clear();
                                                app.summary_scroll = 0;

                                                let status_tx_clone = status_tx.clone();
                                                let provider = app.input_provider.clone();
                                                let api_token = app.input_api_token.clone();
                                                let model = app.input_model.clone();
                                                let api_base = app.input_api_base.clone();

                                                tokio::spawn(async move {
                                                    let _ = status_tx_clone.send(String::from("Requesting AI summary..."));
                                                    match heikal::ai::summarize_unread(
                                                        &provider,
                                                        &api_token,
                                                        &model,
                                                        &api_base,
                                                        &unread,
                                                    )
                                                    .await
                                                    {
                                                        Ok(summary) => {
                                                            let _ = status_tx_clone.send(format!("AI_SUMMARY_SUCCESS:{}", summary));
                                                        }
                                                        Err(e) => {
                                                            let _ = status_tx_clone.send(format!("AI_SUMMARY_ERROR:{}", e));
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                        Err(e) => {
                                            app.status_message = format!("Database error: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('e') => {
                            if app.current_view == AppView::Feeds {
                                if let Some(item) = app.ui_items.get(app.selected_ui_idx) {
                                    if let FeedUiItem::Feed(feed) = item {
                                        app.show_edit_feed = true;
                                        app.edit_feed_id = feed.id;
                                        app.edit_feed_step = 0;
                                        app.edit_input_title = feed.title.clone();
                                        app.edit_input_url = feed.url.clone();
                                        app.edit_input_category = feed.category.clone();
                                    }
                                }
                            }
                        }
                        KeyCode::Char('j') | KeyCode::Down => {
                            app.next();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            app.previous();
                        }
                        KeyCode::Char('h') | KeyCode::Left | KeyCode::Esc => {
                            app.back();
                        }
                        KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                            app.enter()?;
                        }
                        KeyCode::Char('o') => {
                            app.open_selected();
                        }
                        KeyCode::Char('r') => {
                            app.refresh_feeds()?;
                            app.refresh_articles()?;
                            app.status_message = String::from("Refreshed");
                        }
                        _ => {}
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
