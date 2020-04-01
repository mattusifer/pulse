use actix::Message;
use chrono::NaiveDateTime;
use diesel::{Insertable, Queryable};
use egg_mode::tweet::Tweet as EggModeTweet;
use serde::{Deserialize, Serialize};

use crate::schema::{disk_usage, tasks, tweets};

#[derive(Queryable, Clone, Debug)]
pub struct Task {
    pub id: i32,
    pub task: String,
    pub sent_at: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name = "tasks"]
pub struct NewTask {
    pub task: String,
}

impl NewTask {
    pub fn new<S: Into<String>>(task: S) -> Self {
        NewTask { task: task.into() }
    }
}

#[derive(Queryable, Clone, Debug, Message, Serialize, Deserialize)]
#[rtype(result = "()")]
#[serde(rename_all = "snake_case")]
pub struct DiskUsage {
    pub id: i32,
    pub mount: String,
    pub percent_disk_used: f64,
    pub recorded_at: NaiveDateTime,
}

impl Into<String> for DiskUsage {
    fn into(self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

#[derive(Debug, Insertable, Clone)]
#[table_name = "disk_usage"]
pub struct NewDiskUsage {
    pub mount: String,
    pub percent_disk_used: f64,
}

impl NewDiskUsage {
    pub fn new<S: Into<String>>(mount: S, percent_disk_used: f64) -> Self {
        NewDiskUsage {
            mount: mount.into(),
            percent_disk_used,
        }
    }
}

#[derive(Queryable, Clone, Debug, Message, Serialize, Deserialize)]
#[rtype(result = "()")]
#[serde(rename_all = "snake_case")]
pub struct Tweet {
    pub id: i32,
    pub twitter_tweet_id: String,
    pub group_name: Vec<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub favorite_count: i32,
    pub retweet_count: i32,
    pub username: Option<String>,
    pub lang: Option<String>,
    pub text: String,
    pub tweeted_at: NaiveDateTime,
}

impl Into<String> for Tweet {
    fn into(self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

#[derive(Debug, Insertable, Clone)]
#[table_name = "tweets"]
pub struct NewTweet {
    pub twitter_tweet_id: String,
    pub group_name: Vec<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub favorite_count: i32,
    pub retweet_count: i32,
    pub username: Option<String>,
    pub lang: Option<String>,
    pub text: String,
    pub tweeted_at: NaiveDateTime,
}

impl NewTweet {
    pub fn from_egg_mode_tweet(group_name: String, egg_mode_tweet: EggModeTweet) -> Self {
        Self {
            twitter_tweet_id: egg_mode_tweet.id.to_string(),
            group_name: vec![group_name],
            latitude: egg_mode_tweet.coordinates.map(|c| c.0),
            longitude: egg_mode_tweet.coordinates.map(|c| c.1),
            favorite_count: egg_mode_tweet.favorite_count,
            retweet_count: egg_mode_tweet.retweet_count,
            username: egg_mode_tweet.user.map(|u| u.screen_name),
            lang: egg_mode_tweet.lang,
            text: egg_mode_tweet.text,
            tweeted_at: egg_mode_tweet.created_at.naive_utc(),
        }
    }
}
