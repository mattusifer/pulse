use actix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum ScheduleMessage {
    CheckDiskUsage,
}

impl Message for ScheduleMessage {
    type Result = Result<()>;
}
