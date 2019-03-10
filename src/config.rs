use std::{fs::File, io::Read, path::PathBuf, time::Duration};

use serde::Deserialize;

use crate::error::Result;
use crate::services::messages::ScheduleMessage;

#[derive(Deserialize, Debug)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub username: String,
    pub password: String,
    pub recipients: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct BroadcastConfig {
    pub email: EmailConfig,
}

#[derive(Deserialize, Debug)]
pub struct FilesystemConfig {
    pub mount: PathBuf,
    pub available_space_alert_above: f32,
}

#[derive(Deserialize, Debug)]
pub struct SystemMonitorConfig {
    pub filesystems: Vec<FilesystemConfig>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct ScheduleConfig {
    pub interval: Duration,
    pub message: ScheduleMessage,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub system_monitor: Option<SystemMonitorConfig>,
    pub schedule: Vec<ScheduleConfig>,
    pub broadcast: BroadcastConfig,
}
impl Config {
    pub fn from_file() -> Result<Self> {
        let mut contents = String::new();

        let mut config_file = File::open("/home/matt/.pulse/config.toml")?;
        config_file.read_to_string(&mut contents)?;

        let config: Self = toml::from_str(&contents)?;

        Ok(config)
    }
}
