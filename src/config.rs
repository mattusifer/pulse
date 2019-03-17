use std::{fs::File, io::Read, path::PathBuf, sync::Mutex, time::Duration};

use lazy_static::lazy_static;
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
pub struct ScheduleConfig {
    pub schedule_interval: Duration,
    pub message: ScheduleMessage,
}

#[derive(Clone, Deserialize, Debug)]
pub struct AlertConfig {
    pub alert_interval: Duration,
    pub event: BroadcastEventType,
    pub mediums: Vec<BroadcastMedium>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct Config {
    pub system_monitor: Option<SystemMonitorConfig>,
    pub schedules: Vec<ScheduleConfig>,
    pub broadcast: BroadcastConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            system_monitor: None,
            schedules: vec![],
            broadcast: BroadcastConfig {
                email: None,
                alerts: vec![],
            },
        }
    }
}

#[macro_use]
#[cfg(test)]
pub mod test {
    use lazy_static::lazy_static;
    use std::sync::{Mutex, MutexGuard, PoisonError};

    // run the given block with the given config initialized into the
    // global config object, ensuring that no other threads modify the
    // config during execution
    #[macro_export]
    macro_rules! run_with_config {
        ($config:expr, $test_block:expr) => {
            use crate::config::{self, test};

            let _lock = test::lock_config();
            config::initialize_from($config);

            $test_block
        };
    }

    lazy_static! {
        static ref LOCK: Mutex<()> = Mutex::new(());
    }

    pub fn lock_config<'a>(
    ) -> Result<MutexGuard<'a, ()>, PoisonError<MutexGuard<'a, ()>>> {
        LOCK.lock()
    }
}
