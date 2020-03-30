mod email;
mod events;
pub use events::*;

use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
    time::{Duration, Instant},
};

use actix::prelude::*;
use crossbeam::queue::ArrayQueue;
use lazy_static::lazy_static;

use crate::{
    config::{config, AlertConfig, AlertType, EmailConfig},
    error::{Error, Result},
};

type LastAlerted = HashMap<BroadcastEventKey, Instant>;

lazy_static! {
    pub static ref OUTBOX: ArrayQueue<BroadcastEvent> = ArrayQueue::new(100_000);
    static ref LAST_ALERTED: Mutex<LastAlerted> = Mutex::new(HashMap::new());
}

const BROADCAST_TICK_INTERVAL: u64 = 500;

trait BroadcastPorts {
    fn send_email(&self, subject: String, body: String) -> Result<()>;
    fn get_next_event(&self) -> Option<BroadcastEvent>;
    fn lock_last_alerted(&self) -> MutexGuard<LastAlerted>;
}

struct LiveBroadcastPorts {
    email_config: EmailConfig,
}
impl BroadcastPorts for LiveBroadcastPorts {
    fn send_email(&self, subject: String, body: String) -> Result<()> {
        email::send_email(&self.email_config, subject, body)
    }

    fn get_next_event(&self) -> Option<BroadcastEvent> {
        OUTBOX.pop().ok()
    }

    fn lock_last_alerted(&self) -> MutexGuard<LastAlerted> {
        LAST_ALERTED.lock().unwrap()
    }
}

pub struct Broadcast {
    alerts: HashMap<BroadcastEventType, AlertConfig>,
    ports: Box<dyn BroadcastPorts + Send + Sync>,
}

impl Broadcast {
    pub fn new() -> Result<Option<Self>> {
        let config = config().broadcast;
        if config.alerts.is_empty() {
            Ok(None)
        } else if let Some(email_config) = config.email {
            Ok(Some(Self {
                alerts: config
                    .alerts
                    .iter()
                    .map(|alert| (alert.event.clone(), alert.clone()))
                    .collect(),
                ports: Box::new(LiveBroadcastPorts { email_config }),
            }))
        } else {
            Err(Error::unconfigured_email())
        }
    }

    #[cfg(test)]
    fn test(
        alerts: HashMap<BroadcastEventType, AlertConfig>,
        ports: Box<dyn BroadcastPorts + Send + Sync>,
    ) -> Self {
        Self { alerts, ports }
    }
}

impl Actor for Broadcast {
    type Context = Context<Self>;

    /// Start a tick for the broadcast actor
    fn started(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(
            Duration::from_millis(BROADCAST_TICK_INTERVAL),
            move |this, _| {
                while let Some(message) = this.ports.get_next_event() {
                    log::debug!("Broadcast received message: {:?}", message.event_type());

                    let alerts_map = this.alerts.clone();

                    let message_id = message.event_key();
                    let message_type = message.event_type();

                    let mut locked_last_alerted = this.ports.lock_last_alerted();
                    let last_alerted = locked_last_alerted.get(&message_id);

                    // get the configuration for this message, if it exists
                    match alerts_map.get(&message_type) {
                        Some(alert_config)
                            // only need to alert if we haven't already
                            // alerted within the configured window
                            if alert_config.alert_interval.is_none() || last_alerted
                                .map(|instant| {
                                    Instant::now().duration_since(*instant)
                                        > alert_config.alert_interval.unwrap()
                                })
                                .unwrap_or(true) =>
                        {
                            log::debug!("Sending alert for : {:?}", message);
                            let prefix = if last_alerted.is_none() || alert_config.alert_type == AlertType::Digest {
                                "[PULSE]"
                            } else {
                                "[PULSE] Retriggered:"
                            };

                            let (subject, body) = message.subject_and_body();
                            for medium in &alert_config.mediums {
                                match medium {
                                    BroadcastMedium::Email => {
                                        this.ports.send_email(
                                                format!("{} {}", prefix, subject.clone()),
                                                body.clone(),
                                            )
                                            .map_err(|_| ())
                                            .unwrap();
                                    }
                                }
                            }
                            this.alerts.insert(
                                message_type,
                                alert_config.clone(),
                            );
                            locked_last_alerted.insert(
                                message_id,
                                Instant::now()
                            );
                        }
                        _ => {
                            log::debug!(
                                "Not alerting: {:?}. Alerts map entry: {:?}",
                                message.event_type(), alerts_map.get(&message_type)
                            );
                        },
                    }
                }
            },
        );
    }
}

#[macro_use]
#[cfg(test)]
pub mod test {
    use super::*;
    use crate::{
        config::AlertType, error::Result, services::broadcast::events::BroadcastEventType,
    };
    use std::{
        sync::{Arc, Mutex},
        thread,
    };

