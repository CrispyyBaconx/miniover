// #![windows_subsystem = "windows"]

/**
 * Miniover - A minimal Pushover client for Windows
 * - System tray integration
 * - Windows toast notifications for Pushover messages
 * - Auto-start on Windows boot option
 */

mod auth;
mod messages;
mod toast;
mod types;
mod creds;
mod utils;
mod tray;

use tokio::sync::mpsc;
use anyhow::{Result, Error};
use ftail::Ftail;
use log::{debug, info, error, LevelFilter};
use std::sync::Arc;
use tokio::sync::Mutex;
use types::{Event, AppState};
use tray_item::{IconSource, TrayItem};
use utils::{get_app_paths, init_config};
use std::sync::mpsc as std_mpsc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Get application paths
    let (config_dir, log_dir) = get_app_paths();
    
    // Initialize logging with concrete path
    Ftail::new()
        .console(LevelFilter::Debug)
        .daily_file(log_dir.to_str().unwrap_or("logs"), LevelFilter::Debug) 
        .max_file_size(1024 * 1024 * 10) // 10MB
        .retention_days(2) // 2 days
        .init()?;
    
    info!("Miniover starting up");
    info!("Config directory: {:?}", config_dir);
    info!("Log directory: {:?}", log_dir);
    
    // Initialize config and handle login
    let config = match init_config().await {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to initialize: {}", e);
            return Err(e);
        }
    };
    
    // Create mpsc channel for communication
    let (tx, rx) = mpsc::channel::<Event>(100);
    
    // Initialize app state
    let app_state = Arc::new(Mutex::new(AppState {
        config,
    }));
        
    debug!("App state: {:?}", app_state);

    // Create a standard (non-tokio) channel for tray events
    let (std_tx, std_rx) = std_mpsc::channel::<Event>();

    // Spawn a task to bridge the standard channel to the tokio channel
    let bridge_tx = tx.clone();
    tokio::spawn(async move {
        while let Ok(event) = std_rx.recv() {
            debug!("Received event: {:?}", event);
            if let Err(e) = bridge_tx.send(event).await {
                error!("Failed to bridge tray event: {}", e);
            }
        }
    });

    // try to create the tray icon source
    let icon_source = IconSource::Resource("app-icon");

    // Create menu
    let mut tray = TrayItem::new(
        "Miniover",
        icon_source,
    )?;

    debug!("Tray created successfully");

    // Create menu items
    let state = app_state.lock().await.config.clone();

    // Create text for menu items
    let toggle_text = if state.start_on_boot {
        "Start on boot [✓]"
    } else {
        "Start on boot [ ]"
    };

    let toggle_startup_tx = std_tx.clone();
    tray.add_menu_item(toggle_text, move || {
        toggle_startup_tx.send(Event::ToggleStartOnBoot).unwrap();
    })?;

    debug!("Toggle startup menu item added successfully");

    tray.inner_mut().add_separator()?;

    let about_tx = std_tx.clone();
    tray.add_menu_item("About", move || {
        about_tx.send(Event::ShowAbout).unwrap();
    })?;

    debug!("About menu item added successfully");

    let quit_tx = std_tx.clone();
    tray.add_menu_item("Quit", move || {
        quit_tx.send(Event::Quit).unwrap();
    })?;

    debug!("Quit menu item added successfully");

    let logout_tx = std_tx.clone();
    tray.add_menu_item("Logout", move || {
        logout_tx.send(Event::Logout).unwrap();
    })?;

    debug!("Logout menu item added successfully");
    
    info!("Tray icon created successfully");
        
    // Spawn message handling
    let message_handle = tokio::spawn(messages::consume_message_feed(tx.clone()));
    let tray_handle = tokio::spawn(tray::consume_tray_events(rx, app_state.clone()));
    
    // Wait for all tasks to complete (which they won't unless there's an error)
    tokio::try_join!(
        async { message_handle.await.unwrap() },
        async { tray_handle.await.unwrap() }
    )?;

    Ok(())
}