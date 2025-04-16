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
use std::sync::Mutex as StdMutex;

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
    
    // This will be our single event channel with multiple senders
    let (tokio_tx, tokio_rx) = mpsc::channel::<Event>(100);
    
    // Initialize app state
    let app_state = Arc::new(Mutex::new(AppState {
        config,
    }));
        
    debug!("App state: {:?}", app_state);

    // We'll use a direct std::thread to handle bridge events 
    // This ensures we keep a direct thread for processing UI callbacks
    let tokio_tx_clone = tokio_tx.clone();
    let (std_tx, std_rx) = std_mpsc::channel::<Event>();
    
    // Wrap the tokio sender in an Arc<Mutex> so it can be shared safely across threads
    let tokio_tx_for_thread = Arc::new(StdMutex::new(tokio_tx_clone));
    let tokio_tx_clone_for_thread = tokio_tx_for_thread.clone();
    
    // Spawn a std::thread to bridge events (this is different from tokio::spawn)
    std::thread::spawn(move || {
        info!("Bridge thread started");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");
        
        runtime.block_on(async {
            while let Ok(event) = std_rx.recv() {
                debug!("Bridge thread received event: {:?}", event);
                
                // Get the tokio sender from the mutex
                let sender = tokio_tx_clone_for_thread.lock().unwrap();
                match sender.send(event).await {
                    Ok(_) => debug!("Bridge successfully sent event to tokio channel"),
                    Err(e) => error!("Bridge failed to send event: {}", e),
                }
            }
            
            error!("Bridge thread receiver closed unexpectedly");
        });
    });

    // Create test event sender to verify channel works
    let test_tx = tokio_tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        debug!("Sending test event");
        
        // Send a test event directly on the tokio channel
        match test_tx.send(Event::ShowAbout).await {
            Ok(_) => debug!("Test event sent successfully"),
            Err(e) => error!("Failed to send test event: {}", e),
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
        debug!("Toggle startup clicked");
        match toggle_startup_tx.send(Event::ToggleStartOnBoot) {
            Ok(_) => debug!("Toggle startup event sent successfully"),
            Err(e) => error!("Failed to send toggle startup event: {:?}", e),
        }
    })?;

    debug!("Toggle startup menu item added successfully");

    tray.inner_mut().add_separator()?;

    let about_tx = std_tx.clone();
    tray.add_menu_item("About", move || {
        debug!("About clicked");
        match about_tx.send(Event::ShowAbout) {
            Ok(_) => debug!("About event sent successfully"),
            Err(e) => error!("Failed to send about event: {:?}", e),
        }
    })?;

    debug!("About menu item added successfully");

    let quit_tx = std_tx.clone();
    tray.add_menu_item("Quit", move || {
        debug!("Quit clicked");
        match quit_tx.send(Event::Quit) {
            Ok(_) => debug!("Quit event sent successfully"),
            Err(e) => error!("Failed to send quit event: {:?}", e),
        }
    })?;

    debug!("Quit menu item added successfully");

    let logout_tx = std_tx.clone();
    tray.add_menu_item("Logout", move || {
        debug!("Logout clicked");
        match logout_tx.send(Event::Logout) {
            Ok(_) => debug!("Logout event sent successfully"),
            Err(e) => error!("Failed to send logout event: {:?}", e),
        }
    })?;

    debug!("Logout menu item added successfully");
    
    info!("Tray icon created successfully");
        
    // Spawn message handling with its own channel
    let message_handle = tokio::spawn(messages::consume_message_feed());
    let tray_handle = tokio::spawn(tray::consume_tray_events(tokio_rx, app_state.clone()));
    
    // Wait for tasks to complete
    tokio::select! {
        result = message_handle => {
            error!("Message handler exited: {:?}", result);
            Err(anyhow::anyhow!("Message handler exited unexpectedly"))
        },
        result = tray_handle => {
            error!("Tray handler exited: {:?}", result);
            Err(anyhow::anyhow!("Tray handler exited unexpectedly"))
        }
    }
}