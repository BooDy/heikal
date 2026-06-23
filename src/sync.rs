use crate::db::Storage;
use crate::error::Result;
use crate::models::{Article, Feed};
use std::io::Cursor;
use std::sync::Arc;
use tokio::time::{self, Duration};

pub async fn fetch_feed(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let response = client.get(url).send().await?;
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}

pub fn parse_feed(content: &[u8], feed_id: i64) -> Result<Vec<Article>> {
    let cursor = Cursor::new(content);
    let feed = feed_rs::parser::parse(cursor)?;
    let articles = feed
        .entries
        .into_iter()
        .map(|entry| Article {
            id: 0,
            feed_id,
            title: entry.title.map(|t| t.content).unwrap_or_default(),
            link: entry
                .links
                .first()
                .map(|l| l.href.clone())
                .unwrap_or_default(),
            description: entry.summary.map(|s| s.content),
            content: entry.content.and_then(|c| c.body),
            author: entry.authors.first().map(|a| a.name.clone()),
            published: entry
                .published
                .or(entry.updated)
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            is_read: false,
        })
        .collect();
    Ok(articles)
}

pub fn log_error(err: &str) {
    if let Ok(home) = std::env::var("HOME") {
        let log_dir = format!("{}/.local/state/heikal", home);
        let _ = std::fs::create_dir_all(&log_dir);
        let log_path = format!("{}/log", log_dir);
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
        {
            use std::io::Write;
            let now = chrono::Utc::now();
            let _ = writeln!(file, "[{}] {}", now, err);
        }
    }
}

pub async fn sync_feed(storage: &dyn Storage, feed: &Feed) -> Result<()> {
    match do_sync_feed(storage, feed).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let error_msg = format!("Failed to sync feed {} (ID: {}): {}", feed.url, feed.id, e);
            log_error(&error_msg);
            Err(e)
        }
    }
}

async fn do_sync_feed(storage: &dyn Storage, feed: &Feed) -> Result<()> {
    let content = fetch_feed(&feed.url).await?;
    let cursor = Cursor::new(&content);
    let parsed_feed = feed_rs::parser::parse(cursor)?;

    let parsed_title = parsed_feed
        .title
        .as_ref()
        .map(|t| t.content.as_str())
        .unwrap_or(&feed.url);
    let site_url = parsed_feed.links.first().map(|l| l.href.as_str());
    let description = parsed_feed.description.as_ref().map(|d| d.content.as_str());

    storage.update_feed_metadata(feed.id, parsed_title, site_url, description)?;

    let articles = parsed_feed
        .entries
        .into_iter()
        .map(|entry| Article {
            id: 0,
            feed_id: feed.id,
            title: entry.title.map(|t| t.content).unwrap_or_default(),
            link: entry
                .links
                .first()
                .map(|l| l.href.clone())
                .unwrap_or_default(),
            description: entry.summary.map(|s| s.content),
            content: entry.content.and_then(|c| c.body),
            author: entry.authors.first().map(|a| a.name.clone()),
            published: entry
                .published
                .or(entry.updated)
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            is_read: false,
        })
        .collect();

    storage.add_articles(articles)?;
    Ok(())
}

pub async fn sync_all_feeds(storage: &dyn Storage) -> Result<()> {
    let feeds = storage.get_feeds()?;
    let mut failures = Vec::new();
    for feed in feeds {
        if let Err(e) = sync_feed(storage, &feed).await {
            failures.push(format!("{} ({})", feed.title, e));
        }
    }
    if !failures.is_empty() {
        return Err(crate::error::RssyError::Other(format!(
            "Failed to sync {} feed(s)",
            failures.len()
        )));
    }
    Ok(())
}

pub async fn sync_loop(
    storage: Arc<dyn Storage>,
    interval: Duration,
    status_tx: tokio::sync::mpsc::UnboundedSender<String>,
) {
    let mut interval_timer = time::interval(interval);
    loop {
        interval_timer.tick().await;
        let _ = status_tx.send(String::from("Syncing..."));
        if let Err(e) = sync_all_feeds(storage.as_ref()).await {
            let _ = status_tx.send(format!("Sync error: {}", e));
        } else {
            let _ = status_tx.send(String::from("Sync complete"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_feed() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0">
<channel>
    <title>Test Feed</title>
    <item>
        <title>Test Article</title>
        <link>http://example.com/1</link>
        <description>Test Description</description>
    </item>
</channel>
</rss>"#;
        let articles = parse_feed(xml.as_bytes(), 1).unwrap();
        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].title, "Test Article");
        assert_eq!(articles[0].link, "http://example.com/1");
        assert_eq!(
            articles[0].description,
            Some("Test Description".to_string())
        );
    }

    #[test]
    fn test_parse_feed_atom() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Example Feed</title>
  <entry>
    <title>Atom Article</title>
    <link href="http://example.org/2003/12/13/atom03"/>
    <id>urn:uuid:84068d6a-950d-11ee-b9d1-0242ac120002</id>
    <updated>2003-12-13T18:30:02Z</updated>
    <summary>Some summary</summary>
    <content type="html">
      Hello, world!
    </content>
    <author>
      <name>John Doe</name>
    </author>
  </entry>
</feed>"#;
        let articles = parse_feed(xml.as_bytes(), 1).unwrap();
        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].title, "Atom Article");
        assert_eq!(articles[0].link, "http://example.org/2003/12/13/atom03");
        assert_eq!(articles[0].description, Some("Some summary".to_string()));
        assert!(articles[0]
            .content
            .as_ref()
            .unwrap()
            .contains("Hello, world!"));
        assert_eq!(articles[0].author, Some("John Doe".to_string()));
        assert!(articles[0].published.is_some());
    }
}
