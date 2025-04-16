use crate::types::Config;
use crate::auth::{login, register_device};
use crate::toast::{show_success_notification, show_error_notification};
use crate::creds::get_credentials;
use anyhow::{Result, Error};
use std::{fs, io::Write, path::{Path, PathBuf}};
use log::{info, error};

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

pub async fn check_for_autorun() -> Result<()> {
    use auto_launch::AutoLaunchBuilder;
    
    let config_dir = crate::utils::get_app_config_dir();
    let config = load_config(&config_dir)?;
    
    let auto_launch = AutoLaunchBuilder::new()
        .set_app_name("Miniover")
        .set_app_path(std::env::current_exe()?.to_str().unwrap())
        .build()?;
    
    match (config.start_on_boot, auto_launch.is_enabled()?) {
        (true, false) => auto_launch.enable()?,
        (false, true) => auto_launch.disable()?,
        _ => {}
    }
    
    Ok(())
}

pub async fn init_config() -> Result<Config, Error> {
    let (config_dir, _) = get_app_paths();
    let mut config = load_config(&config_dir)?;
    
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
    
    // Ensure autorun is set correctly
    if let Err(e) = check_for_autorun().await {
        error!("Failed to check autorun: {}", e);
    }
    
    Ok(config)
}
