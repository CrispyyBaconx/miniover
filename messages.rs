use crate::types::{Config, Message, MessagesResponse, Event};
use crate::toast;
use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use reqwest::Client;
use std::path::Path;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time;
use tokio_tungstenite::{
    connect_async_tls_with_config, 
    tungstenite::protocol::Message as WsMessage,
    Connector,
    MaybeTlsStream,
    WebSocketStream
};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use crate::utils::{get_app_config_dir, save_config, load_config};

const PUSHOVER_API_URL: &str = "https://api.pushover.net/1";
const PUSHOVER_WS_URL: &str = "wss://client.pushover.net/push";
const RECONNECT_DELAY_MS: u64 = 5000;

// Function to download messages from Pushover API
pub async fn download_messages(secret: &str, device_id: &str) -> Result<Vec<Message>> {
    let client = Client::new();
    let url = format!(
        "{}/messages.json?secret={}&device_id={}",
        PUSHOVER_API_URL, secret, device_id
    );

    debug!("Downloading messages from: {}", url);
    
    let res = client.get(&url).send().await?;
    
    if !res.status().is_success() {
        return Err(anyhow!("Failed to download messages: {}", res.status()));
    }

    debug!("Attempting to parse messages response");
    let text = res.text().await?;
    debug!("Messages response: {}", text);
    
    let messages_response: MessagesResponse = serde_json::from_str(&text)?;

    debug!("Messages response: {:?}", messages_response);
    
    if messages_response.status != 1 {
        return Err(anyhow!("Message download failed with status {}", messages_response.status));
    }
    
    Ok(messages_response.messages)
}

// Function to delete messages from Pushover API
pub async fn delete_messages(secret: &str, device_id: &str, message_id: &str) -> Result<()> {
    let client = Client::new();
    let url = format!(
        "{}/devices/{}/update_highest_message.json",
        PUSHOVER_API_URL, device_id
    );
    
    let form = [
        ("secret", secret),
        ("message", message_id),
    ];
    
    let res = client.post(&url).form(&form).send().await?;
    
    if !res.status().is_success() {
        return Err(anyhow!("Failed to delete messages: {}", res.status()));
    }
    
    let json: serde_json::Value = res.json().await?;
    
    if json["status"] != 1 {
        return Err(anyhow!("Message deletion failed"));
    }
    
    Ok(())
}

// Function to acknowledge emergency priority messages
pub async fn acknowledge_emergency(secret: &str, receipt: &str) -> Result<()> {
    let client = Client::new();
    let url = format!(
        "{}/receipts/{}/acknowledge.json",
        PUSHOVER_API_URL, receipt
    );
    
    let form = [
        ("secret", secret),
    ];
    
    let res = client.post(&url).form(&form).send().await?;
    
    if !res.status().is_success() {
        return Err(anyhow!("Failed to acknowledge emergency message: {}", res.status()));
    }
    
    let json: serde_json::Value = res.json().await?;
    
    if json["status"] != 1 {
        return Err(anyhow!("Acknowledge emergency failed"));
    }
    
    Ok(())
}

// Function to process incoming messages
async fn process_messages(config: &mut Config, config_dir: &Path) -> Result<()> {
    // Guard against missing credentials
    if config.secret.is_none() || config.device_id.is_none() {
        return Err(anyhow!("Missing secret or device ID"));
    }
    
    let secret = config.secret.as_ref().unwrap();
    let device_id = config.device_id.as_ref().unwrap();
    
    // Download messages
    debug!("Downloading messages");
    let messages = download_messages(secret, device_id).await?;
    
    if messages.is_empty() {
        return Ok(());
    }
    
    // Get highest message ID
    debug!("Getting highest message ID");
    let highest_message = messages.iter().max_by_key(|m| m.id).unwrap();
    
    // Process each message
    debug!("Processing messages");
    for message in &messages {
        // Show notification
        if let Err(e) = toast::show_notification(message) {
            error!("Failed to show notification: {}", e);
        }
        
        // If emergency priority, acknowledge it
        if message.priority >= 2 && message.acked == 0 {
            if let Some(receipt) = &message.receipt {
                if let Err(e) = acknowledge_emergency(secret, receipt).await {
                    error!("Failed to acknowledge emergency message: {}", e);
                }
            }
        }
    }
    
    // Delete messages from server
    if let Err(e) = delete_messages(secret, device_id, &highest_message.id_str).await {
        error!("Failed to delete messages: {}", e);
    } else {
        // Update config with last message ID
        config.last_message_id = Some(highest_message.id_str.clone());
        save_config(config, config_dir)?;
    }
    
    Ok(())
}

