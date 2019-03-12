use std::sync::{Arc, Mutex};

use bincode::{deserialize, serialize};
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use sled::Db;

use crate::error::{Error, Result};

const DB_PATH: &str = "/home/matt/.pulse/db";

lazy_static! {
    pub static ref DB: Arc<Mutex<Db>> =
        Arc::new(Mutex::new(Db::start_default(DB_PATH).unwrap()));
}

pub fn insert_now<S: AsRef<[u8]>>(key: S) -> Result<()> {
    let formatted_date = &format!("{:?}", chrono::Utc::now());
    let serialized = serialize(formatted_date)?;
    DB.lock().unwrap().set(key, serialized)?;

    Ok(())
}

pub fn get_date<S: AsRef<[u8]>>(key: S) -> Result<Option<DateTime<Utc>>> {
    let bytes = crate::db::DB.lock().unwrap().get(key)?;

    if let Some(bytes) = bytes {
        if let sled::IVec::Remote { buf } = bytes {
            let date: String = deserialize(&buf)?;
            Ok(Some(date.parse()?))
        } else {
            Err(Error::bincode_error(
                "Expected Remote ivec, received Inline",
            ))
        }
    } else {
        Ok(None)
    }
}

macro_rules! broadcast {
    (
        $delay: expr,
        $operation_id:expr,
        $alerts:expr,
        $broadcaster:expr
    ) => {{
        let key = format!("broadcast_{}", $operation_id);
        let last_broadcast = crate::db::get_date(key.clone())?;

        if (last_broadcast.is_some()
            && chrono::Utc::now()
                .signed_duration_since(last_broadcast.unwrap())
                >= $delay)
            || last_broadcast.is_none()
        {
            crate::db::insert_now(key)?;
            for alert in $alerts {
                $broadcaster(alert)?;
            }
        }

        Ok(())
    }};
}
