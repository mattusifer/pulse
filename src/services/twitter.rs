use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use actix::{fut::wrap_future, Actor, AsyncContext, Context};
use chrono::NaiveDateTime;
use egg_mode::{stream::StreamMessage, KeyPair, Token};
use futures::stream::Stream as _Stream;

use crate::{
    config::{config, TwitterConfig},
    db::{database, models},
    error::{
        future::{PulseStream, Stream},
        Result,
    },
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
    popular_tweets: HashMap<String, Vec<models::NewTweet>>,
    tweets_per_second: HashMap<String, VecDeque<NaiveDateTime>>,
}

impl Twitter {
    pub fn new() -> Option<Self> {
        config().twitter.map(|twitter_config| Self {
            config: twitter_config,
            ports: Arc::new(Box::new(LiveTwitterPorts)),
            popular_tweets: HashMap::new(),
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

    pub fn filter_streams(
        &self,
    ) -> impl Iterator<Item = (String, Box<Stream<StreamMessage>>)> + '_ {
        self.config.terms.iter().map(move |terms| {
            (
                terms.group_name.clone(),
                egg_mode::stream::filter()
                    .track(&terms.terms)
                    .language(&["en"])
                    .start(&self.get_token())
                    .map_err(Into::into)
                    .into_box(),
            )
        })
    }
}

impl Actor for Twitter {
    type Context = Context<Self>;

    /// When the twitter actor is started, open a connection to the
    /// streaming websocket
    fn started(&mut self, ctx: &mut Context<Self>) {
        // let twitter = self.clone();
        // self.filter_streams().for_each(move |(group_name, stream)| {
        //     let twitter = twitter.clone();
        //     let group_name_clone = group_name.clone();
        //     ctx.spawn(wrap_future(
        //         stream
        //             .map_err(move |e| {
        //                 log::error!(
        //                     "Error encountered opening twitter stream for group {}: {:?}",
        //                     group_name_clone,
        //                     e
        //                 )
        //             })
        //             .for_each(move |message| {
        //                 let twitter = twitter.clone();
        //                 if let StreamMessage::Tweet(egg_mode_tweet) = message {
        //                     let tweet = models::NewTweet::from_egg_mode_tweet(
        //                         group_name.clone(),
        //                         egg_mode_tweet,
        //                     );
        //                     if len(twitter.most_popular_tweets) >= MAX_TWEETS_TO_SEND {
        //                         let least_popular =
        //                     }

        //                     if let Err(e) = twitter.ports.record_tweet(tweet) {
        //                         log::error!("Error encountered when recording tweet: {:?}", e)
        //                     }
        //                 }

        //                 futures::future::ok(())
        //             }),
        //     ));
        // });
    }
}
