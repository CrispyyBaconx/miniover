use tray_icon::Icon;
use tokio::sync::mpsc;
use anyhow::{Result, Error};
use ftail::Ftail;
use log::LevelFilter;
use std::sync::Arc;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder,
};
use tokio::sync::Mutex;
use log::{info, error};

// Local modules
mod auth;
mod messages;
mod toast;
mod types;
mod creds;
use types::{Config, TrayMessage};

// UI state struct
struct AppState {
    config: Config,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // init logging
    Ftail::new()
        .console(LevelFilter::Debug)
        .daily_file("logs", LevelFilter::Info) 
        .max_file_size(1024 * 1024 * 10) // 10MB
        .retention_days(2) // 2 days
        .init()?;

    // Load or create config
    let config_dir = types::get_app_config_dir();
    let config = auth::load_config(&config_dir)?;
    
    // Create mpsc channel for communication
    let (tx, mut rx) = mpsc::channel::<TrayMessage>(100);
    
    // Initialize app state
    let app_state = Arc::new(Mutex::new(AppState {
        config,
    }));
    
    let app_state_clone = app_state.clone();
    let tx_clone = tx.clone();
    
    // Create menu and capture IDs as strings before creating menu items
    let tray_menu = Menu::new();
    
    // Create menu items
    let login_item = MenuItem::new("Login", true, None);
    let toggle_startup_item = MenuItem::new("Start on boot", true, None);
    let about_item = MenuItem::new("About", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    
    // Get string representations of IDs
    let login_id_str = format!("{:?}", login_item.id());
    let toggle_id_str = format!("{:?}", toggle_startup_item.id());
    let about_id_str = format!("{:?}", about_item.id());
    let quit_id_str = format!("{:?}", quit_item.id());
    
    tray_menu.append(&login_item)?;
    tray_menu.append(&toggle_startup_item)?;
    tray_menu.append(&about_item)?;
    tray_menu.append(&quit_item)?;
    
    // Create tray icon
    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_icon(Icon::from_path("./icon.ico", None)?)
        .with_tooltip("Miniover - Pushover Client")
        .build()?;
    
    // Set up menu event channel 
    let menu_channel = MenuEvent::receiver();
    let tx_menu = tx.clone();
    
    // Spawn message handling
    let message_handle = tokio::spawn(messages::consume_message_feed(tx.clone()));
    let autorun_handle = tokio::spawn(auth::check_for_autorun());
    
    // Main event loop handling messages from the channel
    let message_handler = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            match message {
                TrayMessage::Quit => {
                    info!("Quitting application");
                    std::process::exit(0);
                }
                TrayMessage::ToggleStartOnBoot => {
                    let app_state_clone_inner = app_state_clone.clone();
                    let config_dir_clone = config_dir.clone();
                    
                    let mut state = app_state_clone_inner.lock().await;
                    state.config.start_on_boot = !state.config.start_on_boot;
                    
                    if let Err(e) = auth::save_config(&state.config, &config_dir_clone) {
                        error!("Failed to save config: {}", e);
                    }
                    
                    if let Err(e) = auth::check_for_autorun().await {
                        error!("Failed to update autorun: {}", e);
                    }
                    
                    // Notify user
                    let status = if state.config.start_on_boot { "enabled" } else { "disabled" };
                    toast::show_success_notification("Autostart Updated", &format!("Start on boot {}", status)).ok();
                }
                TrayMessage::ShowLogin => {
                    info!("Showing login dialog");
                    let app_state_clone_inner = app_state_clone.clone();
                    let config_dir_clone = config_dir.clone();
                    let tx_clone_inner = tx_clone.clone();
                    
                    tokio::spawn(async move {
                        // Use our credential dialog to get email and password
                        if let Some((email, password)) = creds::get_credentials().await {
                            match auth::login(&email, &password, None).await {
                                Ok(login_response) => {
                                    info!("Login successful");
                                    // Register device
                                    match auth::register_device(&login_response.secret).await {
                                        Ok(device_response) => {
                                            info!("Device registered: {}", device_response.id);
                                            
                                            // Update config
                                            let mut state = app_state_clone_inner.lock().await;
                                            state.config.user_key = Some(login_response.id);
                                            state.config.secret = Some(login_response.secret);
                                            state.config.device_id = Some(device_response.id);
                                            
                                            if let Err(e) = auth::save_config(&state.config, &config_dir_clone) {
                                                error!("Failed to save config: {}", e);
                                            }
                                            
                                            // Notify success
                                            toast::show_success_notification("Login Success", "You are now logged in to Pushover").ok();
                                            
                                            // Restart message consumer
                                            tx_clone_inner.send(TrayMessage::Quit).await.ok();
                                        }
                                        Err(e) => {
                                            error!("Device registration failed: {}", e);
                                            toast::show_error_notification("Registration Failed", &format!("Error: {}", e)).ok();
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Login failed: {}", e);
                                    toast::show_error_notification("Login Failed", &format!("Error: {}", e)).ok();
                                }
                            }
                        }
                    });
                }
                TrayMessage::ShowAbout => {
                    toast::show_success_notification(
                        "About Miniover",
                        "Miniover v0.1.0\nA minimal Pushover client for Windows\n\nVibe Coded by: Claude (and a bit of dev by me - CrispyyBaconx)\nGitHub: github.com/CrispyyBaconx/miniover"
                    ).ok();
                }
            }
        }
    });
    
    // Create a thread to handle menu events
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        while let Ok(event) = menu_channel.recv() {
            let event_id_str = format!("{:?}", event.id);
            
            let message = if event_id_str == login_id_str {
                Some(TrayMessage::ShowLogin)
            } else if event_id_str == toggle_id_str {
                Some(TrayMessage::ToggleStartOnBoot)
            } else if event_id_str == about_id_str {
                Some(TrayMessage::ShowAbout)
            } else if event_id_str == quit_id_str {
                Some(TrayMessage::Quit)
            } else {
                None
            };
            
            if let Some(msg) = message {
                rt.block_on(async {
                    tx_menu.send(msg).await.ok();
                });
            }
        }
    });
    
    // Wait for all tasks to complete (which they won't unless there's an error)
    let _ = tokio::try_join!(
        message_handle,
        autorun_handle, 
        message_handler
    );

    Ok(())
}
