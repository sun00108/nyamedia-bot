// @generated automatically by Diesel CLI.

diesel::table! {
    media (id) {
        id -> Integer,
        media_request_id -> Integer,
        title -> Text,
        summary -> Nullable<Text>,
        poster -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

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

diesel::joinable!(media -> media_requests (media_request_id));

diesel::allow_tables_to_appear_in_same_query!(
    media,
    media_requests,
    telegram_users,
);
