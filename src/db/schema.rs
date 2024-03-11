// @generated automatically by Diesel CLI.

diesel::table! {
    collection_file_pairs (collection_id, file_id) {
        collection_id -> Uuid,
        file_id -> Uuid,
    }
}

diesel::table! {
    collections (id) {
        id -> Uuid,
        name -> Text,
        description -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    files (id) {
        id -> Uuid,
        name -> Text,
        mime -> Nullable<Text>,
        size -> Nullable<Int8>,
        hash -> Nullable<Int8>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    user_sessions (token) {
        token -> Text,
        user_id -> Int4,
        created_at -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        username -> Text,
        email -> Text,
        password -> Text,
        joined_at -> Timestamp,
    }
}

diesel::joinable!(collection_file_pairs -> collections (collection_id));
diesel::joinable!(collection_file_pairs -> files (file_id));
diesel::joinable!(user_sessions -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    collection_file_pairs,
    collections,
    files,
    user_sessions,
    users,
);
