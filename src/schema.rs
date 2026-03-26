// @generated automatically by Diesel CLI.

diesel::table! {
    cli_login_challenges (id) {
        id -> Integer,
        state -> Text,
        client_id -> Text,
        status -> Text,
        source -> Nullable<Text>,
        telegram_user_id -> Nullable<BigInt>,
        telegram_username -> Nullable<Text>,
        authorization_code_jti -> Nullable<Text>,
        request_ip -> Nullable<Text>,
        completed_ip -> Nullable<Text>,
        user_agent -> Nullable<Text>,
        created_at -> Text,
        completed_at -> Nullable<Text>,
        expires_at -> Text,
        consumed_at -> Nullable<Text>,
    }
}

diesel::table! {
    media_upload_requests (id) {
        id -> Integer,
        media_request_id -> Integer,
        request_user -> BigInt,
        request_code -> Text,
        media_title -> Text,
        season -> Nullable<Integer>,
        episode -> Nullable<Integer>,
        target_path -> Text,
        status -> Text,
        uploaded_file_name -> Nullable<Text>,
        created_at -> Text,
        completed_at -> Nullable<Text>,
    }
}

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
diesel::joinable!(media_upload_requests -> media_requests (media_request_id));

diesel::allow_tables_to_appear_in_same_query!(
    cli_login_challenges,
    media,
    media_upload_requests,
    media_requests,
    telegram_users,
);
