use actix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ScheduleMessage {
    CheckDiskUsage,
}

impl Message for ScheduleMessage {
    type Result = Result<()>;
}

#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum BroadcastEventType {
    HighDiskUsage,
}

#[derive(Debug)]
pub enum BroadcastEvent {
    HighDiskUsage {
        filesystem_mount: String,
        current_usage: f32,
        max_usage: f32,
    },
}

impl BroadcastEvent {
    pub fn subject_and_body(&self) -> (String, String) {
        match self {
            BroadcastEvent::HighDiskUsage {
                filesystem_mount,
                current_usage,
                max_usage,
            } => (
                "High Disk Usage".to_string(),
                format!(
                    "Filesystem mounted at {} has {:.2}% disk usage, \
                     which is above the max of {:.2}",
                    filesystem_mount, current_usage, max_usage
                )
                .to_string(),
            ),
        }
    }

    pub fn event_type(&self) -> BroadcastEventType {
        match self {
            BroadcastEvent::HighDiskUsage { .. } => {
                BroadcastEventType::HighDiskUsage
            }
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BroadcastMedium {
    Email,
}
