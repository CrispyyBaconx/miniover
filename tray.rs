use crate::types::{Event, AppState};
use crate::toast;
use crate::utils::{get_app_config_dir, save_config, check_for_autorun};
use anyhow::Result;
use log::{error, info};
use tokio::sync::mpsc;
use std::sync::Arc;
use tokio::sync::Mutex;

// Main function to consume tray events
pub async fn consume_tray_events(mut rx: mpsc::Receiver<Event>, app_state: Arc<Mutex<AppState>>) -> Result<()> {
    let config_dir = get_app_config_dir();
    
    while let Some(message) = rx.recv().await {
        match message {
            Event::Quit => {
                info!("Quitting application");
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
                
                if let Err(e) = check_for_autorun().await {
                    error!("Failed to update autorun: {}", e);
                }
                
                // Notify user
                let status = if state.config.start_on_boot { "enabled" } else { "disabled" };
                toast::show_success_notification("Autostart Updated", &format!("Start on boot {}", status)).ok();
            }
            Event::ShowAbout => {
                toast::show_success_notification(
                    "About Miniover",
                    "Miniover v0.1.0\nA minimal Pushover client for Windows\n\nVibe Coded by: Claude (and a bit of dev by me - CrispyyBaconx)\nGitHub: github.com/CrispyyBaconx/miniover"
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
        }
    };

    Ok(())
}