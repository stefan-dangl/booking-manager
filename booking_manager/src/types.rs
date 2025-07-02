use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeslot {
    pub id: Uuid,
    pub datetime: DateTime<Local>,
    pub available: bool,
    pub booker_name: String,
    pub notes: String,
}
