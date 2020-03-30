use std::{fs::File, io::Read, path::PathBuf, str::FromStr, sync::Mutex, time::Duration};

use chrono::Local;
use cron::Schedule as CronSchedule;
use lazy_static::lazy_static;
use nytrs::request::{MostPopularPeriod, ShareType};
use serde::Deserialize;

use crate::{
    constants,
    error::Result,
    services::{
        broadcast::{BroadcastEventType, BroadcastMedium},
        scheduler::{ScheduledStreamMessage, ScheduledTaskMessage},
    },
};

lazy_static! {
    static ref CONFIG: Mutex<Option<Config>> = Mutex::new(None);
}

/// Get the current configuration defined in CONFIG
pub fn config() -> Config {
    CONFIG
        .lock()
        .unwrap()
        .clone()
        .expect("Config was accessed before it was initialized")
}

/// Get location of the config file
fn config_file() -> Result<PathBuf> {
    let mut pulse_dir = constants::pulse_directory()?;
    pulse_dir.push("config");
    pulse_dir.set_extension("toml");
    Ok(pulse_dir)
}

/// Initialize the CONFIG object from the config file
pub fn initialize_from_file() -> Result<()> {
    let mut contents = String::new();

    let mut config_file = File::open(config_file()?)?;
    config_file.read_to_string(&mut contents)?;

    let config: Config = toml::from_str(&contents)?;

    initialize_from(config);

    Ok(())
}

pub fn initialize_from(config: Config) {
    *CONFIG.lock().unwrap() = Some(config);
}

#[derive(Clone, Deserialize, Debug)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub username: String,
    pub password: String,
    pub recipients: Vec<String>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct BroadcastConfig {
    pub email: Option<EmailConfig>,
    pub alerts: Vec<AlertConfig>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct FilesystemConfig {
    pub mount: PathBuf,
    pub available_space_alert_above: f64,
}

#[derive(Clone, Deserialize, Debug)]
pub struct SystemMonitorConfig {
    pub filesystems: Vec<FilesystemConfig>,
    pub tick_ms: u64,
}

#[derive(Clone, Deserialize, Debug)]
pub struct NewYorkTimesConfig {
    pub api_key: String,
    pub most_popular_viewed_period: Option<MostPopularPeriod>,
    pub most_popular_emailed_days: Option<MostPopularPeriod>,
    pub most_popular_shared_period: Option<MostPopularPeriod>,
    pub most_popular_shared_mediums: Vec<ShareType>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct NewsConfig {
    pub new_york_times: Option<NewYorkTimesConfig>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct TwitterTerms {
    pub group_name: String,
    pub terms: Vec<String>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct TwitterConfig {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub access_key: String,
    pub access_secret: String,
    pub terms: Vec<TwitterTerms>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
}

#[derive(Clone, Deserialize, Debug)]
pub struct ScheduledStreamConfig {
    pub message: ScheduledStreamMessage,
}

#[derive(Clone, Deserialize, Debug)]
pub struct ScheduledTaskConfig {
    pub cron: String,
    pub message: ScheduledTaskMessage,
}

impl ScheduledTaskConfig {
    pub fn duration_until_next(&self) -> Duration {
        // TODO: validate the cron syntax before it gets here
        let cron_schedule = CronSchedule::from_str(&self.cron).ok().unwrap();
        let now = Local::now();
        let next = cron_schedule.upcoming(Local).next().unwrap();
        let duration_until = next.signed_duration_since(now);
        Duration::from_millis(duration_until.num_milliseconds() as u64)
    }
}

#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AlertType {
    Digest,
    Alarm,
}

#[derive(Clone, Deserialize, Debug)]
pub struct AlertConfig {
    pub alert_interval: Option<Duration>,
    pub event: BroadcastEventType,
    pub mediums: Vec<BroadcastMedium>,
    pub alert_type: AlertType,
}

#[derive(Clone, Deserialize, Debug)]
pub struct Config {
    pub system_monitor: Option<SystemMonitorConfig>,
    pub news: Option<NewsConfig>,
    pub tasks: Vec<ScheduledTaskConfig>,
    pub streams: Vec<ScheduledStreamConfig>,
    pub broadcast: BroadcastConfig,
    pub database: DatabaseConfig,
    pub twitter: Option<TwitterConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            system_monitor: None,
            news: None,
            streams: vec![],
            tasks: vec![],
            broadcast: BroadcastConfig {
                email: None,
                alerts: vec![],
            },
            database: DatabaseConfig {
                host: "localhost".to_string(),
                port: 5432,
                database: "pulse".to_string(),
                username: "postgres".to_string(),
                password: "postgres".to_string(),
            },
            twitter: None,
        }
    }
}
