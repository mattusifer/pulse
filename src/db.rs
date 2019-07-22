use std::sync::{Arc, Mutex};

use diesel::{pg::PgConnection, prelude::*};
use lazy_static::lazy_static;

use crate::{
    config,
    error::Result,
    schema::{disk_usage, tasks, tweets},
};

pub mod models;

lazy_static! {
    static ref DATABASE: Mutex<Option<Database>> = Mutex::new(None);
}

/// Get a database instance
pub fn database() -> Database {
    DATABASE
        .lock()
        .unwrap()
        .clone()
        .expect("Database was accessed before it was initialized")
}

pub fn initialize_postgres() -> Result<()> {
    let postgres = PostgresDatabase::new()?;
    initialize_from(Database::new(postgres));

    Ok(())
}

pub fn initialize_from(db: Database) {
    *DATABASE.lock().unwrap() = Some(db)
}

#[derive(Clone)]
pub struct Database {
    inner: Arc<Mutex<DatabaseInner + Send>>,
}

impl Database {
    pub fn new<I: 'static + DatabaseInner + Send>(inner: I) -> Self {
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    pub fn insert_task(&self, task: models::NewTask) -> Result<models::Task> {
        self.inner.lock().unwrap().insert_task(task)
    }

    pub fn insert_disk_usage(
        &self,
        disk_usage: models::NewDiskUsage,
    ) -> Result<models::DiskUsage> {
        self.inner.lock().unwrap().insert_disk_usage(disk_usage)
    }

    pub fn insert_tweet(
        &self,
        tweet: models::NewTweet,
    ) -> Result<models::Tweet> {
        self.inner.lock().unwrap().insert_tweet(tweet)
    }
}

pub trait DatabaseInner {
    fn insert_task(&self, task: models::NewTask) -> Result<models::Task>;
    fn insert_disk_usage(
        &self,
        disk_usage: models::NewDiskUsage,
    ) -> Result<models::DiskUsage>;
    fn insert_tweet(&self, tweet: models::NewTweet) -> Result<models::Tweet>;
}

pub struct PostgresDatabase {
    connection: PgConnection,
}

impl PostgresDatabase {
    pub fn new() -> Result<Self> {
        let config = config::config().database;

        let database_url = format!(
            "postgres://{username}:{password}@{host}/{database}",
            username = config.username,
            password = config.password,
            host = config.host,
            database = config.database
        );

        PgConnection::establish(&database_url)
            .map_err(Into::into)
            .map(|connection| Self { connection })
    }
}

impl DatabaseInner for PostgresDatabase {
    fn insert_task(&self, task: models::NewTask) -> Result<models::Task> {
        diesel::insert_into(tasks::table)
            .values(&task)
            .get_result(&self.connection)
            .map_err(Into::into)
    }

    fn insert_disk_usage(
        &self,
        disk_usage: models::NewDiskUsage,
    ) -> Result<models::DiskUsage> {
        diesel::insert_into(disk_usage::table)
            .values(&disk_usage)
            .get_result(&self.connection)
            .map_err(Into::into)
    }

    fn insert_tweet(&self, tweet: models::NewTweet) -> Result<models::Tweet> {
        diesel::insert_into(tweets::table)
            .values(&tweet)
            .get_result(&self.connection)
            .map_err(Into::into)
    }
}
