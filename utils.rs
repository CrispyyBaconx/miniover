use crate::types::Config;
use crate::auth::{login, register_device};
use crate::toast::{show_success_notification, show_error_notification};
use crate::creds::get_credentials;
use anyhow::{Result, Error};
use std::{fs, io::Write, path::{Path, PathBuf}};
use log::{info, error, debug};

#[cfg(windows)]
use auto_launch::AutoLaunch;

const CONFIG_FILENAME: &str = "config.json";

pub fn get_app_config_dir() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("miniover");
    std::fs::create_dir_all(&path).ok();
    path
}

pub fn get_app_paths() -> (std::path::PathBuf, std::path::PathBuf) {
    let config_dir = get_app_config_dir();
    
    let mut log_dir = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    log_dir.push("miniover");
    log_dir.push("logs");
    std::fs::create_dir_all(&log_dir).ok();
    
    (config_dir, log_dir)
}

pub fn save_config(config: &Config, config_dir: &Path) -> Result<()> {
    let config_path = config_dir.join(CONFIG_FILENAME);
    let config_json = serde_json::to_string_pretty(config)?;
    
    let mut file = fs::File::create(config_path)?;
    file.write_all(config_json.as_bytes())?;
    
    Ok(())
}

pub fn load_config(config_dir: &Path) -> Result<Config> {
    let config_path = config_dir.join(CONFIG_FILENAME);
    
    if !config_path.exists() {
        info!("No config file found, using default");
        return Ok(Config::default());
    }
    
    let config_data = fs::read_to_string(config_path)?;
    let config: Config = serde_json::from_str(&config_data)?;
    
    Ok(config)
}

// ============================================================================
// Windows autostart using auto-launch (registry-based)
// ============================================================================

#[cfg(windows)]
pub fn is_autostart_enabled() -> Result<bool> {
    let auto_launch = AutoLaunch::new("Miniover", std::env::current_exe()?.to_str().unwrap(), &[""]);
    Ok(auto_launch.is_enabled()?)
}

#[cfg(windows)]
pub async fn toggle_autorun() -> Result<()> {
    let config_dir = crate::utils::get_app_config_dir();
    let config = load_config(&config_dir)?;
    
    let auto_launch = AutoLaunch::new("Miniover", std::env::current_exe()?.to_str().unwrap(), &[""]);
    
    match (config.start_on_boot, auto_launch.is_enabled()?) {
        (true, false) => auto_launch.enable()?,
        (false, true) => auto_launch.disable()?,
        _ => {}
    }
    
    Ok(())
}

// ============================================================================
// Linux autostart using systemd user service
// ============================================================================

#[cfg(target_os = "linux")]
pub fn is_autostart_enabled() -> Result<bool> {
    use std::process::Command;
    
    let output = Command::new("systemctl")
        .args(["--user", "is-enabled", "miniover.service"])
        .output();
    
    match output {
        Ok(output) => {
            let status = String::from_utf8_lossy(&output.stdout);
            let is_enabled = status.trim() == "enabled";
            debug!("systemd service status: {} (enabled={})", status.trim(), is_enabled);
            Ok(is_enabled)
        }
        Err(e) => {
            debug!("Failed to check systemd service status: {}", e);
            Ok(false)
        }
    }
}

#[cfg(target_os = "linux")]
fn is_service_installed() -> bool {
    // Check if the service file exists in the user's systemd directory
    if let Some(config_dir) = dirs::config_dir() {
        let service_path = config_dir.join("systemd/user/miniover.service");
        if service_path.exists() {
            return true;
        }
    }
    
    // Also check system-wide user services
    let system_service = PathBuf::from("/usr/lib/systemd/user/miniover.service");
    if system_service.exists() {
        return true;
    }
    
    false
}

#[cfg(target_os = "linux")]
pub async fn toggle_autorun() -> Result<()> {
    use std::process::Command;
    
    let config_dir = crate::utils::get_app_config_dir();
    let config = load_config(&config_dir)?;
    let is_enabled = is_autostart_enabled().unwrap_or(false);
    
    match (config.start_on_boot, is_enabled) {
        (true, false) => {
            // Check if service is installed first
            if !is_service_installed() {
                error!("Systemd service not installed");
                show_error_notification(
                    "Service Not Installed",
                    "Please install miniover.service first.\nSee README for instructions."
                )?;
                return Err(Error::msg("Systemd service not installed. Copy miniover.service to ~/.config/systemd/user/"));
            }
            
            // Enable the service
            info!("Enabling systemd user service");
            let output = Command::new("systemctl")
                .args(["--user", "enable", "miniover.service"])
                .output()?;
            
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                error!("Failed to enable service: {}", stderr);
                show_error_notification("Failed to Enable", &format!("Could not enable service: {}", stderr))?;
                return Err(Error::msg(format!("Failed to enable service: {}", stderr)));
            }
        }
        (false, true) => {
            // Disable the service
            info!("Disabling systemd user service");
            let output = Command::new("systemctl")
                .args(["--user", "disable", "miniover.service"])
                .output()?;
            
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                error!("Failed to disable service: {}", stderr);
                show_error_notification("Failed to Disable", &format!("Could not disable service: {}", stderr))?;
                return Err(Error::msg(format!("Failed to disable service: {}", stderr)));
            }
        }
        _ => {}
    }
    
    Ok(())
}

// ============================================================================
// Config initialization
// ============================================================================

pub async fn init_config() -> Result<Config, Error> {
    let (config_dir, _) = get_app_paths();
    let mut config = load_config(&config_dir)?;

    // Load autorun status from system
    config.start_on_boot = is_autostart_enabled().unwrap_or(false);
    
    // Check if login is needed
    if config.user_key.is_none() || config.secret.is_none() || config.device_id.is_none() {
        info!("Login required, showing login dialog");
        
        // Use credential dialog to get email and password
        if let Some((email, password)) = get_credentials().await {
            match login(&email, &password, None).await {
                Ok(login_response) => {
                    info!("Login successful");
                    // Register device
                    match register_device(&login_response.secret).await {
                        Ok(device_response) => {
                            info!("Device registered: {}", device_response.id);
                            
                            // Update config
                            config.user_key = Some(login_response.id);
                            config.secret = Some(login_response.secret);
                            config.device_id = Some(device_response.id);
                            
                            if let Err(e) = save_config(&config, &config_dir) {
                                error!("Failed to save config: {}", e);
                            }
                            
                            // Show success notification
                            show_success_notification("Login Success", "You are now logged in to Pushover").ok();
                        }
                        Err(e) => {
                            error!("Device registration failed: {}", e);
                            show_error_notification("Registration Failed", &format!("Error: {}", e)).ok();
                            return Err(Error::msg(format!("Device registration failed: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    error!("Login failed: {}", e);
                    show_error_notification("Login Failed", &format!("Error: {}", e)).ok();
                    return Err(Error::msg(format!("Login failed: {}", e)));
                }
            }
        } else {
            // User cancelled login
            return Err(Error::msg("Login cancelled"));
        }
    }
    
    Ok(config)
}
