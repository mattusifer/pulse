pub mod email;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use actix::prelude::*;
use crossbeam::queue::ArrayQueue;
use lazy_static::lazy_static;
use log::error;

use crate::config::{config, AlertConfig};
use crate::services::broadcast::email::{Emailer, SendEmail};
use crate::services::messages::{BroadcastEvent, BroadcastMedium};

lazy_static! {
    pub static ref OUTBOX: ArrayQueue<BroadcastEvent> =
        ArrayQueue::new(100_000);
}

const BROADCAST_TICK_INTERVAL: u64 = 500;

pub struct BroadcastInner {
    alerts: HashMap<String, (AlertConfig, Option<Instant>)>,
    emailer: Option<Box<dyn SendEmail>>,
}

impl BroadcastInner {
    pub fn new() -> Self {
        let config = config().broadcast;
        Self {
            alerts: config
                .alerts
                .iter()
                .map(|alert| {
                    (
                        serde_json::to_string(&alert.event).unwrap(),
                        (alert.clone(), None),
                    )
                })
                .collect(),
            emailer: config.email.map(Emailer::new),
        }
    }
}

pub struct Broadcast {
    broadcast: Arc<RwLock<BroadcastInner>>,
}
impl Broadcast {
    pub fn new() -> Self {
        Self {
            broadcast: Arc::new(RwLock::new(BroadcastInner::new())),
        }
    }

    #[cfg(test)]
    pub fn with_emailer(self, emailer: Box<dyn SendEmail>) -> Self {
        self.broadcast.write().unwrap().emailer = Some(emailer);
        self
    }
}

impl Actor for Broadcast {
    type Context = Context<Self>;

    /// Start a tick for the broadcast actor
    fn started(&mut self, ctx: &mut Context<Self>) {
        let broadcast = Arc::clone(&self.broadcast);
        ctx.run_interval(
            Duration::from_millis(BROADCAST_TICK_INTERVAL),
            move |_, _| {
                while let Ok(message) = OUTBOX.pop() {
                    let message_string =
                        serde_json::to_string(&message.event_type()).unwrap();

                    let alerts_map = broadcast.read().unwrap().alerts.clone();

                    // get the configuration for this message, if it exists
                    match alerts_map.get(&message_string) {
                        Some((alert_config, last_alerted))

                            // only need to alert if we haven't already
                            // alerted within the configured window
                            if last_alerted
                                .map(|last_alerted| {
                                    Instant::now().duration_since(last_alerted)
                                        > alert_config.alert_interval
                                })
                                .unwrap_or(true) =>
                        {
                            let prefix = if last_alerted.is_none() {
                                "[PULSE]"
                            } else {
                                "[PULSE] Retriggered:"
                            };

                            let (subject, body) = message.subject_and_body();
                            for medium in &alert_config.mediums {
                                match medium {
                                    BroadcastMedium::Email => {
                                        if let Some(ref emailer) = broadcast
                                            .read()
                                            .unwrap()
                                            .emailer {
                                                emailer.email(
                                                    format!("{} {}", prefix, subject.clone()),
                                                    body.clone(),
                                                )
                                                    .map_err(|_| ())
                                                    .unwrap();
                                            } else {
                                                error!("Email was not configured");
                                            }
                                    }
                                }
                            }
                            broadcast.write().unwrap().alerts.insert(
                                message_string,
                                (alert_config.clone(), Some(Instant::now())),
                            );
                        }
                        _ => (),
                    }
                }
            },
        );
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        config::Config, error::Result, services::messages::BroadcastEventType,
    };
    use std::{sync::Mutex, thread};

    thread_local! {
        static SENT_EMAILS: Vec<(String, String)> = vec![];
    }

    struct TestEmailer {
        sent_emails: Arc<Mutex<Vec<(String, String)>>>,
    }
    impl TestEmailer {
        pub fn new() -> Box<dyn SendEmail>
        where
            Self: SendEmail,
        {
            Box::new(Self {
                sent_emails: Arc::new(Mutex::new(vec![])),
            })
        }
    }
    impl SendEmail for TestEmailer {
        fn email(&self, subject: String, body: String) -> Result<()> {
            self.sent_emails.lock().unwrap().push((subject, body));
            Ok(())
        }
    }

    #[test]
    fn broadcast_sends_emails() {
        let mut config = Config::default();
        config.broadcast.alerts.push(AlertConfig {
            alert_interval: Duration::from_millis(100),
            event: BroadcastEventType::HighDiskUsage,
            mediums: vec![BroadcastMedium::Email],
        });

        let system = System::new("test");

        let emailer = TestEmailer::new();

        let sent_emails =
            &emailer.downcast_ref::<TestEmailer>().unwrap().sent_emails;
        let sent_emails = Arc::clone(sent_emails);

        run_with_config!(config, {
            Broadcast::new().with_emailer(emailer).start();

            let event = BroadcastEvent::HighDiskUsage {
                filesystem_mount: "/".to_string(),
                current_usage: 100.00,
                max_usage: 50.00,
            };

            OUTBOX.push(event).unwrap();

            let current = System::current();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(
                    50 + BROADCAST_TICK_INTERVAL,
                ));

                assert_eq!(sent_emails.lock().unwrap().len(), 1);

                current.stop()
            });

            system.run();
        });
    }
}
