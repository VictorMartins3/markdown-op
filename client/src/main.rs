use std::{collections::HashMap, env, path::Path};
use futures_util::StreamExt;
use tokio::{fs, io::{AsyncWriteExt, BufWriter}, time::{sleep, Duration}};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use shared::FileChange;
use shared::protocol::DEFAULT_SERVER_URL;
use url::Url;

const MAX_RECONNECT_ATTEMPTS: u32 = 15;
const INITIAL_RECONNECT_DELAY_MS: u64 = 100;
const MAX_RECONNECT_DELAY_MS: u64 = 2000;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Markdown Mirror Client");
    let client_id = env::args().nth(1).unwrap_or_else(|| "1".to_string());
    let output_dir = env::var("OUTPUT_DIR").unwrap_or_else(|_| "client".to_string());
    println!("Client ID: {}", client_id);
    println!("Output directory: {}", output_dir);
    fs::create_dir_all(&output_dir).await?;
    let mut file_contents = HashMap::new();
    let mut attempt = 0;
    let mut reconnect_delay = INITIAL_RECONNECT_DELAY_MS;
    loop {
        match connect_and_process(&client_id, &output_dir, &mut file_contents).await {
            Ok(_) => {
                println!("Connection closed normally");
                break;
            }
            Err(e) => {
                attempt += 1;
                if attempt >= MAX_RECONNECT_ATTEMPTS {
                    eprintln!("Max reconnection attempts reached. Exiting.");
                    return Err(e);
                }
                let jitter = (rand::random::<u64>() % 100) as u64;
                let delay = (reconnect_delay + jitter).min(MAX_RECONNECT_DELAY_MS);
                eprintln!("Connection error: {}. Reconnecting in {}ms (attempt {}/{})", e, delay, attempt, MAX_RECONNECT_ATTEMPTS);
                sleep(Duration::from_millis(delay)).await;
                reconnect_delay = (reconnect_delay * 2).min(MAX_RECONNECT_DELAY_MS);
            }
        }
    }
    Ok(())
}

async fn connect_and_process(
    client_id: &str,
    output_dir: &str,
    file_contents: &mut HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = Url::parse(DEFAULT_SERVER_URL)?;
    let connect_result = tokio::time::timeout(Duration::from_secs(5), connect_async(url)).await;
    let (ws_stream, _) = match connect_result {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => return Err(Box::new(e)),
        Err(_) => return Err("Connection timeout".into()),
    };
    println!("Connected to server");
    let (_, mut read) = ws_stream.split();
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Err(e) = process_message(&text, client_id, output_dir, file_contents).await {
                    eprintln!("Error processing message: {}", e);
                }
            }
            Ok(Message::Close(_)) => {
                println!("Server closed connection");
                return Ok(());
            }
            Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
            Err(e) => {
                eprintln!("WebSocket error: {}", e);
                return Err(Box::new(e));
            }
            _ => {}
        }
    }
    Ok(())
}

async fn process_message(
    text: &str,
    client_id: &str,
    output_dir: &str,
    file_contents: &mut HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let change: FileChange = serde_json::from_str(text)?;
    match &change {
        FileChange::FullContent { file_id, content } => {
            file_contents.insert(file_id.clone(), content.clone());
            write_file(client_id, output_dir, content).await?;
            println!("Updated file: client/client{}_README.md", client_id);
        }
        FileChange::Diff { file_id, position, delete_count, insert_text } => {
            let content = file_contents.entry(file_id.clone()).or_insert_with(String::new);
            if *position <= content.len() {
                let end = (*position + *delete_count).min(content.len());
                content.replace_range(*position..end, insert_text);
                write_file(client_id, output_dir, content).await?;
                println!("Applied diff to file: client/client{}_README.md", client_id);
            } else {
                eprintln!("Invalid diff position: {} for content length: {}", position, content.len());
            }
        }
    }
    Ok(())
}

async fn write_file(client_id: &str, output_dir: &str, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = Path::new(output_dir).join(format!("client{}_README.md", client_id));
    let file = fs::File::create(&output_path).await?;
    let mut writer = BufWriter::new(file);
    writer.write_all(content.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}
