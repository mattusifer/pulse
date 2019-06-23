use actix::Message;
use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ScheduledStreamMessage {
    CheckDiskUsage,
}
impl Message for ScheduledStreamMessage {
    type Result = Result<()>;
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ScheduledTaskMessage {
    FetchNews,
}
impl Message for ScheduledTaskMessage {
    type Result = Result<()>;
}
