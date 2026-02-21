# Glow Rust - File System Operations

This document covers file discovery, watching, and gitignore handling.

## File Finder

The file finder searches for markdown files in a directory, respecting gitignore rules.

### Implementation

```rust
// src/file/finder.rs

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::SystemTime;
use walkdir::WalkDir;
use ignore::gitignore::Gitignore;

/// Markdown file extensions
const MARKDOWN_EXTENSIONS: &[&str] = &[
    "md", "mdown", "mkdn", "mkd", "markdown",
];

/// Result from file search
#[derive(Debug, Clone)]
pub struct FileSearchResult {
    pub path: PathBuf,
    pub modified: SystemTime,
    pub is_dir: bool,
}

/// File finder that respects gitignore
pub struct FileFinder {
    directory: PathBuf,
    show_all: bool,
    ignore_patterns: Vec<String>,
    tx: Sender<FileSearchResult>,
}

impl FileFinder {
    /// Create a new file finder
    pub fn new(directory: &Path) -> Self {
        let (tx, _) = mpsc::channel();
        Self {
            directory: directory.to_path_buf(),
            show_all: false,
            ignore_patterns: Vec::new(),
            tx,
        }
    }
    
    /// Show all files including ignored ones
    pub fn show_all(mut self, show: bool) -> Self {
        self.show_all = show;
        self
    }
    
    /// Add additional ignore patterns
    pub fn ignore_patterns(mut self, patterns: Vec<String>) -> Self {
        self.ignore_patterns = patterns;
        self
    }
    
    /// Set the sender for results
    pub fn sender(mut self, tx: Sender<FileSearchResult>) -> Self {
        self.tx = tx;
        self
    }
    
    /// Start the file search in a background thread
    /// Returns a receiver for the results
    pub fn spawn(self) -> Receiver<FileSearchResult> {
        let (tx, rx) = mpsc::channel();
        let tx = if let Ok(existing) = self.tx.send(FileSearchResult {
            path: PathBuf::new(),
            modified: SystemTime::UNIX_EPOCH,
            is_dir: false,
        }) {
            // Use provided sender if it worked
            drop(existing);
            tx
        } else {
            tx
        };
        
        let directory = self.directory.clone();
        let show_all = self.show_all;
        let ignore_patterns = self.ignore_patterns;
        
        std::thread::spawn(move || {
            // Build gitignore if needed
            let gitignore = if !show_all {
                build_gitignore(&directory, &ignore_patterns)
            } else {
                None
            };
            
            // Walk directory
            for entry in WalkDir::new(&directory)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                
                // Skip if ignored
                if let Some(ref gi) = gitignore {
                    if gi.matched(path, false).is_ignore() {
                        continue;
                    }
                }
                
                // Check if it's a markdown file
                if is_markdown_file(path) {
                    if let Ok(metadata) = entry.metadata() {
                        let _ = tx.send(FileSearchResult {
                            path: path.to_path_buf(),
                            modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                            is_dir: metadata.is_dir(),
                        });
                    }
                }
            }
        });
        
        rx
    }
    
    /// Find files synchronously (blocks until complete)
    pub fn find(self) -> Vec<FileSearchResult> {
        let mut results = Vec::new();
        
        let gitignore = if !self.show_all {
            build_gitignore(&self.directory, &self.ignore_patterns)
        } else {
            None
        };
        
        for entry in WalkDir::new(&self.directory)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            if let Some(ref gi) = gitignore {
                if gi.matched(path, false).is_ignore() {
                    continue;
                }
            }
            
            if is_markdown_file(path) {
                if let Ok(metadata) = entry.metadata() {
                    results.push(FileSearchResult {
                        path: path.to_path_buf(),
                        modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                        is_dir: metadata.is_dir(),
                    });
                }
            }
        }
        
        results
    }
}

/// Check if a file is a markdown file
fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| MARKDOWN_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Build a gitignore matcher
fn build_gitignore(dir: &Path, extra_patterns: &[String]) -> Option<Gitignore> {
    let mut builder = ignore::gitignore::GitignoreBuilder::new(dir);
    
    // Standard ignore patterns
    let standard_patterns = [
        ".git",
        "node_modules",
        "target",
        "vendor",
        ".hg",
        ".svn",
        "__pycache__",
        "*.pyc",
        ".DS_Store",
        "Thumbs.db",
        "*.swp",
        "*.swo",
        "*~",
        "#*#",
    ];
    
    for pattern in &standard_patterns {
        builder.add_line(None, pattern).ok()?;
    }
    
    // Load .gitignore from directory
    let gitignore_path = dir.join(".gitignore");
    if gitignore_path.exists() {
        builder.add(&gitignore_path).ok()?;
    }
    
    // Add extra patterns
    for pattern in extra_patterns {
        builder.add_line(None, pattern).ok()?;
    }
    
    builder.build().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;
    
    #[test]
    fn test_find_markdown_files() {
        let dir = TempDir::new().unwrap();
        
        // Create test files
        File::create(dir.path().join("test.md")).unwrap();
        File::create(dir.path().join("README.md")).unwrap();
        File::create(dir.path().join("notes.markdown")).unwrap();
        File::create(dir.path().join("ignore.txt")).unwrap();
        
        let results = FileFinder::new(dir.path()).find();
        
        assert_eq!(results.len(), 3);
        assert!(results.iter().any(|r| r.path.ends_with("test.md")));
        assert!(results.iter().any(|r| r.path.ends_with("README.md")));
        assert!(results.iter().any(|r| r.path.ends_with("notes.markdown")));
    }
}
```

