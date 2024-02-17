// @generated automatically by Diesel CLI.

diesel::table! {
    telegram_users (id) {
        id -> Integer,
        telegram_id -> BigInt,
        username -> Text,
        admin -> Bool,
    }
}
