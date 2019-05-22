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

#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub username: String,
    pub password: String,
    pub recipients: Vec<String>,
}

#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
pub struct BroadcastConfig {
    pub email: Option<EmailConfig>,
    pub alerts: Vec<AlertConfig>,
}

#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
pub struct CommandsConfig {
    pub commands: Vec<CommandConfig>,
}

#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
pub struct CommandConfig {
    pub command_id: String,
    pub command: String,
    pub alert: bool,
}

#[derive(Clone, Deserialize, Debug, PartialEq)]
pub struct FilesystemConfig {
    pub mount: PathBuf,
    pub available_space_alert_above: f32,
}

#[derive(Clone, Deserialize, Debug, PartialEq)]
pub struct SystemMonitorConfig {
    pub filesystems: Vec<FilesystemConfig>,
}

#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
pub struct NewYorkTimesConfig {
    pub api_key: String,
    pub most_popular_viewed_period: Option<MostPopularPeriod>,
    pub most_popular_emailed_period: Option<MostPopularPeriod>,
    pub most_popular_shared_period: Option<MostPopularPeriod>,
    pub most_popular_shared_mediums: Vec<ShareType>,
}

#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
pub struct NewsConfig {
    pub new_york_times: Option<NewYorkTimesConfig>,
}

#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
pub struct SchedulerConfig {
    pub schedules: Vec<ScheduleConfig>,
    pub tick_ms: u32,
}

#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
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

#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
pub struct AlertConfig {
    pub alert_interval: Option<Duration>,
    pub event: BroadcastEventType,
    pub mediums: Vec<BroadcastMedium>,
    pub alert_type: AlertType,
}

#[derive(Clone, Deserialize, Debug, PartialEq)]
pub struct Config {
    pub system_monitor: Option<SystemMonitorConfig>,
    pub commands: Option<CommandsConfig>,
    pub news: Option<NewsConfig>,
    pub scheduler: SchedulerConfig,
    pub broadcast: BroadcastConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            system_monitor: None,
            commands: None,
            news: None,
            scheduler: SchedulerConfig {
                schedules: vec![],
                tick_ms: 5000,
            },
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
    use super::*;

    use lazy_static::lazy_static;
    use std::result::Result as StdResult;
    use std::sync::{Mutex, MutexGuard, PoisonError};

    lazy_static! {
        static ref LOCK: Mutex<()> = Mutex::new(());
    }

    pub fn lock_config<'a>(
    ) -> StdResult<MutexGuard<'a, ()>, PoisonError<MutexGuard<'a, ()>>> {
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

    #[test]
    fn parse_config_toml() {
        let test_toml = r#"
            [[system_monitor.filesystems]]
            mount = "/"
            available_space_alert_above = 10.0

            [news.new_york_times]
            api_key = "api-key"
            most_popular_viewed_period = "7"
            most_popular_emailed_period = "7"
            most_popular_shared_period = "7"
            most_popular_shared_mediums = ["facebook"]

            [scheduler]
            tick_ms = 5000

            [[scheduler.schedules]]
            message = "check-disk-usage"

            [[scheduler.schedules]]
            cron = "0,10,20,30,40,50 * * * * * *"
            message = "fetch-news"

            [broadcast.email]
            smtp_host = "smtp.yahoo.com"
            username = "username"
            password = "password"
            recipients = ["recipient@aol.com"]

            [[broadcast.alerts]]
            alert_interval = { secs = 3600, nanos = 0 }
            mediums = ["email"]
            event = "high-disk-usage"
            alert_type = "alarm"

            [[broadcast.alerts]]
            mediums = ["email"]
            event = "newscast"
            alert_type = "digest"
        "#;

        let config: Config = toml::from_str(&test_toml).unwrap();
        let expected = Config {
            system_monitor: Some(SystemMonitorConfig {
                filesystems: vec![FilesystemConfig {
                    mount: PathBuf::from("/"),
                    available_space_alert_above: 10.0,
                }],
            }),
            commands: None,
            news: Some(NewsConfig {
                new_york_times: Some(NewYorkTimesConfig {
                    api_key: "api-key".to_string(),
                    most_popular_viewed_period: Some(
                        MostPopularPeriod::SevenDays,
                    ),
                    most_popular_emailed_period: Some(
                        MostPopularPeriod::SevenDays,
                    ),
                    most_popular_shared_period: Some(
                        MostPopularPeriod::SevenDays,
                    ),
                    most_popular_shared_mediums: vec![ShareType::Facebook],
                }),
            }),
            scheduler: SchedulerConfig {
                schedules: vec![
                    ScheduleConfig {
                        cron: None,
                        message: ScheduleMessage::CheckDiskUsage,
                    },
                    ScheduleConfig {
                        cron: Some("0,10,20,30,40,50 * * * * * *".to_string()),
                        message: ScheduleMessage::FetchNews,
                    },
                ],
                tick_ms: 5000,
            },
            broadcast: BroadcastConfig {
                email: Some(EmailConfig {
                    smtp_host: "smtp.yahoo.com".to_string(),
                    username: "username".to_string(),
                    password: "password".to_string(),
                    recipients: vec!["recipient@aol.com".to_string()],
                }),
                alerts: vec![
                    AlertConfig {
                        alert_interval: Some(Duration::from_secs(3600)),
                        mediums: vec![BroadcastMedium::Email],
                        event: BroadcastEventType::HighDiskUsage,
                        alert_type: AlertType::Alarm,
                    },
                    AlertConfig {
                        alert_interval: None,
                        mediums: vec![BroadcastMedium::Email],
                        event: BroadcastEventType::Newscast,
                        alert_type: AlertType::Digest,
                    },
                ],
            },
        };

        println!("config {:?}", config);
        println!("expected: {:?}", expected);

        assert!(config == expected);
    }
}