## File Watcher

Watch files for changes and trigger reloads.

```rust
// src/file/watcher.rs

use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event, EventKind, Config as NotifyConfig};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

/// Events from file watcher
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// File was modified
    Modified(PathBuf),
    /// File was created
    Created(PathBuf),
    /// File was deleted
    Deleted(PathBuf),
    /// Watcher error
    Error(String),
}

/// File system watcher
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    watched_path: PathBuf,
    rx: Receiver<WatchEvent>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new() -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::channel();
        
        // Create watcher with callback
        let tx_clone = tx.clone();
        let watcher = notify::recommended_watcher(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        let paths = event.paths.clone();
                        let event_type = match event.kind {
                            EventKind::Modify(_) => Some(WatchEvent::Modified),
                            EventKind::Create(_) => Some(WatchEvent::Created),
                            EventKind::Remove(_) => Some(WatchEvent::Deleted),
                            _ => None,
                        };
                        
                        if let Some(event_type) = event_type {
                            for path in paths {
                                let _ = tx_clone.send(event_type(path));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(WatchEvent::Error(e.to_string()));
                    }
                }
            }
        )?;
        
        Ok(Self {
            watcher,
            watched_path: PathBuf::new(),
            rx,
        })
    }
    
    /// Start watching a file's directory
    pub fn watch(&mut self, path: &Path) -> anyhow::Result<()> {
        let dir = path.parent()
            .ok_or_else(|| anyhow::anyhow!("No parent directory"))?;
        
        self.watcher.watch(dir, RecursiveMode::NonRecursive)?;
        self.watched_path = path.to_path_buf();
        
        Ok(())
    }
    
    /// Stop watching
    pub fn unwatch(&mut self) -> anyhow::Result<()> {
        if let Some(dir) = self.watched_path.parent() {
            self.watcher.unwatch(dir)?;
        }
        self.watched_path = PathBuf::new();
        Ok(())
    }
    
    /// Get the watched file path
    pub fn watched_path(&self) -> Option<&Path> {
        if self.watched_path.is_empty() {
            None
        } else {
            Some(&self.watched_path)
        }
    }
    
    /// Try to receive a watch event (non-blocking)
    pub fn try_recv(&self) -> Option<WatchEvent> {
        self.rx.try_recv().ok()
    }
    
    /// Check if there are pending events
    pub fn has_events(&self) -> bool {
        !self.rx.is_empty()
    }
    
    /// Check if a modification event is for the watched file
    pub fn is_watched_file(&self, path: &Path) -> bool {
        self.watched_path == path
    }
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new().expect("Failed to create file watcher")
    }
}

/// Debounced file watcher (prevents rapid-fire events)
pub struct DebouncedWatcher {
    watcher: FileWatcher,
    last_event: Option<(WatchEvent, std::time::Instant)>,
    debounce_duration: Duration,
}

impl DebouncedWatcher {
    pub fn new(debounce_duration: Duration) -> anyhow::Result<Self> {
        Ok(Self {
            watcher: FileWatcher::new()?,
            last_event: None,
            debounce_duration,
        })
    }
    
    pub fn watch(&mut self, path: &Path) -> anyhow::Result<()> {
        self.watcher.watch(path)
    }
    
    pub fn unwatch(&mut self) -> anyhow::Result<()> {
        self.watcher.unwatch()
    }
    
    /// Try to receive a debounced event
    pub fn try_recv(&mut self) -> Option<WatchEvent> {
        // Get raw event
        if let Some(event) = self.watcher.try_recv() {
            let now = std::time::Instant::now();
            
            // Check if we should debounce
            if let Some((_, last_time)) = &self.last_event {
                if now.duration_since(*last_time) < self.debounce_duration {
                    // Skip this event (debounced)
                    return None;
                }
            }
            
            self.last_event = Some((event.clone(), now));
            return Some(event);
        }
        
        None
    }
}
```

