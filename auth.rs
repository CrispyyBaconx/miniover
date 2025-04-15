use crate::types::{Config, DeviceRegisterResponse, LoginResponse};
use anyhow::{anyhow, Result};
use log::info;
use reqwest::Client;
use std::fs;
use std::io::Write;
use std::path::Path;

const PUSHOVER_API_URL: &str = "https://api.pushover.net/1";
const CONFIG_FILENAME: &str = "config.json";
const DEVICE_NAME: &str = "miniover_client";

pub async fn login(email: &str, password: &str, twofa: Option<&str>) -> Result<LoginResponse> {
    let client = Client::new();
    let mut form = vec![
        ("email", email),
        ("password", password),
    ];
    
    if let Some(code) = twofa {
        form.push(("twofa", code));
    }
    
    let res = client
        .post(&format!("{}/users/login.json", PUSHOVER_API_URL))
        .form(&form)
        .send()
        .await?;

    if res.status() == reqwest::StatusCode::PRECONDITION_FAILED {
        return Err(anyhow!("Two-factor authentication required: {}", res.text().await?));
    } else if !res.status().is_success() {
        return Err(anyhow!("Login failed: {}", res.status()));
    }

    let login_response: LoginResponse = res.json().await?;
    if login_response.status != 1 {
        return Err(anyhow!("Login failed with status {}", login_response.status));
    }

    Ok(login_response)
}

pub async fn register_device(secret: &str) -> Result<DeviceRegisterResponse> {
    let client = Client::new();
    let form = [
        ("secret", secret),
        ("name", DEVICE_NAME),
        ("os", "O"),
    ];
    
    let res = client
        .post(&format!("{}/devices.json", PUSHOVER_API_URL))
        .form(&form)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(anyhow!("Device registration failed: {}", res.status()));
    }

    let device_response: DeviceRegisterResponse = res.json().await?;
    if device_response.status != 1 {
        return Err(anyhow!("Device registration failed with status {}", device_response.status));
    }

    Ok(device_response)
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
    
    let config_dir = crate::types::get_app_config_dir();
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
