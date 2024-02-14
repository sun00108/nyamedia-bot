// @generated automatically by Diesel CLI.

diesel::table! {
    telegram_users (id) {
        id -> Nullable<Integer>,
        telegram_id -> Integer,
        username -> Nullable<Text>,
        created_at -> Nullable<Timestamp>,
    }
}