## Document Model

```rust
// src/markdown/document.rs

use std::path::PathBuf;
use std::time::SystemTime;
use time::OffsetDateTime;

/// Represents a markdown document
#[derive(Debug, Clone)]
pub struct MarkdownDocument {
    /// Full path to the local markdown file
    pub local_path: PathBuf,
    
    /// Value used for filtering (normalized filename)
    pub filter_value: String,
    
    /// The raw markdown content
    pub body: String,
    
    /// Display name (relative path from cwd)
    pub note: String,
    
    /// Last modification time
    pub modified: OffsetDateTime,
}

impl MarkdownDocument {
    /// Create a new document from a file path
    pub fn from_path(path: PathBuf, cwd: &Path) -> anyhow::Result<Self> {
        let metadata = std::fs::metadata(&path)?;
        let modified: OffsetDateTime = metadata.modified()?.into();
        
        let body = std::fs::read_to_string(&path)?;
        
        let note = path.strip_prefix(cwd)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        
        let mut doc = Self {
            local_path: path,
            filter_value: String::new(),
            body,
            note,
            modified,
        };
        
        doc.build_filter_value();
        Ok(doc)
    }
    
    /// Build the filter value from the note (normalized for fuzzy search)
    pub fn build_filter_value(&mut self) {
        self.filter_value = normalize_for_search(&self.note);
    }
    
    /// Get relative time string (e.g., "2 hours ago")
    pub fn relative_time(&self) -> String {
        relative_time(self.modified)
    }
    
    /// Reload content from disk
    pub fn reload(&mut self) -> anyhow::Result<()> {
        self.body = std::fs::read_to_string(&self.local_path)?;
        let metadata = std::fs::metadata(&self.local_path)?;
        self.modified = metadata.modified()?.into();
        Ok(())
    }
}

/// Normalize text for search (remove diacritics, lowercase)
fn normalize_for_search(input: &str) -> String {
    use unicode_normalization::{nfkd, char::is_combining_mark};
    
    nfkd(input)
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
}

/// Format time as relative string
fn relative_time(then: OffsetDateTime) -> String {
    use time::{Duration, OffsetDateTime};
    
    let now = OffsetDateTime::now_utc();
    let diff = now - then;
    
    if diff < Duration::minutes(1) {
        "just now".to_string()
    } else if diff < Duration::hours(1) {
        let mins = diff.whole_minutes();
        format!("{} minute{} ago", mins, if mins == 1 { "" } else { "s" })
    } else if diff < Duration::days(1) {
        let hours = diff.whole_hours();
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else if diff < Duration::weeks(1) {
        let days = diff.whole_days();
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    } else if diff < Duration::days(30) {
        let weeks = diff.whole_weeks();
        format!("{} week{} ago", weeks, if weeks == 1 { "" } else { "s" })
    } else if diff < Duration::days(365) {
        let months = diff.whole_days() / 30;
        format!("{} month{} ago", months, if months == 1 { "" } else { "s" })
    } else {
        let years = diff.whole_days() / 365;
        format!("{} year{} ago", years, if years == 1 { "" } else { "s" })
    }
}

/// Sort documents alphabetically by note
pub fn sort_documents(docs: &mut [MarkdownDocument]) {
    docs.sort_by(|a, b| a.note.cmp(&b.note));
}
```