    struct TestBroadcastPorts {
        sent_emails: Arc<Mutex<Vec<(String, String)>>>,
        events_buffer: Arc<Mutex<Vec<BroadcastEvent>>>,
        last_alerted: Arc<Mutex<LastAlerted>>,
    }
    impl TestBroadcastPorts {
        pub fn new() -> Self {
            Self {
                sent_emails: Arc::new(Mutex::new(vec![])),
                events_buffer: Arc::new(Mutex::new(vec![])),
                last_alerted: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        pub fn with_sent_emails(mut self, sent_emails: Arc<Mutex<Vec<(String, String)>>>) -> Self {
            self.sent_emails = sent_emails;
            self
        }

        pub fn with_events_buffer(
            mut self,
            events_buffer: Arc<Mutex<Vec<BroadcastEvent>>>,
        ) -> Self {
            self.events_buffer = events_buffer;
            self
        }
    }
    impl BroadcastPorts for TestBroadcastPorts {
        fn send_email(&self, subject: String, body: String) -> Result<()> {
            self.sent_emails.lock().unwrap().push((subject, body));
            Ok(())
        }

        fn get_next_event(&self) -> Option<BroadcastEvent> {
            self.events_buffer.lock().unwrap().pop()
        }

        fn lock_last_alerted(&self) -> MutexGuard<LastAlerted> {
            self.last_alerted.lock().unwrap()
        }
    }

    #[test]
    fn broadcast_sends_an_email() {
        let alerts: HashMap<BroadcastEventType, AlertConfig> = vec![(
            BroadcastEventType::HighDiskUsage,
            AlertConfig {
                alert_interval: Some(Duration::from_millis(100)),
                event: BroadcastEventType::HighDiskUsage,
                mediums: vec![BroadcastMedium::Email],
                alert_type: AlertType::Alarm,
            },
        )]
        .into_iter()
        .collect();

        let event = BroadcastEvent::HighDiskUsage {
            filesystem_mount: "/".to_string(),
            current_usage: 100.00,
            max_usage: 50.00,
        };

        let system = System::new("test");

        let sent_emails = Arc::new(Mutex::new(vec![]));

        let ports = TestBroadcastPorts::new()
            .with_events_buffer(Arc::new(Mutex::new(vec![event])))
            .with_sent_emails(Arc::clone(&sent_emails));

        Broadcast::test(alerts, Box::new(ports)).start();

        let current = System::current();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50 + BROADCAST_TICK_INTERVAL));
            current.stop()
        });

        system.run().unwrap();

        assert_eq!(sent_emails.lock().unwrap().len(), 1);
    }

    #[test]
    fn broadcast_sends_multiple_emails() {
        let alerts: HashMap<BroadcastEventType, AlertConfig> = vec![(
            BroadcastEventType::HighDiskUsage,
            AlertConfig {
                alert_interval: Some(Duration::from_millis(100)),
                event: BroadcastEventType::HighDiskUsage,
                mediums: vec![BroadcastMedium::Email],
                alert_type: AlertType::Alarm,
            },
        )]
        .into_iter()
        .collect();

        let events = vec![
            BroadcastEvent::HighDiskUsage {
                filesystem_mount: "/".to_string(),
                current_usage: 100.00,
                max_usage: 50.00,
            },
            BroadcastEvent::HighDiskUsage {
                filesystem_mount: "/mnt/test".to_string(),
                current_usage: 100.00,
                max_usage: 50.00,
            },
        ];

        let system = System::new("test");

        let sent_emails = Arc::new(Mutex::new(vec![]));

        let ports = TestBroadcastPorts::new()
            .with_events_buffer(Arc::new(Mutex::new(events)))
            .with_sent_emails(Arc::clone(&sent_emails));

        Broadcast::test(alerts, Box::new(ports)).start();

        let current = System::current();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50 + BROADCAST_TICK_INTERVAL));
            current.stop()
        });

        system.run().unwrap();
        assert_eq!(sent_emails.lock().unwrap().len(), 2);
    }

    #[test]
    fn broadcast_ignores_alerts_if_an_alert_was_just_sent() {
        let alerts: HashMap<BroadcastEventType, AlertConfig> = vec![(
            BroadcastEventType::HighDiskUsage,
            AlertConfig {
                alert_interval: Some(Duration::from_millis(100)),
                event: BroadcastEventType::HighDiskUsage,
                mediums: vec![BroadcastMedium::Email],
                alert_type: AlertType::Alarm,
            },
        )]
        .into_iter()
        .collect();

        // simulate 10 identical events in a row
        let event = BroadcastEvent::HighDiskUsage {
            filesystem_mount: "/".to_string(),
            current_usage: 100.00,
            max_usage: 50.00,
        };
        let events: Arc<Mutex<Vec<BroadcastEvent>>> = Arc::new(Mutex::new(vec![]));
        let events_clone = Arc::clone(&events);
        for _ in 1..10 {
            events.lock().unwrap().push(event.clone());
        }

        let system = System::new("test");

        let sent_emails = Arc::new(Mutex::new(vec![]));

        let ports = TestBroadcastPorts::new()
            .with_events_buffer(Arc::clone(&events))
            .with_sent_emails(Arc::clone(&sent_emails));

        Broadcast::test(alerts, Box::new(ports)).start();

        let current = System::current();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50 + BROADCAST_TICK_INTERVAL));

            // push another one, this one should get alerted on
            // now that we're past the alert interval
            events_clone.lock().unwrap().push(event.clone());

            thread::sleep(Duration::from_millis(BROADCAST_TICK_INTERVAL));
            current.stop()
        });

        system.run().unwrap();

        assert_eq!(sent_emails.lock().unwrap().len(), 2);
    }
}
