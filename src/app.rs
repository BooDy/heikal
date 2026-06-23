use crate::db::Storage;
use crate::error::Result;
use crate::models::{Article, Feed};
use crate::shaping::TextShaper;
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq)]
pub enum AppView {
    Feeds,
    Articles,
    Reader,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeedUiItem {
    Category(String),
    Feed(Feed),
}

pub struct App {
    pub storage: Arc<dyn Storage>,
    pub feeds: Vec<Feed>,
    pub articles: Vec<Article>,
    pub selected_feed_idx: usize,
    pub selected_article_idx: usize,
    pub current_view: AppView,
    pub should_quit: bool,
    pub status_message: String,
    pub shaper: Option<TextShaper>,
    pub shaped_article_cache: Option<(usize, Vec<String>)>,
    pub show_help: bool,
    pub show_add_feed: bool,
    pub input_feed_url: String,
    pub ui_items: Vec<FeedUiItem>,
    pub selected_ui_idx: usize,
    pub collapsed_categories: std::collections::HashSet<String>,
    pub add_feed_step: u8,
    pub input_category: String,
    pub show_edit_feed: bool,
    pub edit_feed_id: i64,
    pub edit_feed_step: u8,
    pub edit_input_title: String,
    pub edit_input_url: String,
    pub edit_input_category: String,
}

impl App {
    pub fn new(storage: Arc<dyn Storage>) -> Result<Self> {
        let shaper = TextShaper::new().ok();
        let mut app = Self {
            storage,
            feeds: Vec::new(),
            articles: Vec::new(),
            selected_feed_idx: 0,
            selected_article_idx: 0,
            current_view: AppView::Feeds,
            should_quit: false,
            status_message: String::from("Ready"),
            shaper,
            shaped_article_cache: None,
            show_help: false,
            show_add_feed: false,
            input_feed_url: String::new(),
            ui_items: Vec::new(),
            selected_ui_idx: 0,
            collapsed_categories: std::collections::HashSet::new(),
            add_feed_step: 0,
            input_category: String::new(),
            show_edit_feed: false,
            edit_feed_id: 0,
            edit_feed_step: 0,
            edit_input_title: String::new(),
            edit_input_url: String::new(),
            edit_input_category: String::new(),
        };
        app.refresh_feeds()?;
        Ok(app)
    }

    pub fn refresh_feeds(&mut self) -> Result<()> {
        self.feeds = self.storage.get_feeds()?;
        if self.selected_feed_idx >= self.feeds.len() && !self.feeds.is_empty() {
            self.selected_feed_idx = self.feeds.len() - 1;
        }
        self.rebuild_ui_items();
        self.shaped_article_cache = None;
        Ok(())
    }

    pub fn rebuild_ui_items(&mut self) {
        let mut grouped: std::collections::BTreeMap<String, Vec<Feed>> =
            std::collections::BTreeMap::new();
        for feed in &self.feeds {
            grouped
                .entry(feed.category.clone())
                .or_default()
                .push(feed.clone());
        }

        let mut new_items = Vec::new();
        for (category, mut feeds) in grouped {
            feeds.sort_by_key(|a| a.title.to_lowercase());
            new_items.push(FeedUiItem::Category(category.clone()));
            if !self.collapsed_categories.contains(&category) {
                for feed in feeds {
                    new_items.push(FeedUiItem::Feed(feed));
                }
            }
        }
        self.ui_items = new_items;

        if self.selected_ui_idx >= self.ui_items.len() && !self.ui_items.is_empty() {
            self.selected_ui_idx = self.ui_items.len() - 1;
        }
    }

    pub fn refresh_articles(&mut self) -> Result<()> {
        if let Some(feed) = self.feeds.get(self.selected_feed_idx) {
            self.articles = self.storage.get_articles_by_feed(feed.id)?;
            if self.selected_article_idx >= self.articles.len() && !self.articles.is_empty() {
                self.selected_article_idx = self.articles.len() - 1;
            }
        } else {
            self.articles = Vec::new();
            self.selected_article_idx = 0;
        }
        self.shaped_article_cache = None;
        Ok(())
    }

    pub fn next(&mut self) {
        match self.current_view {
            AppView::Feeds => {
                if !self.ui_items.is_empty() {
                    self.selected_ui_idx = (self.selected_ui_idx + 1) % self.ui_items.len();
                    self.shaped_article_cache = None;
                }
            }
            AppView::Articles => {
                if !self.articles.is_empty() {
                    self.selected_article_idx =
                        (self.selected_article_idx + 1) % self.articles.len();
                    self.shaped_article_cache = None;
                }
            }
            AppView::Reader => {}
        }
    }

    pub fn previous(&mut self) {
        match self.current_view {
            AppView::Feeds => {
                if !self.ui_items.is_empty() {
                    if self.selected_ui_idx == 0 {
                        self.selected_ui_idx = self.ui_items.len() - 1;
                    } else {
                        self.selected_ui_idx -= 1;
                    }
                    self.shaped_article_cache = None;
                }
            }
            AppView::Articles => {
                if !self.articles.is_empty() {
                    if self.selected_article_idx == 0 {
                        self.selected_article_idx = self.articles.len() - 1;
                    } else {
                        self.selected_article_idx -= 1;
                    }
                    self.shaped_article_cache = None;
                }
            }
            AppView::Reader => {}
        }
    }

    pub fn enter(&mut self) -> Result<()> {
        match self.current_view {
            AppView::Feeds => {
                if let Some(item) = self.ui_items.get(self.selected_ui_idx) {
                    match item {
                        FeedUiItem::Category(name) => {
                            if self.collapsed_categories.contains(name) {
                                self.collapsed_categories.remove(name);
                            } else {
                                self.collapsed_categories.insert(name.clone());
                            }
                            self.rebuild_ui_items();
                        }
                        FeedUiItem::Feed(feed) => {
                            if let Some(idx) = self.feeds.iter().position(|f| f.id == feed.id) {
                                self.selected_feed_idx = idx;
                            }
                            self.refresh_articles()?;
                            self.current_view = AppView::Articles;
                            self.selected_article_idx = 0;
                        }
                    }
                }
            }
            AppView::Articles => {
                if let Some(article) = self.articles.get(self.selected_article_idx) {
                    self.storage.mark_as_read(article.id)?;
                    // Update local state to reflect read status
                    if let Some(a) = self.articles.get_mut(self.selected_article_idx) {
                        a.is_read = true;
                    }
                }
                self.current_view = AppView::Reader;
            }
            AppView::Reader => {}
        }
        self.shaped_article_cache = None;
        Ok(())
    }

    pub fn back(&mut self) {
        match self.current_view {
            AppView::Feeds => {}
            AppView::Articles => {
                self.current_view = AppView::Feeds;
            }
            AppView::Reader => {
                self.current_view = AppView::Articles;
            }
        }
        self.shaped_article_cache = None;
    }

    pub fn open_selected(&self) {
        if let Some(article) = self.articles.get(self.selected_article_idx) {
            let link = &article.link;
            #[cfg(target_os = "linux")]
            let _ = std::process::Command::new("xdg-open").arg(link).spawn();
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open").arg(link).spawn();
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("cmd")
                .arg("/C")
                .arg("start")
                .arg(link)
                .spawn();
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}
