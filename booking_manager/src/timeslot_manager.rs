use crate::{backend::TimeslotBackend, types::Timeslot};
use chrono::{DateTime, Local};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct TimeslotManager {
    timeslots: Arc<Mutex<HashMap<Uuid, Timeslot>>>,
}

impl TimeslotBackend for TimeslotManager {
    // TODO: Remove
    fn insert_example_timeslots(&self) {
        const NUMBER_OF_EXAMPLES: i64 = 5;
        for i in 1..=NUMBER_OF_EXAMPLES {
            let datetime = Local::now() + chrono::Duration::days(i);
            self.add_timeslot(datetime, "Example Slot".into());
        }
    }

    fn timeslots(&self) -> Vec<Timeslot> {
        self.timeslots
            .lock()
            .unwrap()
            .clone()
            .values()
            .cloned()
            .collect()
    }

    fn book_timeslot(&self, id: Uuid, booker_name: String) -> Result<(), String> {
        let mut timeslots = self.timeslots.lock().unwrap();
        match timeslots.get_mut(&id) {
            Some(timeslot) => {
                if !timeslot.available {
                    return Err("Timeslot was already booked".into());
                }
                timeslot.available = false;
                timeslot.booker_name = booker_name
            }
            None => return Err("Timeslot does not exist and can't therefore not be booked".into()),
        }
        Ok(())
    }

    fn add_timeslot(&self, datetime: DateTime<Local>, notes: String) {
        println!("ACTUAL BACKEND CALLED");

        let id = Uuid::new_v4();
        // TODO_SD: Check if id is not yet in HashMap
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

    fn remove_timeslot(&self, id: Uuid) -> Result<(), String> {
        let mut timeslots = self.timeslots.lock().unwrap();
        if timeslots.remove(&id).is_none() {
            return Err("Timeslot does not exist and can't therefore not be removed".into());
        }
        Ok(())
    }

    fn remove_all_timeslot(&self) {
        let mut timeslots = self.timeslots.lock().unwrap();
        timeslots.clear();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{backend::TimeslotBackend, timeslot_manager::TimeslotManager};
    use chrono::Local;

    #[test]
    fn test_add_book_remove_single_timeslot() {
        let timeslot_manager = TimeslotManager::default();

        let datetime = Local::now();
        let notes = String::from("First Timeslot");
        timeslot_manager.add_timeslot(datetime, notes.clone());

        let timeslots = timeslot_manager.timeslots();
        let timeslot_id = timeslots[0].id;
        assert_eq!(timeslots.len(), 1);
        assert_eq!(timeslots[0].notes, notes);
        assert!(timeslots[0].available);
        assert_eq!(timeslots[0].booker_name, "");

        let booker_name = String::from("Stefan");
        timeslot_manager
            .book_timeslot(timeslot_id, booker_name.clone())
            .unwrap();

        let timeslots = timeslot_manager.timeslots();
        assert_eq!(timeslots.len(), 1);
        assert!(!timeslots[0].available);
        assert_eq!(timeslots[0].booker_name, booker_name);

        let booker_name = String::from("Peter");
        timeslot_manager
            .book_timeslot(timeslot_id, booker_name.clone())
            .unwrap_err();

        timeslot_manager.remove_timeslot(timeslot_id).unwrap();
        let timeslots = timeslot_manager.timeslots();
        assert_eq!(timeslots.len(), 0);

        timeslot_manager.remove_timeslot(timeslot_id).unwrap_err();
    }

    #[test]
    fn test_remove_multiple_timeslots() {
        let timeslot_manager = TimeslotManager::default();

        let datetime_1 = Local::now();
        let notes_1 = String::from("First Timeslot");
        let datetime_2 = Local::now();
        let notes_2 = String::from("Seconds Timeslot");
        let datetime_3 = Local::now();
        let notes_3 = String::from("Third Timeslot");

        timeslot_manager.add_timeslot(datetime_1, notes_1.clone());
        timeslot_manager.add_timeslot(datetime_2, notes_2.clone());
        timeslot_manager.add_timeslot(datetime_3, notes_3.clone());
        let timeslots = timeslot_manager.timeslots();
        assert_eq!(timeslots.len(), 3);

        timeslot_manager.remove_timeslot(timeslots[0].id).unwrap();
        let timeslots = timeslot_manager.timeslots();
        assert_eq!(timeslots.len(), 2);

        timeslot_manager.remove_all_timeslot();
        let timeslots = timeslot_manager.timeslots();
        assert_eq!(timeslots.len(), 0);
    }
}
