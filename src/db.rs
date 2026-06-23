use crate::error::Result;
use crate::models::{Article, Feed};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

pub trait Storage: Send + Sync {
    fn add_feed(
        &self,
        title: &str,
        url: &str,
        site_url: Option<&str>,
        description: Option<&str>,
        category: Option<&str>,
    ) -> Result<i64>;
    fn get_feeds(&self) -> Result<Vec<Feed>>;
    fn delete_feed(&self, id: i64) -> Result<()>;
    fn update_feed_metadata(
        &self,
        id: i64,
        title: &str,
        site_url: Option<&str>,
        description: Option<&str>,
    ) -> Result<()>;
    fn update_feed_details(&self, id: i64, title: &str, url: &str, category: &str) -> Result<()>;

    fn add_articles(&self, articles: Vec<Article>) -> Result<()>;
    fn get_articles_by_feed(&self, feed_id: i64) -> Result<Vec<Article>>;
    fn mark_as_read(&self, article_id: i64) -> Result<()>;
}

pub struct SqliteStorage {
    conn: Mutex<Connection>,
}

impl SqliteStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.init_schema()?;
        Ok(storage)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS feeds (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                url TEXT NOT NULL UNIQUE,
                site_url TEXT,
                description TEXT,
                last_updated DATETIME,
                category TEXT NOT NULL DEFAULT 'Uncategorized'
            )",
            [],
        )?;

        // Alter table to add category column if it does not exist (for existing databases)
        let _ = conn.execute(
            "ALTER TABLE feeds ADD COLUMN category TEXT NOT NULL DEFAULT 'Uncategorized'",
            [],
        );

        conn.execute(
            "CREATE TABLE IF NOT EXISTS articles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                feed_id INTEGER NOT NULL,
                title TEXT NOT NULL,
                link TEXT NOT NULL UNIQUE,
                description TEXT,
                content TEXT,
                author TEXT,
                published DATETIME,
                is_read BOOLEAN NOT NULL DEFAULT 0,
                FOREIGN KEY (feed_id) REFERENCES feeds (id) ON DELETE CASCADE
            )",
            [],
        )?;

        Ok(())
    }
}

impl Storage for SqliteStorage {
    fn add_feed(
        &self,
        title: &str,
        url: &str,
        site_url: Option<&str>,
        description: Option<&str>,
        category: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let cat = category.unwrap_or("Uncategorized");
        conn.execute(
            "INSERT INTO feeds (title, url, site_url, description, category) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![title, url, site_url, description, cat],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn get_feeds(&self) -> Result<Vec<Feed>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, url, site_url, description, last_updated, category FROM feeds",
        )?;
        let feed_iter = stmt.query_map([], |row| {
            Ok(Feed {
                id: row.get(0)?,
                title: row.get(1)?,
                url: row.get(2)?,
                site_url: row.get(3)?,
                description: row.get(4)?,
                last_updated: row.get(5)?,
                category: row.get(6)?,
            })
        })?;

        let mut feeds = Vec::new();
        for feed in feed_iter {
            feeds.push(feed?);
        }
        Ok(feeds)
    }

    fn delete_feed(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM feeds WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn update_feed_metadata(
        &self,
        id: i64,
        title: &str,
        site_url: Option<&str>,
        description: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feeds SET title = ?1, site_url = ?2, description = ?3 WHERE id = ?4",
            params![title, site_url, description, id],
        )?;
        Ok(())
    }

    fn update_feed_details(&self, id: i64, title: &str, url: &str, category: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feeds SET title = ?1, url = ?2, category = ?3 WHERE id = ?4",
            params![title, url, category, id],
        )?;
        Ok(())
    }

    fn add_articles(&self, articles: Vec<Article>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        for article in articles {
            conn.execute(
                "INSERT OR IGNORE INTO articles (feed_id, title, link, description, content, author, published, is_read) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    article.feed_id,
                    article.title,
                    article.link,
                    article.description,
                    article.content,
                    article.author,
                    article.published,
                    article.is_read
                ],
            )?;
        }
        Ok(())
    }

    fn get_articles_by_feed(&self, feed_id: i64) -> Result<Vec<Article>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, feed_id, title, link, description, content, author, published, is_read 
             FROM articles WHERE feed_id = ?1 ORDER BY published DESC",
        )?;
        let article_iter = stmt.query_map(params![feed_id], |row| {
            Ok(Article {
                id: row.get(0)?,
                feed_id: row.get(1)?,
                title: row.get(2)?,
                link: row.get(3)?,
                description: row.get(4)?,
                content: row.get(5)?,
                author: row.get(6)?,
                published: row.get(7)?,
                is_read: row.get(8)?,
            })
        })?;

        let mut articles = Vec::new();
        for article in article_iter {
            articles.push(article?);
        }
        Ok(articles)
    }

    fn mark_as_read(&self, article_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE articles SET is_read = 1 WHERE id = ?1",
            params![article_id],
        )?;
        Ok(())
    }
}
