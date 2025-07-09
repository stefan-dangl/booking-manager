use crate::schema::timeslots;
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Queryable, AsChangeset)]
pub struct Timeslot {
    pub id: Uuid,
    pub datetime: DateTime<Utc>,
    pub available: bool,
    pub booker_name: String,
    pub notes: String,
}
