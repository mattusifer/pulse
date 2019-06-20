table! {
    disk_usage (id) {
        id -> Int4,
        mount -> Varchar,
        percent_disk_used -> Float8,
        recorded_at -> Timestamptz,
    }
}

table! {
    schedules (id) {
        id -> Int4,
        message -> Varchar,
        schedule -> Text,
    }
}

table! {
    tasks (id) {
        id -> Int4,
        task -> Varchar,
        sent_at -> Timestamptz,
    }
}

allow_tables_to_appear_in_same_query!(
    disk_usage,
    schedules,
    tasks,
);
