use crate::types::{DeviceRegisterResponse, LoginResponse};
use anyhow::{anyhow, Result};
use reqwest::Client;

const PUSHOVER_API_URL: &str = "https://api.pushover.net/1";
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