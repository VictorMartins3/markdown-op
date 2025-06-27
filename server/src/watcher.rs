use std::{collections::HashMap, path::{Path, PathBuf}, sync::{Arc, Mutex}, time::Instant};
use tokio::sync::{broadcast, mpsc};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event};
use shared::FileChange;

const DEBOUNCE_MS: u64 = 25;

lazy_static::lazy_static! {
    static ref LAST_CONTENT: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    static ref DEBOUNCE_STATE: Mutex<HashMap<PathBuf, Instant>> = Mutex::new(HashMap::new());
}

/// File watcher for a single file
pub struct FileWatcher {
    watcher: RecommendedWatcher,
}

impl FileWatcher {
    /// Creates a new file watcher
    pub fn new() -> Self {
        Self {
            watcher: notify::recommended_watcher(|_| {}).expect("Failed to create watcher"),
        }
    }
    
    /// Starts watching a file with
    /// event processing
    pub fn watch_file(
        &mut self,
        file_id: String,
        watch_path: &str,
        sender: broadcast::Sender<FileChange>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let abs_path = Self::absolute_path(watch_path)?;
        let parent_dir = abs_path.parent().unwrap_or_else(|| Path::new("."));
        let file_id = Arc::new(file_id);
        let (event_tx, mut event_rx) = mpsc::channel(500);
        let mut watcher = notify::recommended_watcher(move |result| {
            if let Ok(event) = result {
                let _ = event_tx.blocking_send(event);
            } else if let Err(e) = result {
                eprintln!("Watcher error: {e:?}");
            }
        })?;
        watcher.watch(parent_dir, RecursiveMode::NonRecursive)?;
        self.watcher = watcher;
        let file_id_clone = Arc::clone(&file_id);
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                handle_event(event, sender.clone(), &file_id_clone).await;
            }
        });
        Ok(())
    }

    fn absolute_path(path: &str) -> Result<PathBuf, std::io::Error> {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            Ok(path)
        } else {
            Ok(std::env::current_dir()?.join(&path))
        }
    }
}

/// event processing with better filtering and faster response
async fn handle_event(
    event: Event,
    sender: broadcast::Sender<FileChange>,
    file_id: &Arc<String>,
) {
    if should_filter_event(&event) {
        return;
    }
    let target_filename = extract_filename(file_id);
    let relevant_paths = filter_relevant_paths(&event, &target_filename);
    if relevant_paths.is_empty() {
        return;
    }
    for path in relevant_paths {
        if !should_process_path(&path) {
            continue;
        }
        if let Some(changes) = detect_file_changes(&path, file_id).await {
            for change in changes {
                let _ = sender.send(change);
            }
        }
    }
}

fn should_filter_event(event: &Event) -> bool {
    use notify::event::ModifyKind;
    matches!(
        &event.kind,
        notify::EventKind::Access(_)
            | notify::EventKind::Modify(ModifyKind::Metadata(_))
            | notify::EventKind::Other
    )
}

fn extract_filename(file_id: &Arc<String>) -> String {
    Path::new(file_id.as_str())
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("")
        .to_string()
}

fn filter_relevant_paths(event: &Event, target_filename: &str) -> Vec<PathBuf> {
    event
        .paths
        .iter()
        .filter(|path| {
            path.file_name()
                .and_then(|f| f.to_str())
                .map_or(false, |name| name == target_filename)
        })
        .cloned()
        .collect()
}

/// Check if path should be processed (debouncing logic)
fn should_process_path(path: &PathBuf) -> bool {
    let mut last_seen = DEBOUNCE_STATE.lock().expect("lock");
    let now = Instant::now();
    if let Some(&last_time) = last_seen.get(path) {
        if now.duration_since(last_time) < std::time::Duration::from_millis(DEBOUNCE_MS) {
            return false;
        }
    }
    last_seen.insert(path.clone(), now);
    true
}

/// Process file changes and return changes to broadcast
async fn detect_file_changes(
    path: &PathBuf,
    file_id: &Arc<String>,
) -> Option<Vec<FileChange>> {
    let new_content = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        tokio::fs::read_to_string(path),
    )
    .await
    .ok()
    .and_then(|r| r.ok())?;
    
    // only use FullContent for very small files (< 1KB)
    if new_content.len() < 1024 {
        return Some(vec![FileChange::FullContent {
            file_id: file_id.to_string(),
            content: new_content,
        }]);
    }
    
    let mut last_content = LAST_CONTENT.lock().expect("lock");
    let old_content = last_content.get(file_id.as_str()).map(String::as_str).unwrap_or("");
    if old_content != new_content {
        let changes = FileChange::create_diff(file_id.as_str(), old_content, &new_content);
        last_content.insert(file_id.to_string(), new_content);
        if !changes.is_empty() {
            Some(changes)
        } else {
            None
        }
    } else {
        None
    }
}

/// Wait for all events to be processed with shorter timeout
pub async fn wait_for_events_processed() {
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!("All events processed");
}
