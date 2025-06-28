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
}

impl TimeslotManager {
    pub fn insert_example_timeslots(&self) {
        const NUMBER_OF_EXAMPLES: i64 = 5;
        for i in 1..=NUMBER_OF_EXAMPLES {
            let datetime = Local::now() + chrono::Duration::days(i);
            self.add_timeslot(datetime);
        }
    }

    pub fn timeslots(&self) -> Arc<Mutex<HashMap<Uuid, Timeslot>>> {
        self.timeslots.clone()
    }

    pub fn add_timeslot(&self, datetime: DateTime<Local>) {
        let id = Uuid::new_v4();
        let mut timeslots = self.timeslots.lock().unwrap();
        timeslots.insert(
            id,
            Timeslot {
                id,
                datetime,
                available: true,
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
