use diesel::{pg::data_types::*, Insertable, Queryable};

use crate::schema::messages;

#[derive(Queryable, Clone, Debug)]
pub struct Message {
    pub id: i32,
    pub message: String,
    pub sent_at: PgTimestamp,
}

#[derive(Insertable)]
#[table_name = "messages"]
pub struct NewMessage {
    pub message: String,
}

impl NewMessage {
    pub fn new<S: Into<String>>(message: S) -> Self {
        NewMessage {
            message: message.into(),
        }
    }
}
