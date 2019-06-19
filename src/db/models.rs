use diesel::{pg::data_types::*, Insertable, Queryable};

use crate::schema::{disk_usage, tasks};

#[derive(Queryable, Clone, Debug)]
pub struct Task {
    pub id: i32,
    pub task: String,
    pub sent_at: PgTimestamp,
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

#[derive(Queryable, Clone, Debug)]
pub struct DiskUsage {
    pub id: i32,
    pub mount: String,
    pub available_space: i64,
    pub space_used: i64,
    pub recorded_at: PgTimestamp,
}

#[derive(Insertable)]
#[table_name = "disk_usage"]
pub struct NewDiskUsage {
    pub mount: String,
    pub available_space: i64,
    pub space_used: i64,
}

impl NewDiskUsage {
    pub fn new<S: Into<String>>(
        mount: S,
        available_space: i64,
        space_used: i64,
    ) -> Self {
        NewDiskUsage {
            mount: mount.into(),
            available_space,
            space_used,
        }
    }
}