// Function to establish WebSocket connection and handle messages
async fn connect_websocket(config: &Config) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    let request = PUSHOVER_WS_URL.into_client_request()?;
    
    let (ws_stream, _) = connect_async_tls_with_config(
        request,
        None,
        false,
        Some(Connector::NativeTls(
            native_tls::TlsConnector::builder().build()?,
        )),
    )
    .await?;
    
    info!("WebSocket connection established");
    
    // Send login message
    let mut ws_stream = ws_stream;
    let login_msg = format!(
        "login:{}:{}\n",
        config.device_id.as_ref().unwrap(),
        config.secret.as_ref().unwrap()
    );
    
    ws_stream.send(WsMessage::Text(login_msg.into())).await?;
    
    Ok(ws_stream)
}

// Main function to consume message feed
pub async fn consume_message_feed(tx: mpsc::Sender<Event>) -> Result<()> {
    let config_dir = get_app_config_dir();
    let mut config = load_config(&config_dir)?;
    
    // Check if we're logged in
    if config.secret.is_none() || config.device_id.is_none() {
        panic!("Not logged in, login flow was disrupted");
        // ! we should be logged in by now, so this is a bug
    }
    
    // Process any existing messages first (but silently)
    if let Err(e) = process_messages(&mut config, &config_dir).await {
        error!("Failed to process existing messages: {}", e);
    }
    
    // Main WebSocket loop
    loop {
        // Make sure we have credentials
        if config.secret.is_none() || config.device_id.is_none() {
            error!("Missing credentials for WebSocket connection");
            time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
            continue;
        }
        
        match connect_websocket(&config).await {
            Ok(mut ws_stream) => {
                info!("Connected to Pushover WebSocket");
                
                while let Some(msg) = ws_stream.next().await {
                    match msg {
                        Ok(WsMessage::Text(text)) => {
                            debug!("Received text message: {}", text);
                        }
                        Ok(WsMessage::Binary(binary)) => {
                            debug!("Received binary message: {:?}", binary);
                            // Convert binary to string and process commands
                            if binary.len() == 1 {
                                let command = binary[0] as char;
                                match command {
                                    '#' => {
                                        // Keep-alive packet, no response needed
                                        debug!("Received keep-alive packet");
                                    }
                                    '!' => {
                                        // New message arrived
                                        info!("New message notification received");
                                        if let Err(e) = process_messages(&mut config, &config_dir).await {
                                            error!("Failed to process messages: {}", e);
                                        }
                                    }
                                    'R' => {
                                        // Reload request
                                        info!("Reload request received, reconnecting...");
                                        break;
                                    }
                                    'E' => {
                                        // Error
                                        error!("Permanent error received, need to re-login");
                                        config.secret = None;
                                        config.device_id = None;
                                        if let Err(e) = save_config(&config, &config_dir) {
                                            error!("Failed to save config: {}", e);
                                        }
                                        tx.send(Event::Logout).await?;
                                        break;
                                    }
                                    'A' => {
                                        // Session closed
                                        warn!("Session closed, device logged in elsewhere");
                                        config.secret = None;
                                        config.device_id = None;
                                        if let Err(e) = save_config(&config, &config_dir) {
                                            error!("Failed to save config: {}", e);
                                        }
                                        tx.send(Event::Logout).await?;
                                        // ! maybe add a toast notification here saying "Session closed, device logged in elsewhere" or something
                                        break;
                                    }
                                    _ => {
                                        warn!("Unknown WebSocket command: {}", command);
                                    }
                                }
                            } else {
                                debug!("As string: {:?}", String::from_utf8_lossy(&binary));
                            }
                        }
                        Ok(WsMessage::Ping(_)) => {
                            debug!("Received ping");
                        }
                        Ok(WsMessage::Pong(_)) => {
                            debug!("Received pong");
                        }
                        Ok(WsMessage::Close(close)) => {
                            info!("WebSocket closed: {:?}", close);
                            break;
                        }
                        Ok(WsMessage::Frame(frame)) => {
                            debug!("Received frame: {:?}", frame);
                        }
                        Err(e) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to connect to WebSocket: {}", e);
            }
        }
        
        // Reconnect delay
        info!("Reconnecting in {} ms", RECONNECT_DELAY_MS);
        time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
    }
}