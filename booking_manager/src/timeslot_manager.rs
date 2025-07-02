use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct TimeslotManager {
    timeslots: Arc<Mutex<HashMap<Uuid, Timeslot>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeslot {
    pub id: Uuid,
    pub datetime: DateTime<Local>,
    pub available: bool,
    pub booker_name: String,
    pub notes: String,
}

impl TimeslotManager {
    pub fn insert_example_timeslots(&self) {
        const NUMBER_OF_EXAMPLES: i64 = 5;
        for i in 1..=NUMBER_OF_EXAMPLES {
            let datetime = Local::now() + chrono::Duration::days(i);
            self.add_timeslot(datetime, "Example Slot".into());
        }
    }

    pub fn timeslots(&self) -> Arc<Mutex<HashMap<Uuid, Timeslot>>> {
        self.timeslots.clone()
    }

    pub fn book_timeslot(&self, id: Uuid, booker_name: String) -> Result<(), String> {
        let mut timeslots = self.timeslots.lock().unwrap();
        match timeslots.get_mut(&id) {
            Some(timeslot) => {
                timeslot.available = false;
                timeslot.booker_name = booker_name
            }
            None => return Err("Timeslot does not exist and can't therefore not be booked".into()),
        }
        Ok(())
    }

    pub fn add_timeslot(&self, datetime: DateTime<Local>, notes: String) {
        let id = Uuid::new_v4();
        let mut timeslots = self.timeslots.lock().unwrap();
        timeslots.insert(
            id,
            Timeslot {
                id,
                datetime,
                available: true,
                booker_name: String::new(),
                notes,
            },
        );
    }

    pub fn remove_timeslot(&self, id: Uuid) -> Result<(), String> {
        let mut timeslots = self.timeslots.lock().unwrap();
        if timeslots.remove(&id).is_none() {
            return Err("Timeslot does not exist and can't therefore not be removed".into());
        }
        Ok(())
    }

    pub fn remove_all_timeslot(&self) {
        let mut timeslots = self.timeslots.lock().unwrap();
        timeslots.clear();
    }
}
