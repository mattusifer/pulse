pub mod email;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use actix::prelude::*;
use crossbeam::queue::ArrayQueue;
use lazy_static::lazy_static;

use crate::config::{config, AlertConfig, AlertType};
use crate::services::broadcast::email::{Emailer, SendEmail};
use crate::services::messages::{
    BroadcastEvent, BroadcastEventKey, BroadcastEventType, BroadcastMedium,
};

lazy_static! {
    pub static ref OUTBOX: ArrayQueue<BroadcastEvent> =
        ArrayQueue::new(100_000);
}

const BROADCAST_TICK_INTERVAL: u64 = 500;

pub struct BroadcastInner {
    alerts: HashMap<BroadcastEventType, AlertConfig>,
    last_alerted: HashMap<BroadcastEventKey, Instant>,
    emailer: Option<Box<dyn SendEmail>>,
}

impl BroadcastInner {
    pub fn new() -> Self {
        let config = config().broadcast;
        Self {
            alerts: config
                .alerts
                .iter()
                .map(|alert| (alert.event.clone(), alert.clone()))
                .collect(),
            last_alerted: vec![].into_iter().collect(),
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
                    log::debug!("Broadcast received message: {:?}", message.event_type());

                    let alerts_map = broadcast.read().unwrap().alerts.clone();
                    let last_alerted_map = broadcast.read().unwrap().last_alerted.clone();

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
                                                log::error!("Email was not configured");
                                            }
                                    }
                                }
                            }
                            broadcast.write().unwrap().alerts.insert(
                                message_type,
                                alert_config.clone(),
                            );
                            broadcast.write().unwrap().last_alerted.insert(
                                message_id,
                                Instant::now()
                            );
                        }
                        _ => {
                            log::debug!("Not alerting: {:?}. Alerts map entry: {:?}", message.event_type(), alerts_map.get(&message_type));
                            ()
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
        config::{AlertType, Config},
        error::Result,
        services::messages::BroadcastEventType,
    };
    use std::{
        result,
        sync::{Mutex, MutexGuard, PoisonError},
        thread,
    };

    lazy_static! {
        static ref LOCK: Mutex<()> = Mutex::new(());
    }

    pub fn lock_outbox<'a>(
    ) -> result::Result<MutexGuard<'a, ()>, PoisonError<MutexGuard<'a, ()>>>
    {
        LOCK.lock()
    }

    // run the given block with exclusive access to the OUTBOX
    #[macro_export]
    macro_rules! run_with_outbox {
        ($test_block:expr) => {{
            use crate::services::broadcast::test;
            let _lock = test::lock_outbox();

            $test_block
        }};
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
    fn broadcast_sends_an_email() {
        let mut config = Config::default();
        config.broadcast.alerts.push(AlertConfig {
            alert_interval: Some(Duration::from_millis(100)),
            event: BroadcastEventType::HighDiskUsage,
            mediums: vec![BroadcastMedium::Email],
            alert_type: AlertType::Alarm,
        });

        let system = System::new("test");

        let emailer = TestEmailer::new();

        let sent_emails =
            &emailer.downcast_ref::<TestEmailer>().unwrap().sent_emails;
        let sent_emails = Arc::clone(sent_emails);

        run_with_config!(config, {
            Broadcast::new().with_emailer(emailer).start();
        });

        let event = BroadcastEvent::HighDiskUsage {
            filesystem_mount: "/".to_string(),
            current_usage: 100.00,
            max_usage: 50.00,
        };

        run_with_outbox!({
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

    #[test]
    fn broadcast_sends_multiple_emails() {
        let mut config = Config::default();
        config.broadcast.alerts.push(AlertConfig {
            alert_interval: Some(Duration::from_millis(100)),
            event: BroadcastEventType::HighDiskUsage,
            mediums: vec![BroadcastMedium::Email],
            alert_type: AlertType::Alarm,
        });

        let system = System::new("test");

        let emailer = TestEmailer::new();

        let sent_emails =
            &emailer.downcast_ref::<TestEmailer>().unwrap().sent_emails;
        let sent_emails = Arc::clone(sent_emails);

        run_with_config!(config, {
            Broadcast::new().with_emailer(emailer).start();
        });

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

        run_with_outbox!({
            for event in events {
                OUTBOX.push(event).unwrap();
            }

            let current = System::current();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(
                    50 + BROADCAST_TICK_INTERVAL,
                ));

                assert_eq!(sent_emails.lock().unwrap().len(), 2);

                current.stop()
            });

            system.run();
        });
    }

    #[test]
    fn broadcast_ignores_alerts_if_an_alert_was_just_sent() {
        let mut config = Config::default();
        config.broadcast.alerts.push(AlertConfig {
            alert_interval: Some(Duration::from_millis(100)),
            event: BroadcastEventType::HighDiskUsage,
            mediums: vec![BroadcastMedium::Email],
            alert_type: AlertType::Alarm,
        });

        let system = System::new("test");

        let emailer = TestEmailer::new();

        let sent_emails =
            &emailer.downcast_ref::<TestEmailer>().unwrap().sent_emails;
        let sent_emails = Arc::clone(sent_emails);

        run_with_config!(config, {
            Broadcast::new().with_emailer(emailer).start();
        });

        let event = BroadcastEvent::HighDiskUsage {
            filesystem_mount: "/".to_string(),
            current_usage: 100.00,
            max_usage: 50.00,
        };

        run_with_outbox!({
            // push 10 events in a row, only one of these should get
            // alerted on.
            for _ in 1..10 {
                OUTBOX.push(event.clone()).unwrap();
            }

            let current = System::current();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(
                    50 + BROADCAST_TICK_INTERVAL,
                ));

                // push another one, this one should get alerted on
                // now that we're past the alert interval
                OUTBOX.push(event.clone()).unwrap();

                thread::sleep(Duration::from_millis(BROADCAST_TICK_INTERVAL));

                assert_eq!(sent_emails.lock().unwrap().len(), 2);

                current.stop()
            });

            system.run();
        });
    }
}
