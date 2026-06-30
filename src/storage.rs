//! Saving and loading conversations to disk.
//!
//! A conversation file is plain text: a few `# key: value` headers carrying
//! the model, followed by one raw turn description per line. This makes saved
//! conversations self-contained; they restore their own model and prices.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::model::Model;

/// A conversation loaded from disk.
#[derive(Clone)]
pub struct SavedConversation {
    pub name: String,
    pub model: Model,
    pub turns: Vec<String>,
    /// The file this conversation was read from, for later deletion. The on-disk
    /// filename is derived from a sanitized name, so it can't be reconstructed
    /// reliably from `name` alone, so we keep the real path instead.
    pub path: PathBuf,
}

/// Directory where conversations live: `$HOME/.token-visualizer/conversations`.
pub fn conversations_dir() -> PathBuf {
    let base = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    Path::new(&base).join(".token-visualizer").join("conversations")
}

fn sanitize(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if s.is_empty() {
        "conversation".into()
    } else {
        s
    }
}

/// Write a conversation to disk, returning the file path.
pub fn save(name: &str, model: &Model, turns: &[String]) -> io::Result<PathBuf> {
    let dir = conversations_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.txt", sanitize(name)));

    let mut s = String::new();
    s.push_str(&format!("# name: {name}\n"));
    s.push_str(&format!("# model: {}\n", model.name));
    s.push_str(&format!("# input: {}\n", model.input_per_m));
    s.push_str(&format!("# output: {}\n", model.output_per_m));
    s.push_str(&format!("# cached: {}\n", model.cached_per_m));
    s.push_str(&format!("# context: {}\n", model.context_window));
    for t in turns {
        s.push_str(t);
        s.push('\n');
    }
    fs::write(&path, s)?;
    Ok(path)
}

/// Delete a saved conversation file from disk.
pub fn delete(path: &Path) -> io::Result<()> {
    fs::remove_file(path)
}

/// List every saved conversation, sorted by name.
pub fn list() -> Vec<SavedConversation> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(conversations_dir()) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&path) {
            if let Some(c) = parse(&content, &path) {
                out.push(c);
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

fn parse(content: &str, path: &Path) -> Option<SavedConversation> {
    let mut name = path.file_stem()?.to_string_lossy().to_string();
    let mut model_name = "Custom".to_string();
    let (mut input, mut output, mut cached) = (0.0, 0.0, 0.0);
    let mut context = 1_000_000u64;
    let mut turns = Vec::new();

    for line in content.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        if let Some(v) = l.strip_prefix("# name:") {
            name = v.trim().to_string();
        } else if let Some(v) = l.strip_prefix("# model:") {
            model_name = v.trim().to_string();
        } else if let Some(v) = l.strip_prefix("# input:") {
            input = v.trim().parse().unwrap_or(0.0);
        } else if let Some(v) = l.strip_prefix("# output:") {
            output = v.trim().parse().unwrap_or(0.0);
        } else if let Some(v) = l.strip_prefix("# cached:") {
            cached = v.trim().parse().unwrap_or(0.0);
        } else if let Some(v) = l.strip_prefix("# context:") {
            context = v.trim().parse().unwrap_or(1_000_000);
        } else if l.starts_with('#') {
            // Unknown header; ignore.
        } else {
            turns.push(l.to_string());
        }
    }

    Some(SavedConversation {
        name,
        model: Model::new(&model_name, input, output, cached, context),
        turns,
        path: path.to_path_buf(),
    })
}
