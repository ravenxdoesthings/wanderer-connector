// Manual schema definition (no migrations)
// This assumes you have a simple users table in your PostgreSQL database

diesel::table! {
    users (id) {
        id -> Uuid,
        name -> Varchar,
        email -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    map_system_v1 (id) {
        id -> Uuid,
        solar_system_id -> Int8,
        name -> Text,
        custom_name -> Text,
        description -> Text,
        tag -> Text,
        labels -> Text,
        status -> Int8,
        visible -> Bool,
        locked -> Bool,
        position_x -> Int8,
        position_y -> Int8,
        added_at -> Timestamptz,
        inserted_at -> Timestamptz,
        updated_at -> Timestamptz,
        map_id -> Uuid,
        temporary_name -> Text,
        linked_sig_eve_id -> Text,
    }
}

// If you have other tables, add them here
// diesel::table! {
//     posts (id) {
//         id -> Int4,
//         title -> Varchar,
//         body -> Text,
//         user_id -> Uuid,
//         created_at -> Timestamptz,
//     }
// }

// Define relationships if needed
// diesel::joinable!(posts -> users (user_id));

// Allow tables to appear in the same query
diesel::allow_tables_to_appear_in_same_query!(
    users,
    // posts,
);
