use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Protocol constants for WebSocket communication
pub mod protocol {
    pub const DEFAULT_SERVER_URL: &str = "ws://localhost:3030";
    pub const DEFAULT_SERVER_PORT: u16 = 3030;
    pub const DEFAULT_WATCH_FILE: &str = "README.md";
}

/// Represents a change in a file's content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileChange {
    /// Complete file content
    FullContent {
        file_id: String,
        content: String,
    },
    
    /// Represents a diff between versions
    Diff {
        file_id: String,
        position: usize,
        delete_count: usize,
        insert_text: String,
    }
}

impl FileChange {
    /// Creates an efficient diff between two strings
    pub fn create_diff(file_id: &str, old_content: &str, new_content: &str) -> Vec<Self> {
        let mut changes = Vec::new();
        let mut i = 0;
        let mut j = 0;
        let old_chars: Vec<char> = old_content.chars().collect();
        let new_chars: Vec<char> = new_content.chars().collect();
        while i < old_chars.len() && j < new_chars.len() {
            if old_chars[i] == new_chars[j] {
                i += 1;
                j += 1;
            } else {
                let start = i;
                while i < old_chars.len() && (j >= new_chars.len() || old_chars[i] != new_chars[j]) {
                    i += 1;
                }
                let delete_count = i - start;
                let mut insert_end = j;
                while insert_end < new_chars.len() && i < old_chars.len() && old_chars[i] != new_chars[insert_end] {
                    insert_end += 1;
                }
                let insert_text: String = new_chars[j..insert_end].iter().collect();
                if !insert_text.is_empty() || delete_count > 0 {
                    changes.push(FileChange::Diff {
                        file_id: file_id.to_string(),
                        position: start,
                        delete_count,
                        insert_text,
                    });
                }
                j = insert_end;
            }
        }
        if i < old_chars.len() {
            changes.push(FileChange::Diff {
                file_id: file_id.to_string(),
                position: i,
                delete_count: old_chars.len() - i,
                insert_text: String::new(),
            });
        } else if j < new_chars.len() {
            let insert_text: String = new_chars[j..].iter().collect();
            changes.push(FileChange::Diff {
                file_id: file_id.to_string(),
                position: old_chars.len(),
                delete_count: 0,
                insert_text,
            });
        }
        changes
    }
    
    /// Applies the change to a string in-place
    pub fn apply(&self, content: &mut String) {
        match self {
            FileChange::FullContent { content: new_content, .. } => {
                *content = new_content.clone();
            }
            FileChange::Diff { position, delete_count, insert_text, .. } => {
                if *position <= content.len() {
                    let end = (*position + *delete_count).min(content.len());
                    content.replace_range(*position..end, insert_text);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    pub content: String,
    pub last_modified: std::time::SystemTime,
}

impl Default for FileState {
    fn default() -> Self {
        Self {
            content: String::new(),
            last_modified: std::time::SystemTime::now(),
        }
    }
}

pub type FileRegistry = HashMap<String, FileState>;