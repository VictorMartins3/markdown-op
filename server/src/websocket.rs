use tokio::net::{TcpStream, TcpListener};
use tokio::sync::{broadcast, oneshot};
use tokio_tungstenite::{accept_async, tungstenite::{protocol::Message, Error as WsError}, WebSocketStream};
use futures_util::{StreamExt, SinkExt};
use shared::FileChange;

pub struct WebSocketHandler {
    sender: broadcast::Sender<FileChange>,
}

impl WebSocketHandler {
    pub fn new(sender: broadcast::Sender<FileChange>) -> Self {
        Self { sender }
    }

    pub async fn start_server(
        &self,
        addr: String,
        mut shutdown_rx: oneshot::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(&addr).await?;
        println!("WebSocket server listening on ws://{}", addr);
        let sender = self.sender.clone();
        let watched_file = std::env::args().nth(1).unwrap_or_else(|| "README.md".to_string());
        let mut connection_count = 0;

        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, client_addr)) => {
                            connection_count += 1;
                            println!("New connection from: {} (total: {})", client_addr, connection_count);
                            if connection_count > 100 {
                                eprintln!("Too many connections, rejecting: {}", client_addr);
                                continue;
                            }
                            let sender_clone = sender.clone();
                            let watched_file_clone = watched_file.clone();
                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_client(stream, sender_clone, watched_file_clone).await {
                                    eprintln!("Error from client {}: {}", client_addr, e);
                                }
                                println!("Client {} disconnected", client_addr);
                            });
                        }
                        Err(e) => eprintln!("Error accepting connection: {}", e),
                    }
                }
                _ = &mut shutdown_rx => {
                    println!("Received shutdown signal, closing WebSocket server...");
                    break;
                }
            }
        }
        Ok(())
    }

    async fn handle_client(
        stream: TcpStream,
        sender: broadcast::Sender<FileChange>,
        watched_file: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ws_stream = accept_async(stream).await?;
        let (mut write, mut read) = ws_stream.split();
        let mut rx = sender.subscribe();

        Self::send_initial_content(&mut write, &watched_file).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        Self::process_messages(&mut write, &mut read, &mut rx, &watched_file).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn send_initial_content(
        write: &mut futures_util::stream::SplitSink<WebSocketStream<TcpStream>, Message>,
        watched_file: &str,
    ) -> Result<(), WsError> {
        if let Ok(content) = tokio::fs::read_to_string(watched_file).await {
            let change = FileChange::FullContent {
                file_id: watched_file.to_string(),
                content,
            };
            let content = serde_json::to_string(&change).map_err(|e| WsError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
            write.send(Message::Text(content)).await?;
            write.flush().await?;
        }
        Ok(())
    }

    async fn process_messages(
        write: &mut futures_util::stream::SplitSink<WebSocketStream<TcpStream>, Message>,
        read: &mut futures_util::stream::SplitStream<WebSocketStream<TcpStream>>,
        rx: &mut broadcast::Receiver<FileChange>,
        _watched_file: &str,
    ) -> Result<(), WsError> {
        loop {
            tokio::select! {
                msg = read.next() => {
                    if !Self::handle_incoming_message(msg, write).await? {
                        break;
                    }
                }
                change_result = rx.recv() => {
                    if !Self::handle_broadcast(change_result, write).await? {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_incoming_message(
        msg: Option<Result<Message, WsError>>,
        write: &mut futures_util::stream::SplitSink<WebSocketStream<TcpStream>, Message>,
    ) -> Result<bool, WsError> {
        match msg {
            Some(Ok(Message::Close(_))) => {
                let _ = write.send(Message::Close(None)).await;
                Ok(false)
            }
            Some(Ok(Message::Ping(data))) => {
                if write.send(Message::Pong(data)).await.is_err() {
                    return Ok(false);
                }
                Ok(true)
            }
            Some(Ok(Message::Pong(_))) => Ok(true),
            Some(Ok(_)) => Ok(true),
            Some(Err(_)) => Ok(false),
            None => Ok(false),
        }
    }

    async fn handle_broadcast(
        change_result: Result<FileChange, broadcast::error::RecvError>,
        write: &mut futures_util::stream::SplitSink<WebSocketStream<TcpStream>, Message>,
    ) -> Result<bool, WsError> {
        match change_result {
            Ok(change) => {
                let content = serde_json::to_string(&change).map_err(|e| WsError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
                if write.send(Message::Text(content)).await.is_err() {
                    return Ok(false);
                }
                if write.flush().await.is_err() {
                    return Ok(false);
                }
                Ok(true)
            }
            Err(_) => {
                let _ = write.send(Message::Close(None)).await;
                Ok(false)
            }
        }
    }
}
