use std::{fs::File, io::Read, path::PathBuf, sync::Mutex, time::Duration};

use lazy_static::lazy_static;
use nytrs::request::{MostPopularPeriod, ShareType};
use serde::Deserialize;

use crate::constants;
use crate::error::Result;
use crate::services::messages::{
    BroadcastEventType, BroadcastMedium, ScheduleMessage,
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

pub fn initialize_from<'a>(config: Config) -> () {
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
    pub available_space_alert_above: f32,
}

#[derive(Clone, Deserialize, Debug)]
pub struct SystemMonitorConfig {
    pub filesystems: Vec<FilesystemConfig>,
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
pub struct SchedulerConfig {
    pub schedules: Vec<ScheduleConfig>,
    pub tick_ms: u32,
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
pub struct ScheduleConfig {
    pub cron: Option<String>,
    pub message: ScheduleMessage,
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
    pub scheduler: SchedulerConfig,
    pub broadcast: BroadcastConfig,
    pub database: DatabaseConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            system_monitor: None,
            news: None,
            scheduler: SchedulerConfig {
                schedules: vec![],
                tick_ms: 5000,
            },
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
        }
    }
}

#[macro_use]
#[cfg(test)]
pub mod test {
    use lazy_static::lazy_static;
    use std::sync::{Mutex, MutexGuard, PoisonError};

    lazy_static! {
        static ref LOCK: Mutex<()> = Mutex::new(());
    }

    pub fn lock_config<'a>(
    ) -> Result<MutexGuard<'a, ()>, PoisonError<MutexGuard<'a, ()>>> {
        LOCK.lock()
    }

    // run the given block with the given config initialized into the
    // global config object, ensuring that no other threads modify the
    // config during execution
    #[macro_export]
    macro_rules! run_with_config {
        ($config:expr, $test_block:expr) => {{
            use crate::config::{self, test};

            let _lock = test::lock_config();
            config::initialize_from($config);

            $test_block
        }};
    }
}
