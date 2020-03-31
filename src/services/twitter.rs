use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use actix::{fut::wrap_future, Actor, AsyncContext, Context};
use chrono::NaiveDateTime;
use egg_mode::{
    stream::{StreamMessage, TwitterStream},
    KeyPair, Token,
};
use futures::stream::StreamExt;

use crate::{
    config::{config, TwitterConfig},
    db::{database, models},
    error::Result,
    services::broadcast::{BroadcastEvent, OUTBOX},
};

const MAX_TWEETS_TO_SEND: usize = 100;

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
    popular_tweets: BTreeMap<String, Vec<models::NewTweet>>,
    tweets_per_second: HashMap<String, VecDeque<NaiveDateTime>>,
}

impl Twitter {
    pub fn new() -> Option<Self> {
        config().twitter.map(|twitter_config| Self {
            config: twitter_config,
            ports: Arc::new(Box::new(LiveTwitterPorts)),
            popular_tweets: BTreeMap::new(),
            tweets_per_second: HashMap::new(),
        })
    }

    fn get_token(&self) -> Token {
        let consumer_token = KeyPair::new(
            self.config.consumer_key.clone(),
            self.config.consumer_secret.clone(),
        );
        let access_token = KeyPair::new(
            self.config.access_key.clone(),
            self.config.access_secret.clone(),
        );

        Token::Access {
            consumer: consumer_token,
            access: access_token,
        }
    }

    pub fn filter_streams(&self) -> impl Iterator<Item = (String, TwitterStream)> + '_ {
        self.config.terms.iter().map(move |terms| {
            (
                terms.group_name.clone(),
                egg_mode::stream::filter()
                    .track(&terms.terms)
                    .language(&["en"])
                    .start(&self.get_token()),
            )
        })
    }
}

impl Actor for Twitter {
    type Context = Context<Self>;

    /// When the twitter actor is started, open a connection to the
    /// streaming websocket
    fn started(&mut self, ctx: &mut Context<Self>) {
        let twitter = Arc::new(Mutex::new(self.clone()));
        self.filter_streams().for_each(move |(group_name, stream)| {
            let twitter = Arc::clone(&twitter);
            let group_name_clone = group_name.clone();
            ctx.spawn(wrap_future(stream.for_each(move |message| {
                let mut twitter = twitter.lock().unwrap();
                if let Ok(StreamMessage::Tweet(egg_mode_tweet)) = message {
                    let new_tweet =
                        models::NewTweet::from_egg_mode_tweet(group_name.clone(), egg_mode_tweet);

                    log::info!("Received {:?}", new_tweet);

                    if !twitter.popular_tweets.contains_key(&group_name_clone) {
                        twitter
                            .popular_tweets
                            .insert(group_name_clone.clone(), vec![new_tweet.clone()]);
                    } else if twitter.popular_tweets[&group_name_clone].len() >= MAX_TWEETS_TO_SEND
                    {
                        let tweets = twitter.popular_tweets.get_mut(&group_name_clone).unwrap();
                        let minimum_favorite = tweets
                            .iter()
                            .map(|tweet| tweet.favorite_count)
                            .min()
                            .unwrap();
                        let minimum_favorite_idx = tweets
                            .iter()
                            .position(|tweet| tweet.favorite_count == minimum_favorite)
                            .unwrap();

                        let removed = tweets.remove(minimum_favorite_idx);
                        log::info!(
                            "Maximum popular tweets exceeded for {}, removed {:?}",
                            group_name_clone,
                            removed
                        );

                        tweets.push(new_tweet.clone());
                    }

                    if let Err(e) = twitter.ports.record_tweet(new_tweet) {
                        log::error!("Error encountered when recording tweet: {:?}", e)
                    }
                } else {
                    log::error!("Error encountered parsing tweet: {:?}", message)
                }
                futures::future::ready(())
            })));
        });
    }
}
