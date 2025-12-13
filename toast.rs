use anyhow::Result;
use log::error;

use crate::types::Message;

// ============================================================================
// Windows implementation using tauri-winrt-notification
// ============================================================================

#[cfg(windows)]
use tauri_winrt_notification::{Duration, Sound, Toast};

#[cfg(windows)]
pub fn show_notification(message: &Message) -> Result<()> {
    let title = match &message.title {
        Some(title) if !title.is_empty() => title,
        _ => &message.app
    };

    let action_on_click = match &message.url {
        Some(url) if !url.is_empty() => Some(url.clone()),
        _ => None,
    };

    let mut notification = Toast::new(Toast::POWERSHELL_APP_ID) 
        .title(title)
        .text1(&message.message)
        .duration(Duration::Short);

    // Set sound based on message.sound if available
    if message.priority >= 1 {
        notification = notification.sound(Some(Sound::SMS));
    }

    // Add click action if URL is available
    if let Some(url) = action_on_click {
        notification = notification.on_activated(move |_| {
            match open::that(&url) {
                Ok(_) => Ok(()),
                Err(e) => {
                    error!("Failed to open URL: {}", e);
                    Err(tauri_winrt_notification::Error::Io(e))
                }
            }
        });
    }

    // Show the notification
    notification.show()?;
    
    Ok(())
}

#[cfg(windows)]
pub fn show_error_notification(title: &str, message: &str) -> Result<()> {
    Toast::new(Toast::POWERSHELL_APP_ID)
        .title(title)
        .text1(message)
        .duration(Duration::Short)
        .sound(Some(Sound::SMS))
        .show()?;
    
    Ok(())
}

#[cfg(windows)]
pub fn show_success_notification(title: &str, message: &str) -> Result<()> {
    Toast::new(Toast::POWERSHELL_APP_ID)
        .title(title)
        .text1(message)
        .duration(Duration::Short)
        .show()?;
    
    Ok(())
}

// ============================================================================
// Linux implementation using notify-rust
// ============================================================================

#[cfg(target_os = "linux")]
use notify_rust::{Notification, Urgency};

#[cfg(target_os = "linux")]
pub fn show_notification(message: &Message) -> Result<()> {
    let title = match &message.title {
        Some(title) if !title.is_empty() => title,
        _ => &message.app
    };

    let urgency = if message.priority >= 2 {
        Urgency::Critical
    } else if message.priority >= 1 {
        Urgency::Normal
    } else {
        Urgency::Low
    };

    let mut notification = Notification::new();
    notification
        .summary(title)
        .body(&message.message)
        .appname("Miniover")
        .urgency(urgency);

    // Add click action if URL is available
    if let Some(url) = &message.url {
        if !url.is_empty() {
            notification.action("open", "Open URL");
            let url_clone = url.clone();
            
            // Show notification and spawn detached thread for action handling
            // This avoids blocking the Tokio runtime thread
            let handle = notification.show()?;
            std::thread::spawn(move || {
                handle.wait_for_action(|action| {
                    if action == "open" {
                        if let Err(e) = open::that(&url_clone) {
                            error!("Failed to open URL: {}", e);
                        }
                    }
                });
            });
            return Ok(());
        }
    }

    notification.show()?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn show_error_notification(title: &str, message: &str) -> Result<()> {
    Notification::new()
        .summary(title)
        .body(message)
        .appname("Miniover")
        .urgency(Urgency::Critical)
        .show()?;
    
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn show_success_notification(title: &str, message: &str) -> Result<()> {
    Notification::new()
        .summary(title)
        .body(message)
        .appname("Miniover")
        .urgency(Urgency::Normal)
        .show()?;
    
    Ok(())
}
