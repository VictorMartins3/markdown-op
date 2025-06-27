mod watcher;
mod websocket;

use std::sync::Arc;
use tokio::sync::{broadcast, oneshot};
use tokio::signal;
use crate::watcher::FileWatcher;
use crate::websocket::WebSocketHandler;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting Markdown Mirror Server");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let (broadcast_tx, _) = broadcast::channel(1000);
    let broadcast_tx = Arc::new(broadcast_tx);
    let watched_file = std::env::args().nth(1).unwrap_or_else(|| "README.md".to_string());
    let file_id = watched_file.clone();
    let mut watcher = FileWatcher::new();
    watcher.watch_file(file_id, &watched_file, broadcast_tx.as_ref().clone())?;
    println!("Watching file: {}", watched_file);
    let ws_handler = WebSocketHandler::new(broadcast_tx.as_ref().clone());
    let ws_task = tokio::spawn(async move {
        if let Err(e) = ws_handler.start_server("127.0.0.1:3030".to_string(), shutdown_rx).await {
            eprintln!("WebSocket server error: {}", e);
        }
    });
    tokio::select! {
        _ = signal::ctrl_c() => {
            println!("Received Ctrl+C, shutting down...");
            let _ = shutdown_tx.send(());
        }
        _ = ws_task => {
            println!("WebSocket server stopped");
        }
    }
    watcher::wait_for_events_processed().await;
    Ok(())
}
