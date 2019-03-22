use actix::prelude::*;
use serde::{Deserialize, Serialize};

use super::news;
use crate::error::Result;

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ScheduleMessage {
    CheckDiskUsage,
    FetchNews,
}

impl Message for ScheduleMessage {
    type Result = Result<()>;
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BroadcastEventType {
    HighDiskUsage,
    Newscast,
}

#[derive(Clone, Debug)]
pub enum BroadcastEvent {
    HighDiskUsage {
        filesystem_mount: String,
        current_usage: f32,
        max_usage: f32,
    },
    Newscast {
        new_york_times: Vec<news::ArticleSection>,
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

            BroadcastEvent::Newscast { new_york_times } => (
                "News".to_string(),
                new_york_times
                    .iter()
                    .map(|section| {
                        format!("<h2>{}</h2>", section.section_title)
                            + &section
                                .articles
                                .iter()
                                .map(|article| {
                                    format!(
                                        r#"<strong>{}</strong> ({})<br><a href="{url}">{url}</a><br>"#,
                                        article.title, article.published_date, url = article.url
                                    )
                                })
                                .collect::<Vec<String>>()
                                .join("<br>")
                    })
                    .collect::<Vec<String>>()
                    .join("<br>"),
            ),
        }
    }

    pub fn event_type(&self) -> BroadcastEventType {
        match self {
            BroadcastEvent::HighDiskUsage { .. } => {
                BroadcastEventType::HighDiskUsage
            }
            BroadcastEvent::Newscast { .. } => BroadcastEventType::Newscast,
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BroadcastMedium {
    Email,
}
