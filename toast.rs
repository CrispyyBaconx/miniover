use anyhow::Result;
use log::error;
use tauri_winrt_notification::{Duration, Sound, Toast};

use crate::types::Message;

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

pub fn show_error_notification(title: &str, message: &str) -> Result<()> {
    Toast::new(Toast::POWERSHELL_APP_ID)
        .title(title)
        .text1(message)
        .duration(Duration::Short)
        .sound(Some(Sound::SMS))
        .show()?;
    
    Ok(())
}

pub fn show_success_notification(title: &str, message: &str) -> Result<()> {
    Toast::new(Toast::POWERSHELL_APP_ID)
        .title(title)
        .text1(message)
        .duration(Duration::Short)
        .show()?;
    
    Ok(())
}
