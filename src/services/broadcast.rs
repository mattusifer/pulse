mod email;

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use actix::prelude::*;
use crossbeam::queue::ArrayQueue;
use lazy_static::lazy_static;

use crate::{
    config::{config, AlertConfig, AlertType, EmailConfig},
    error::{Error, Result},
    services::messages::{
        BroadcastEvent, BroadcastEventKey, BroadcastEventType, BroadcastMedium,
    },
};

lazy_static! {
    pub static ref OUTBOX: ArrayQueue<BroadcastEvent> =
        ArrayQueue::new(100_000);
}

const BROADCAST_TICK_INTERVAL: u64 = 500;

trait BroadcastPorts {
    fn send_email(&self, subject: String, body: String) -> Result<()>;
    fn get_next_event(&self) -> Option<BroadcastEvent>;
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
}

pub struct Broadcast {
    alerts: HashMap<BroadcastEventType, AlertConfig>,
    last_alerted: HashMap<BroadcastEventKey, Instant>,
    ports: Box<BroadcastPorts + Send + Sync>,
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
                last_alerted: vec![].into_iter().collect(),
                ports: Box::new(LiveBroadcastPorts { email_config }),
            }))
        } else {
            Err(Error::unconfigured_email())
        }
    }

    #[cfg(test)]
    fn test(
        alerts: HashMap<BroadcastEventType, AlertConfig>,
        ports: Box<BroadcastPorts + Send + Sync>,
    ) -> Self {
        Self {
            alerts,
            last_alerted: vec![].into_iter().collect(),
            ports,
        }
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
                    let last_alerted_map = this.last_alerted.clone();

                    let message_id = message.event_key();
                    let message_type = message.event_type();

                    let last_alerted = last_alerted_map.get(&message_id);

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
                            this.last_alerted.insert(
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
        config::AlertType, error::Result,
        services::messages::BroadcastEventType,
    };
    use std::{
        sync::{Arc, Mutex},
        thread,
    };

    struct TestBroadcastPorts {
        sent_emails: Vec<(String, String)>,
        events_buffer: Vec<BroadcastEvent>,
    }
    impl TestBroadcastPorts {
        pub fn new(events_buffer: Vec<BroadcastEvent>) -> Self {
            Self {
                sent_emails: vec![],
                events_buffer,
            }
        }
    }
    impl BroadcastPorts for Arc<Mutex<TestBroadcastPorts>> {
        fn send_email(&self, subject: String, body: String) -> Result<()> {
            self.lock().unwrap().sent_emails.push((subject, body));
            Ok(())
        }

        fn get_next_event(&self) -> Option<BroadcastEvent> {
            self.lock().unwrap().events_buffer.pop()
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

        let ports = Arc::new(Mutex::new(TestBroadcastPorts::new(vec![event])));

        Broadcast::test(alerts, Box::new(Arc::clone(&ports))).start();

        let current = System::current();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50 + BROADCAST_TICK_INTERVAL));
            current.stop()
        });

        system.run().unwrap();

        assert_eq!(ports.lock().unwrap().sent_emails.len(), 1);
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

        let ports = Arc::new(Mutex::new(TestBroadcastPorts::new(events)));

        Broadcast::test(alerts, Box::new(Arc::clone(&ports))).start();

        let current = System::current();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50 + BROADCAST_TICK_INTERVAL));
            current.stop()
        });

        system.run().unwrap();
        assert_eq!(ports.lock().unwrap().sent_emails.len(), 2);
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
        let mut events: Vec<BroadcastEvent> = vec![];
        for _ in 1..10 {
            events.push(event.clone());
        }

        let system = System::new("test");

        let ports = Arc::new(Mutex::new(TestBroadcastPorts::new(events)));
        let ports_clone = Arc::clone(&ports);

        Broadcast::test(alerts, Box::new(Arc::clone(&ports))).start();

        let current = System::current();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50 + BROADCAST_TICK_INTERVAL));

            // push another one, this one should get alerted on
            // now that we're past the alert interval
            ports_clone
                .lock()
                .unwrap()
                .events_buffer
                .push(event.clone());

            thread::sleep(Duration::from_millis(BROADCAST_TICK_INTERVAL));
            current.stop()
        });

        system.run().unwrap();

        assert_eq!(ports.lock().unwrap().sent_emails.len(), 2);
    }
}
