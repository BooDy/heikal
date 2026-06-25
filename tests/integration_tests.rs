use heikal::db::{SqliteStorage, Storage};
use heikal::models::Article;
use heikal::shaping::{shape_text, TextShaper};
use heikal::sync::parse_feed;
use tempfile::NamedTempFile;

#[test]
fn test_db_operations() {
    let db_file = NamedTempFile::new().unwrap();
    let storage = SqliteStorage::new(db_file.path()).unwrap();

    // Test add_feed
    let feed_id = storage
        .add_feed(
            "Test Feed",
            "http://example.com/rss",
            Some("http://example.com"),
            Some("Description"),
            None,
        )
        .unwrap();
    assert!(feed_id > 0);

    // Test get_feeds
    let feeds = storage.get_feeds().unwrap();
    assert_eq!(feeds.len(), 1);
    assert_eq!(feeds[0].title, "Test Feed");
    assert_eq!(feeds[0].category, "Uncategorized");

    // Test add_articles
    let article = Article {
        id: 0,
        feed_id,
        title: "Test Article".to_string(),
        link: "http://example.com/1".to_string(),
        description: Some("Desc".to_string()),
        content: Some("Content".to_string()),
        author: Some("Author".to_string()),
        published: None,
        is_read: false,
    };
    storage.add_articles(vec![article]).unwrap();

    // Test get_articles_by_feed
    let articles = storage.get_articles_by_feed(feed_id).unwrap();
    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0].title, "Test Article");
    assert_eq!(articles[0].is_read, false);

    // Test mark_as_read
    storage.mark_as_read(articles[0].id).unwrap();
    let articles = storage.get_articles_by_feed(feed_id).unwrap();
    assert_eq!(articles[0].is_read, true);

    // Test delete_feed
    storage.delete_feed(feed_id).unwrap();
    let feeds = storage.get_feeds().unwrap();
    assert_eq!(feeds.len(), 0);
}

#[test]
fn test_feed_parsing_integration() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0">
<channel>
    <title>Integration Test</title>
    <item>
        <title>Article 1</title>
        <link>http://example.com/1</link>
    </item>
    <item>
        <title>Article 2</title>
        <link>http://example.com/2</link>
    </item>
</channel>
</rss>"#;
    let articles = parse_feed(xml.as_bytes(), 1).unwrap();
    assert_eq!(articles.len(), 2);
    assert_eq!(articles[0].title, "Article 1");
    assert_eq!(articles[1].title, "Article 2");
}
#[test]
fn test_text_shaping_integration() {
    // Test fallback shaping (no font needed)
    let cases = vec![
        ("", 20),
        ("Hello", 20),
        ("مرحبا", 20),
        ("Hello مرحبا", 20),
        ("A very long line that should be wrapped into multiple lines because it exceeds the width limit", 10),
        ("مرحبا بك في تطبيق heikal الذي يدعم اللغة العربية بشكل ممتاز", 10),
    ];

    for (input, width) in cases {
        let shaped = shape_text(None, input, width);
        assert!(!shaped.is_empty() || input.is_empty());
    }

    // Test with real shaper if possible
    if let Ok(shaper) = TextShaper::new() {
        let input = "مرحبا بك";
        let shaped_real = shape_text(Some(&shaper), input, 20);
        assert!(!shaped_real.is_empty());
    }
}

