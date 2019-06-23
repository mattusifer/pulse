use serde::{Deserialize, Serialize};

use crate::services::news;

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
pub struct BroadcastEventKey(String);

impl From<String> for BroadcastEventKey {
    fn from(s: String) -> Self {
        BroadcastEventKey(s)
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum BroadcastEventType {
    HighDiskUsage,
    Newscast,
}

#[derive(Clone, Debug)]
pub enum BroadcastEvent {
    HighDiskUsage {
        filesystem_mount: String,
        current_usage: f64,
        max_usage: f64,
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

            BroadcastEvent::Newscast { new_york_times } => {
                ("News".to_string(), {
                    let sections = new_york_times
                        .iter()
                        .map(|section| {
                            let articles = section
                                .articles
                                .iter()
                                .map(|article| {
                                    format!(
                                        include_str!(
                                            "../../../resources/email/news/article.html"),
                                            url = article.url,
                                            title = article.title,
                                            publish_date = article.published_date,
                                            r#abstract = article.r#abstract
                                    )
                                })
                                .collect::<Vec<String>>()
                                .join("<br>");

                            format!(
                                include_str!("../../../resources/email/news/section.html"),
                                section_title = section.section_title,
                                articles = articles
                            )
                        })
                        .collect::<Vec<String>>()
                        .join("<br>");

                    format!(
                        include_str!(
                            "../../../resources/email/news/outline.html"
                        ),
                        title = "Digest",
                        sections = sections,
                        css = include_str!(
                            "../../../resources/email/news/style.css"
                        )
                    )
                })
            }
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

    /// Unique identifier for this event
    pub fn event_key(&self) -> BroadcastEventKey {
        match self {
            BroadcastEvent::HighDiskUsage {
                filesystem_mount, ..
            } => (serde_json::to_string(&self.event_type()).unwrap()
                + filesystem_mount)
                .into(),
            BroadcastEvent::Newscast { .. } => {
                serde_json::to_string(&self.event_type()).unwrap().into()
            }
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BroadcastMedium {
    Email,
}
