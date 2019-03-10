use actix::prelude::*;
use serde::Deserialize;

use crate::error::Result;

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum ScheduleMessage {
    CheckDiskUsage,
}
impl Message for ScheduleMessage {
    type Result = Result<()>;
}
