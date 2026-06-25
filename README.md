# ⚡ Heikal ⚡

> **A keyboard-driven Terminal User Interface (TUI) RSS reader in Rust, featuring native bidirectional layout formatting, contextual shaping, and right-to-left (RTL) alignment for Arabic script.**

---

[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![TUI](https://img.shields.io/badge/TUI-ratatui-magenta.svg)](https://github.com/ratatui/ratatui)

Standard terminal applications write text strictly from left to right. When rendering bidirectional scripts (such as Arabic or Hebrew), terminal screens scramble mixed-language lines, print letters backwards, and break cursive connections. **Heikal** solves this by implementing a full pre-render shaping and layout pipeline, enabling a native, beautiful CLI RSS reading experience for bidirectional script readers.

---

## ✨ Features

- 📖 **Beautiful Dashboard Landing Page:** Opens to an informative landing page showing feed statistics, quick start guides, and keyboard shortcuts.
- ⚡ **Native BiDi & Arabic Shaping:** Leverages `unicode-bidi` for layout reordering and a custom Arabic text shaper supporting ligatures (e.g., Lam-Alef) and cursive joining.
- 📁 **Feed Grouping & Categories:** Group your RSS/Atom feeds into custom collapsible categories to organize your sidebar.
- 🛠️ **Interactive Edit Wizard Modal:** Edit any feed's title, update its URL, and move it between categories on the fly with a multi-step popup wizard.
- 📊 **Multi-Column Articles View:** Browse articles using a clean table showing the **Title** (with read/unread state markers), **Publish Date**, and **Read/Unread** status.
- 🌐 **Rich HTML Reader Layout:** Parses and formats HTML contents into clear, readable terminal layouts (supporting list bullet alignment, indentations, and blockquotes) using `html2text`.
- 🛡️ **Fail-Safe & Silent Logging:** Network/sync failures are silently logged to `~/.local/state/heikal/log` with single-line summary warnings in the status bar, preventing terminal layout corruption.
- 🤖 **AI-Powered Feed Summaries:** Instantly summarize all unread articles in a feed with a single keypress. Supports major providers (OpenAI, Anthropic, Gemini, OpenRouter) and local models (Ollama/Custom endpoints).
- 🖥️ **Stateless & Portable:** Leverages a lightweight local SQLite database (`heikal.db`) for caching feeds, articles, and settings.

---

## ⌨️ Keyboard Shortcuts Reference

Heikal is fully keyboard-driven. Navigate the interface with the following hotkeys:

### Global Controls
- `?` or `m` : Toggle the Help modal overlay
- `c`         : Open the interactive **AI frontier Model Configuration** wizard (Provider, Token, Model, Base URL)
- `q`         : Quit the application

### Feed Sidebar & Navigation
- `j` or `Down`  : Navigate down the feed list
- `k` or `Up`    : Navigate up the feed list
- `Enter` / `l`  : Open the selected feed and load its articles (focus moves to Articles Table)
- `h` or `Left`  : Return focus to the Feed Sidebar (when in Articles view)
- `a`            : Open the interactive **Add Feed** dialog modal (Step 1: URL, Step 2: Category)
- `e`            : Open the interactive **Edit Feed** wizard modal (Step 1: Title, Step 2: URL, Step 3: Category)
- `s`            : Asynchronously request an **AI Summary** of all unread articles in the selected feed (scroll summary modal with `j/k`)

### Articles Table
- `j` or `Down`  : Navigate down the article table
- `k` or `Up`    : Navigate up the article table
- `Enter`        : Open and read the selected article content in the TUI Reader
- `o`            : Open the selected article URL directly in your system browser (e.g., Firefox, Chrome, Safari)
- `h` or `Left`  : Close articles table and return to Feed Sidebar

### Reader View
- `h` or `Left`  : Exit the Reader view and return to the Articles Table
- `o`            : Open the source link in your system browser

---

## 🚀 Getting Started

### 📋 Prerequisites

To compile Heikal, you must have Rust and Cargo installed. If you do not have Rust installed, set it up via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

You also need the standard development libraries for your platform (SQLite development files are bundled automatically).

> [!IMPORTANT]
> **Terminal Font Requirement:** Since terminal TUIs cannot embed fonts, your terminal emulator (e.g., WezTerm, Kitty, Alacritty, or GNOME Terminal) must be configured with a font that supports Arabic glyph sets (e.g., *DejaVu Sans Mono*, *Fira Code*, or regional Arabic system fonts) to display Arabic shaped text correctly.

### 🛠️ Installation

Clone the repository and build the release binary:

```bash
git clone https://github.com/yourusername/heikal.git
cd heikal
cargo build --release
```

The compiled binary will be available at `./target/release/heikal`. You can symlink or copy it to your path:

```bash
sudo cp target/release/heikal /usr/local/bin/
```

---

## 💻 Command Line Interface (CLI)

Heikal can also be managed directly from the command line without opening the TUI interface.

### Add a new feed
```bash
heikal add <feed_url> [category]
```
*Example:*
```bash
heikal add https://news.ycombinator.com/rss "Tech"
```

### List all subscribed feeds
```bash
heikal list
```

### Delete a feed by ID
```bash
heikal delete <feed_id>
```
*(To find the feed ID, use the `heikal list` command first)*

---

## 🗃️ Directories & Logging

Heikal adheres strictly to standard XDG directories:
- **Local Cache Database:** `heikal.db` in the working directory (SQLite format).
- **Error Log File:** `~/.local/state/heikal/log` where malformed feed parser outputs or connection errors are appended.

---

## 🤝 Contributing

Contributions, issues, and feature requests are welcome! Feel free to check the [issues page](https://github.com/yourusername/heikal/issues).

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

---

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.
