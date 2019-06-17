use std::sync::{Arc, Mutex};

use diesel::{pg::PgConnection, prelude::*};
use lazy_static::lazy_static;

use crate::{config, error::Result, schema::messages};

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

pub fn initialize_from(db: Database) -> () {
    *DATABASE.lock().unwrap() = Some(db)
}

#[cfg(test)]
pub fn unset() -> () {
    *DATABASE.lock().unwrap() = None
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

    pub fn insert_message(
        &self,
        message: models::NewMessage,
    ) -> Result<models::Message> {
        self.inner.lock().unwrap().insert_message(message)
    }
}

pub trait DatabaseInner {
    fn insert_message(
        &self,
        message: models::NewMessage,
    ) -> Result<models::Message>;
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
    fn insert_message(
        &self,
        message: models::NewMessage,
    ) -> Result<models::Message> {
        diesel::insert_into(messages::table)
            .values(&message)
            .get_result(&self.connection)
            .map_err(Into::into)
    }
}

#[macro_use]
#[cfg(test)]
pub mod test {
    use diesel::pg::data_types::PgTimestamp;
    use lazy_static::lazy_static;
    use std::{
        result::Result as StdResult,
        sync::{Arc, Mutex, MutexGuard, PoisonError},
    };

    use crate::{
        db::{models, DatabaseInner},
        Result,
    };

    lazy_static! {
        static ref LOCK: Mutex<()> = Mutex::new(());
    }

    pub fn lock_database<'a>(
    ) -> StdResult<MutexGuard<'a, ()>, PoisonError<MutexGuard<'a, ()>>> {
        LOCK.lock()
    }

    #[derive(Debug, Clone)]
    pub struct TestDatabaseState {
        pub messages: Vec<models::Message>,
    }

    impl TestDatabaseState {
        pub fn new() -> Self {
            Self { messages: vec![] }
        }
    }

    pub struct TestDatabase {
        state: Arc<Mutex<TestDatabaseState>>,
    }

    impl TestDatabase {
        pub fn new(state: Arc<Mutex<TestDatabaseState>>) -> Self {
            Self { state }
        }
    }

    impl DatabaseInner for TestDatabase {
        fn insert_message(
            &self,
            message: models::NewMessage,
        ) -> Result<models::Message> {
            let message = models::Message {
                id: 0,
                message: message.message,
                sent_at: PgTimestamp(0),
            };

            let mut state = self.state.lock().unwrap();
            state.messages.push(message.clone());
            Ok(message)
        }
    }

    // run the given block with a TestDatabase instance initialized
    // into the global database object, ensuring that no other threads
    // modify the database during execution. return the database state
    // after run.
    #[macro_export]
    macro_rules! run_with_db {
        ($system:expr) => {{
            use crate::db::{self, test};

            let _lock = test::lock_database();

            let state = Arc::new(Mutex::new(test::TestDatabaseState::new()));
            db::initialize_from(db::Database::new(test::TestDatabase::new(
                Arc::clone(&state),
            )));

            $system.run().unwrap();

            // unset the database in order to access the underlying state
            db::unset();
            let mutex = Arc::try_unwrap(state).ok().unwrap();
            let result = mutex.lock().unwrap().clone();
            result
        }};
    }
}
