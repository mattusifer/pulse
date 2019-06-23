use actix::Message;
use chrono::NaiveDateTime;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::schema::{disk_usage, tasks};

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
