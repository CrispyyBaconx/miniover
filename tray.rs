use crate::types::{Event, AppState};
use crate::toast;
use crate::utils::{get_app_config_dir, get_app_paths, save_config, toggle_autorun};
use anyhow::Result;
use log::{error, info, debug};
use tokio::sync::{mpsc, Mutex};
use tray_item::TrayItem;
use std::sync::Arc;

pub struct TrayContext {
    pub tray: TrayItem,
    pub toggle_startup_menu_item_id: u32,
}

// Main function to consume tray events
pub async fn consume_tray_events(mut rx: mpsc::Receiver<Event>, app_state: Arc<Mutex<AppState>>, mut tray_context: TrayContext) -> Result<()> {
    let config_dir = get_app_config_dir();

    debug!("Tray events consumer started");
    
    while let Some(message) = rx.recv().await {
        debug!("Received event on tray thread: {:?}", message);
        match message {
            Event::Quit => {
                info!("Quitting application");
                toast::show_success_notification("Miniover", "Miniover has been closed successfully")?;
                std::process::exit(0);
            }
            Event::ToggleStartOnBoot => {
                info!("Toggling start on boot");
                let app_state_clone_inner = app_state.clone();
                let config_dir_clone = config_dir.clone();
                
                let mut state = app_state_clone_inner.lock().await;
                state.config.start_on_boot = !state.config.start_on_boot;
                
                if let Err(e) = save_config(&state.config, &config_dir_clone) {
                    error!("Failed to save config: {}", e);
                }
                
                if let Err(e) = toggle_autorun().await {
                    error!("Failed to update autorun: {}", e);
                }

                // update tray menu item state
                let toggle_text = match state.config.start_on_boot {
                    true => "Start on boot [✓]",
                    false => "Start on boot [ ]",
                };
                tray_context.tray.inner_mut().set_menu_item_label(toggle_text, tray_context.toggle_startup_menu_item_id).unwrap();

                // Notify user
                let status = if state.config.start_on_boot { "enabled" } else { "disabled" };
                toast::show_success_notification("Autostart Updated", &format!("Start on boot {}", status)).ok();
            }
            Event::ShowAbout => {
                toast::show_success_notification(
                    "About Miniover",
                    "Miniover v0.1.0\nA minimal Pushover client\n\nVibe Coded by: CrispyyBaconx (& Claude)\nGitHub: github.com/CrispyyBaconx/miniover"
                ).ok();
            }
            Event::Logout => {
                info!("Logging out");
                let app_state_clone_inner = app_state.clone();
                let config_dir_clone = config_dir.clone();
                
                let mut state = app_state_clone_inner.lock().await;
                // Clear credentials
                state.config.user_key = None;
                state.config.secret = None;
                state.config.device_id = None;
                
                if let Err(e) = save_config(&state.config, &config_dir_clone) {
                    error!("Failed to save config during logout: {}", e);
                }
                
                // Notify user
                toast::show_success_notification("Logged Out", "You have been logged out of Pushover").ok();
                
                // App should restart or show login screen
                // For simplicity, just exit and let the user restart
                std::process::exit(0);
            }
            Event::ShowLogs => {
                info!("Showing logs");
                let logs_dir = get_app_paths().1;
                
                // Open logs directory in system file manager
                if let Err(e) = open::that(&logs_dir) {
                    error!("Failed to open logs directory: {}", e);
                }
            }
        }
    }

    error!("Tray event channel closed unexpectedly");
    Ok(())
}