#[tokio::test]
async fn test_app_navigation() {
    let db_file = NamedTempFile::new().unwrap();
    let storage = std::sync::Arc::new(SqliteStorage::new(db_file.path()).unwrap());

    // Add some data
    let feed_id = storage
        .add_feed("Test Feed", "http://example.com/rss", None, None, None)
        .unwrap();
    storage
        .add_articles(vec![Article {
            id: 0,
            feed_id,
            title: "Article 1".to_string(),
            link: "http://example.com/1".to_string(),
            description: None,
            content: None,
            author: None,
            published: None,
            is_read: false,
        }])
        .unwrap();

    let mut app = heikal::app::App::new(storage).unwrap();
    assert_eq!(app.current_view, heikal::app::AppView::Feeds);
    assert_eq!(app.feeds.len(), 1);

    // Move selection from Category header to Feed item
    app.next();
    // Enter Articles view
    app.enter().unwrap();
    assert_eq!(app.current_view, heikal::app::AppView::Articles);
    assert_eq!(app.articles.len(), 1);

    // Enter Reader view
    app.enter().unwrap();
    assert_eq!(app.current_view, heikal::app::AppView::Reader);
    assert!(app.articles[0].is_read);

    // Go back
    app.back();
    assert_eq!(app.current_view, heikal::app::AppView::Articles);
    app.back();
    assert_eq!(app.current_view, heikal::app::AppView::Feeds);
}

#[tokio::test]
async fn test_app_help_and_empty_feeds() {
    let db_file = NamedTempFile::new().unwrap();
    let storage = std::sync::Arc::new(SqliteStorage::new(db_file.path()).unwrap());

    let mut app = heikal::app::App::new(storage).unwrap();
    assert_eq!(app.feeds.len(), 0);
    assert!(!app.show_help);

    // Toggle help
    app.show_help = true;
    assert!(app.show_help);

    // Toggle help off
    app.show_help = false;
    assert!(!app.show_help);
}

#[tokio::test]
async fn test_app_add_feed_state() {
    let db_file = NamedTempFile::new().unwrap();
    let storage = std::sync::Arc::new(SqliteStorage::new(db_file.path()).unwrap());

    let mut app = heikal::app::App::new(storage).unwrap();
    assert!(!app.show_add_feed);
    assert_eq!(app.input_feed_url, "");

    // Simulate switching to add feed mode and typing
    app.show_add_feed = true;
    app.input_feed_url.push_str("https://example.com/feed.xml");
    assert!(app.show_add_feed);
    assert_eq!(app.input_feed_url, "https://example.com/feed.xml");

    // Clear state
    app.show_add_feed = false;
    app.input_feed_url.clear();
    assert!(!app.show_add_feed);
    assert_eq!(app.input_feed_url, "");
}

#[test]
fn test_update_feed_metadata() {
    let db_file = NamedTempFile::new().unwrap();
    let storage = SqliteStorage::new(db_file.path()).unwrap();

    let feed_id = storage
        .add_feed(
            "http://example.com/rss",
            "http://example.com/rss",
            None,
            None,
            None,
        )
        .unwrap();

    // Verify initial title is the URL
    let feeds = storage.get_feeds().unwrap();
    assert_eq!(feeds[0].title, "http://example.com/rss");

    // Update feed metadata
    storage
        .update_feed_metadata(
            feed_id,
            "Test Feed Title",
            Some("http://example.com"),
            Some("Feed Description"),
        )
        .unwrap();

    // Verify it is updated
    let feeds = storage.get_feeds().unwrap();
    assert_eq!(feeds[0].title, "Test Feed Title");
    assert_eq!(feeds[0].site_url, Some("http://example.com".to_string()));
    assert_eq!(feeds[0].description, Some("Feed Description".to_string()));
}

