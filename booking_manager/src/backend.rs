use crate::types::Timeslot;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub trait TimeslotBackend: Clone + Send + Sync + 'static {
    fn timeslots(&self) -> Vec<Timeslot>;
    fn book_timeslot(&self, id: Uuid, booker_name: String) -> Result<(), String>;
    fn add_timeslot(&self, datetime: DateTime<Utc>, notes: String);
    fn remove_timeslot(&self, id: Uuid) -> Result<(), String>;
    fn remove_all_timeslot(&self);
}
