table! {
    disk_usage (id) {
        id -> Int4,
        mount -> Varchar,
        percent_disk_used -> Float8,
        recorded_at -> Timestamptz,
    }
}

table! {
    tasks (id) {
        id -> Int4,
        task -> Varchar,
        sent_at -> Timestamptz,
    }
}

table! {
    tweets (id) {
        id -> Int4,
        twitter_tweet_id -> Varchar,
        group_name -> Array<Text>,
        latitude -> Nullable<Float8>,
        longitude -> Nullable<Float8>,
        favorite_count -> Int4,
        retweet_count -> Int4,
        username -> Nullable<Varchar>,
        lang -> Nullable<Varchar>,
        text -> Varchar,
        tweeted_at -> Timestamptz,
    }
}

allow_tables_to_appear_in_same_query!(disk_usage, tasks, tweets,);
