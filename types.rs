use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct AppState {
    pub config: Config,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub user_key: Option<String>,
    pub secret: Option<String>,
    pub device_id: Option<String>,
    pub start_on_boot: bool,
    pub last_message_id: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            user_key: None,
            secret: None,
            device_id: None,
            start_on_boot: false,
            last_message_id: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub status: i32,
    pub id: String,
    pub secret: String,
    pub request: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceRegisterResponse {
    pub status: i32,
    pub id: String,
    pub request: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: i64,
    pub id_str: String,
    pub message: String,
    pub app: String,
    pub aid: i64,
    pub aid_str: String,
    pub icon: String,
    pub date: i64,
    pub priority: i32,
    pub acked: i32,
    pub umid: i64,
    pub umid_str: String,
    pub title: Option<String>,
    pub url: Option<String>,
    pub url_title: Option<String>,
    pub sound: Option<String>,
    pub html: Option<i32>,
    pub receipt: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessagesResponse {
    pub status: i32,
    pub request: String,
    pub messages: Vec<Message>,
}

#[derive(Debug)]
pub enum Event {
    Quit,
    ToggleStartOnBoot,
    ShowAbout,
    Logout,
}