## Integration with App

```rust
// In app.rs

impl App {
    /// Start searching for markdown files
    pub fn start_file_search(&mut self) -> anyhow::Result<()> {
        let dir = self.config.path.clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap());
        
        let tx = self.file_sender.clone();
        
        std::thread::spawn(move || {
            let finder = FileFinder::new(&dir)
                .show_all(self.config.show_all_files)
                .sender(tx);
            
            finder.spawn();
        });
        
        Ok(())
    }
    
    /// Handle a file search result
    pub fn handle_file_result(&mut self, result: FileSearchResult) {
        let cwd = std::env::current_dir().unwrap_or_default();
        
        if let Ok(doc) = MarkdownDocument::from_path(result.path, &cwd) {
            self.stash.add_document(doc);
        }
    }
    
    /// Check for file search results
    pub fn check_file_search(&mut self) {
        if let Some(ref rx) = self.file_receiver {
            while let Ok(result) = rx.try_recv() {
                self.handle_file_result(result);
            }
        }
    }
    
    /// Start watching the current document
    pub fn watch_current_document(&mut self) -> anyhow::Result<()> {
        if let Some(ref doc) = self.pager.current_document {
            self.pager.watcher.watch(&doc.local_path)?;
        }
        Ok(())
    }
    
    /// Check for file watcher events
    pub fn check_file_watcher(&mut self) -> anyhow::Result<()> {
        if let Some(ref mut watcher) = self.pager.watcher {
            if let Some(event) = watcher.try_recv() {
                match event {
                    WatchEvent::Modified(path) | WatchEvent::Created(path) => {
                        if watcher.is_watched_file(&path) {
                            // Reload document
                            if let Some(ref mut doc) = self.pager.current_document {
                                doc.reload()?;
                                self.render_current_document()?;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
```

## Platform-Specific Considerations

### macOS

```rust
// src/file/ignore_darwin.rs

/// Additional patterns to ignore on macOS
pub const PLATFORM_IGNORE_PATTERNS: &[&str] = &[
    ".DS_Store",
    ".fseventsd",
    ".Spotlight-V100",
    ".Trashes",
    "._*",
];
```

### Windows

```rust
// src/file/ignore_windows.rs

/// Additional patterns to ignore on Windows
pub const PLATFORM_IGNORE_PATTERNS: &[&str] = &[
    "Thumbs.db",
    "desktop.ini",
    "$RECYCLE.BIN",
    "*.lnk",
];
```

### Linux/Unix

```rust
// src/file/ignore_unix.rs

/// Additional patterns to ignore on Unix
pub const PLATFORM_IGNORE_PATTERNS: &[&str] = &[
    "*~",
    ".directory",
    ".Trash-*",
];
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;
    
    #[test]
    fn test_file_finder_ignores_gitignore() {
        let dir = TempDir::new().unwrap();
        
        // Create .gitignore
        let mut gitignore = File::create(dir.path().join(".gitignore")).unwrap();
        writeln!(gitignore, "ignored.md").unwrap();
        
        // Create files
        File::create(dir.path().join("test.md")).unwrap();
        File::create(dir.path().join("ignored.md")).unwrap();
        
        let results = FileFinder::new(dir.path()).find();
        
        assert_eq!(results.len(), 1);
        assert!(results[0].path.ends_with("test.md"));
    }
    
    #[test]
    fn test_document_relative_time() {
        let doc = MarkdownDocument {
            local_path: PathBuf::from("/test.md"),
            filter_value: String::new(),
            body: String::new(),
            note: "test.md".to_string(),
            modified: OffsetDateTime::now_utc() - time::Duration::hours(2),
        };
        
        assert_eq!(doc.relative_time(), "2 hours ago");
    }
}
```
