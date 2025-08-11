// @generated automatically by Diesel CLI.

diesel::table! {
    timeslots (id) {
        id -> Uuid,
        datetime -> Timestamptz,
        available -> Bool,
        booker_name -> Varchar,
        notes -> Varchar,
    }
}
