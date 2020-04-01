use std::collections::{HashMap, HashSet, VecDeque};
use std::iter::FromIterator;
use std::result::Result as StdResult;
use std::sync::Arc;

use actix::{
    fut::{wrap_future, wrap_stream, ActorStream},
    Actor, AsyncContext, Context,
};
use egg_mode::{error::Error as EggModeError, stream::StreamMessage, KeyPair, Token};

use crate::{
    config::{config, TwitterConfig},
    db::{database, models},
    error::Result,
    services::broadcast::{BroadcastEvent, OUTBOX},
};

const MAX_TWEETS_TO_SEND: usize = 100;

fn get_token(config: &TwitterConfig) -> Token {
    let consumer_token = KeyPair::new(config.consumer_key.clone(), config.consumer_secret.clone());
    let access_token = KeyPair::new(config.access_key.clone(), config.access_secret.clone());

    Token::Access {
        consumer: consumer_token,
        access: access_token,
    }
}

trait TwitterPorts {
    fn record_tweet(&self, tweet: models::NewTweet) -> Result<models::Tweet>;

    fn send_alert(&self, event: BroadcastEvent) -> Result<()>;
}

struct LiveTwitterPorts;
impl TwitterPorts for LiveTwitterPorts {
    fn record_tweet(&self, tweet: models::NewTweet) -> Result<models::Tweet> {
        database().insert_tweet(tweet)
    }

    fn send_alert(&self, event: BroadcastEvent) -> Result<()> {
        OUTBOX.push(event).map_err(Into::into)
    }
}

#[derive(Clone)]
pub struct Twitter {
    config: TwitterConfig,
    ports: Arc<Box<dyn TwitterPorts>>,
    tweet_buffer: HashMap<String, VecDeque<models::NewTweet>>,
}

impl Twitter {
    pub fn new() -> Option<Self> {
        config().twitter.map(|twitter_config| Self {
            config: twitter_config,
            ports: Arc::new(Box::new(LiveTwitterPorts)),
            tweet_buffer: HashMap::new(),
        })
    }

    fn filter_streams(
        &self,
        config: &TwitterConfig,
    ) -> impl ActorStream<Item = StdResult<StreamMessage, EggModeError>, Actor = Twitter> {
        let terms = config.terms.iter().fold(HashSet::new(), |acc, terms| {
            acc.union(&HashSet::from_iter(terms.terms.iter()))
                .cloned()
                .collect()
        });
        wrap_stream::<_, Twitter>(
            egg_mode::stream::filter()
                .track(&terms)
                .language(&["en"])
                .start(&get_token(&config)),
        )
    }
}

impl Actor for Twitter {
    type Context = Context<Self>;

    /// When the twitter actor is started, open a connection to the
    /// streaming websocket
    fn started(&mut self, ctx: &mut Context<Self>) {
        let stream_filter_process =
            self.filter_streams(&self.config)
                .fold((), |_acc, message, actor, _ctx| {
                    if let Ok(StreamMessage::Tweet(egg_mode_tweet)) = message {
                        for term in &actor.config.terms {
                            let egg_mode_tweet = egg_mode_tweet.clone();
                            let group_name_clone = term.group_name.clone();
                            let tweet_contains_term =
                                term.terms.iter().any(|t| egg_mode_tweet.text.contains(t));

                            if tweet_contains_term {
                                let new_tweet = models::NewTweet::from_egg_mode_tweet(
                                    term.group_name.clone(),
                                    egg_mode_tweet,
                                );
                                if let Err(e) = actor.ports.record_tweet(new_tweet.clone()) {
                                    log::error!("Error encountered when recording tweet: {:?}", e)
                                }

                                if !actor.tweet_buffer.contains_key(&group_name_clone) {
                                    actor
                                        .tweet_buffer
                                        .insert(group_name_clone.clone(), vec![new_tweet].into());
                                } else {
                                    let tweets =
                                        actor.tweet_buffer.get_mut(&group_name_clone).unwrap();
                                    if tweets.len() >= MAX_TWEETS_TO_SEND {
                                        tweets.pop_front();
                                    }

                                    tweets.push_back(new_tweet.clone());
                                }
                            }
                        }
                    } else {
                        log::error!("Error encountered parsing tweet: {:?}", message)
                    }
                    log::info!(
                        "sizes: {:?}",
                        actor
                            .tweet_buffer
                            .iter()
                            .map(|(k, v)| (k, v.len()))
                            .collect::<Vec<(&String, usize)>>()
                    );
                    wrap_future(futures::future::ready(()))
                });

        ctx.spawn(stream_filter_process);
    }
}