#[tokio::test]
async fn test_app_edit_feed_state() {
    let db_file = NamedTempFile::new().unwrap();
    let storage = std::sync::Arc::new(SqliteStorage::new(db_file.path()).unwrap());

    // Add a feed to edit
    let feed_id = storage
        .add_feed(
            "Initial Title",
            "http://initial.com/rss",
            None,
            None,
            Some("Initial Category"),
        )
        .unwrap();

    let mut app = heikal::app::App::new(storage).unwrap();
    assert!(!app.show_edit_feed);

    // Simulate selecting a feed item and pressing 'e'
    app.rebuild_ui_items();
    let feed_idx = app
        .ui_items
        .iter()
        .position(|item| {
            if let heikal::app::FeedUiItem::Feed(f) = item {
                f.id == feed_id
            } else {
                false
            }
        })
        .expect("Feed item should be in UI items");

    app.selected_ui_idx = feed_idx;
    app.current_view = heikal::app::AppView::Feeds;

    // Simulate trigger of edit
    if let Some(heikal::app::FeedUiItem::Feed(feed)) = app.ui_items.get(app.selected_ui_idx) {
        app.show_edit_feed = true;
        app.edit_feed_id = feed.id;
        app.edit_feed_step = 0;
        app.edit_input_title = feed.title.clone();
        app.edit_input_url = feed.url.clone();
        app.edit_input_category = feed.category.clone();
    }

    assert!(app.show_edit_feed);
    assert_eq!(app.edit_feed_id, feed_id);
    assert_eq!(app.edit_input_title, "Initial Title");
    assert_eq!(app.edit_input_url, "http://initial.com/rss");
    assert_eq!(app.edit_input_category, "Initial Category");

    // Perform edits
    app.edit_input_title = "New Manual Title".to_string();
    app.edit_input_url = "http://newurl.com/rss".to_string();
    app.edit_input_category = "New Category".to_string();

    // Simulate saving on step 2
    app.edit_feed_step = 2;
    let title = app.edit_input_title.trim().to_string();
    let url = app.edit_input_url.trim().to_string();
    let category = app.edit_input_category.trim().to_string();

    app.storage
        .update_feed_details(app.edit_feed_id, &title, &url, &category)
        .unwrap();
    app.refresh_feeds().unwrap();
    app.show_edit_feed = false;

    // Verify database updates
    let updated_feeds = app.storage.get_feeds().unwrap();
    assert_eq!(updated_feeds.len(), 1);
    assert_eq!(updated_feeds[0].title, "New Manual Title");
    assert_eq!(updated_feeds[0].url, "http://newurl.com/rss");
    assert_eq!(updated_feeds[0].category, "New Category");
}

#[test]
fn test_html_shaping_integration() {
    let html_content = "<p>Hello <b>World</b>!</p>\n<ul>\n  <li>Item 1</li>\n</ul>";
    let plain_text = html2text::from_read(html_content.as_bytes(), 80);

    // Check that html2text correctly stripped HTML and formatted
    assert!(plain_text.contains("Hello World!"));
    assert!(plain_text.contains("* Item 1"));

    // Check shape_preformatted_text preserves lines and structure
    let lines = heikal::shaping::shape_preformatted_text(None, &plain_text, 80);
    assert!(lines.iter().any(|l| l.contains("Hello World!")));
    assert!(lines.iter().any(|l| l.contains("* Item 1")));
}

#[test]
fn test_html2text_min_width() {
    use html2text::config;
    for w in 0..100 {
        let result = config::plain()
            .allow_width_overflow()
            .string_from_read("<p>Hello world</p>".as_bytes(), w);
        if w > 0 {
            assert!(result.is_ok());
        }
    }
}

#[tokio::test]
async fn test_app_current_view_flow() {
    let db_file = NamedTempFile::new().unwrap();
    let storage = std::sync::Arc::new(SqliteStorage::new(db_file.path()).unwrap());

    let app = heikal::app::App::new(storage).unwrap();
    // Default view when starting the app is Feeds, which renders the dashboard
    assert_eq!(app.current_view, heikal::app::AppView::Feeds);
}

#[test]
fn test_app_settings() {
    let db_file = NamedTempFile::new().unwrap();
    let storage = SqliteStorage::new(db_file.path()).unwrap();

    // Verify settings default to None
    assert!(storage.get_setting("ai_provider").unwrap().is_none());

    // Set settings
    storage.set_setting("ai_provider", "OpenAI").unwrap();
    storage.set_setting("ai_model", "gpt-4o").unwrap();

    // Get settings and verify
    assert_eq!(storage.get_setting("ai_provider").unwrap().unwrap(), "OpenAI");
    assert_eq!(storage.get_setting("ai_model").unwrap().unwrap(), "gpt-4o");
}
