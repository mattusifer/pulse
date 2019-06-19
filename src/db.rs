use std::sync::{Arc, Mutex};

use diesel::{pg::PgConnection, prelude::*};
use lazy_static::lazy_static;

use crate::{
    config,
    error::Result,
    schema::{disk_usage, tasks},
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

    pub fn insert_task(&self, task: models::NewTask) -> Result<models::Task> {
        self.inner.lock().unwrap().insert_task(task)
    }

    pub fn insert_disk_usage(
        &self,
        disk_usage: models::NewDiskUsage,
    ) -> Result<models::DiskUsage> {
        self.inner.lock().unwrap().insert_disk_usage(disk_usage)
    }
}

pub trait DatabaseInner {
    fn insert_task(&self, task: models::NewTask) -> Result<models::Task>;
    fn insert_disk_usage(
        &self,
        disk_usage: models::NewDiskUsage,
    ) -> Result<models::DiskUsage>;
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
        pub tasks: Vec<models::Task>,
        pub disk_usage: Vec<models::DiskUsage>,
    }

    impl TestDatabaseState {
        pub fn new() -> Self {
            Self {
                tasks: vec![],
                disk_usage: vec![],
            }
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
        fn insert_task(&self, task: models::NewTask) -> Result<models::Task> {
            let task = models::Task {
                id: 0,
                task: task.task,
                sent_at: PgTimestamp(0),
            };

            let mut state = self.state.lock().unwrap();
            state.tasks.push(task.clone());
            Ok(task)
        }

        fn insert_disk_usage(
            &self,
            disk_usage: models::NewDiskUsage,
        ) -> Result<models::DiskUsage> {
            let disk_usage = models::DiskUsage {
                id: 0,
                mount: disk_usage.mount,
                available_space: disk_usage.available_space,
                space_used: disk_usage.space_used,
                recorded_at: PgTimestamp(0),
            };

            let mut state = self.state.lock().unwrap();
            state.disk_usage.push(disk_usage.clone());
            Ok(disk_usage)
        }
    }

    // run the given block with a TestDatabase instance initialized
    // into the global database object, ensuring that no other threads
    // modify the database during execution. return the database state
    // after run.
    #[macro_export]
    macro_rules! run_with_db {
        ($system:expr) => {{
            use std::sync::{Arc, Mutex};

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
