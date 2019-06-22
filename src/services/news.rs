use actix::prelude::*;
use chrono::NaiveDate;
use nytrs::NewYorkTimes;

use super::{
    broadcast::OUTBOX,
    messages::{BroadcastEvent, ScheduledTaskMessage},
};
use crate::{
    config::{config, NewsConfig},
    error::Result,
};

#[derive(Clone, Debug)]
pub struct ArticleSection {
    pub section_title: String,
    pub articles: Vec<Article>,
}

#[derive(Clone, Debug)]
pub struct Article {
    pub url: String,
    pub published_date: NaiveDate,
    pub title: String,
    pub r#abstract: String,
    pub metric: String,
}

pub struct News {
    config: NewsConfig,
}

impl News {
    pub fn new() -> Self {
        let config = config().news.unwrap();

        Self { config }
    }

    fn build_new_york_times_articles(&self) -> Result<Vec<ArticleSection>> {
        self.config
            .new_york_times
            .iter()
            .flat_map(|nyt_config| {
                let new_york_times =
                    NewYorkTimes::new(nyt_config.api_key.clone());

                nyt_config.most_popular_viewed_period.iter().map(
                    move |period| {
                        new_york_times
                            .most_popular_viewed(period.clone())
                            .map(|response| ArticleSection {
                                section_title: "Most Viewed".to_string(),
                                articles: response
                                    .results
                                    .into_iter()
                                    .map(|article| Article {
                                        url: article.url,
                                        published_date:
                                            NaiveDate::parse_from_str(
                                                &article.published_date,
                                                "%Y-%m-%d",
                                            )
                                            .unwrap(),
                                        title: article.title,
                                        r#abstract: article.r#abstract,
                                        metric: format!(
                                            "{} views",
                                            article.views
                                        ),
                                    })
                                    .collect(),
                            })
                            .map_err(Into::into)
                    },
                )
            })
            .collect()
    }

    fn build_newscast(&self) -> Result<()> {
        let message = BroadcastEvent::Newscast {
            new_york_times: self.build_new_york_times_articles()?,
        };

        OUTBOX.push(message)?;

        Ok(())
    }
}

impl Actor for News {
    type Context = Context<Self>;
}

impl Handler<ScheduledTaskMessage> for News {
    type Result = Result<()>;

    fn handle(
        &mut self,
        msg: ScheduledTaskMessage,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        match msg {
            ScheduledTaskMessage::FetchNews => self.build_newscast(),
        }
    }
}
