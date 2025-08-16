// @generated automatically by Diesel CLI.

diesel::table! {
    media_requests (id) {
        id -> Integer,
        source -> Text,
        media_id -> Text,
        request_user -> BigInt,
        status -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    telegram_users (id) {
        id -> Integer,
        telegram_id -> BigInt,
        username -> Text,
        admin -> Bool,
        emby_user_id -> Nullable<Text>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    media_requests,
    telegram_users,
);
