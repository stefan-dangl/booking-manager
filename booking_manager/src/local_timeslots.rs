use crate::{backend::TimeslotBackend, types::Timeslot};
use chrono::{DateTime, Duration, Utc};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct LocalTimeslots {
    timeslots: Arc<Mutex<HashMap<Uuid, Timeslot>>>,
}

impl LocalTimeslots {
    fn cleanup_outdated_timeslots(&self, max_age: Duration) {
        let current_time = Utc::now();
        let cutoff_time = current_time - max_age;
        let mut timeslots = self.timeslots.lock().unwrap();

        timeslots.retain(|_, timeslot| timeslot.datetime >= cutoff_time);
    }
}

impl TimeslotBackend for LocalTimeslots {
    fn timeslots(&self) -> Vec<Timeslot> {
        println!("LOAD TIMESLOTS");

        self.cleanup_outdated_timeslots(Duration::days(1));

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

    fn add_timeslot(&self, datetime: DateTime<Utc>, notes: String) {
        println!("ACTUAL BACKEND CALLED");

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
    use crate::{backend::TimeslotBackend, local_timeslots::LocalTimeslots};

    #[test]
    fn test_add_book_remove_single_timeslot() {
        let local_timeslots = LocalTimeslots::default();

        let datetime = Utc::now();
        let notes = String::from("First Timeslot");
        local_timeslots.add_timeslot(datetime, notes.clone());

        let timeslots = local_timeslots.timeslots();
        let timeslot_id = timeslots[0].id;
        assert_eq!(timeslots.len(), 1);
        assert_eq!(timeslots[0].notes, notes);
        assert!(timeslots[0].available);
        assert_eq!(timeslots[0].booker_name, "");

        let booker_name = String::from("Stefan");
        local_timeslots
            .book_timeslot(timeslot_id, booker_name.clone())
            .unwrap();

        let timeslots = local_timeslots.timeslots();
        assert_eq!(timeslots.len(), 1);
        assert!(!timeslots[0].available);
        assert_eq!(timeslots[0].booker_name, booker_name);

        let booker_name = String::from("Peter");
        local_timeslots
            .book_timeslot(timeslot_id, booker_name.clone())
            .unwrap_err();

        local_timeslots.remove_timeslot(timeslot_id).unwrap();
        let timeslots = local_timeslots.timeslots();
        assert_eq!(timeslots.len(), 0);

        local_timeslots.remove_timeslot(timeslot_id).unwrap_err();
    }

    #[test]
    fn test_remove_multiple_timeslots() {
        let local_timeslots = LocalTimeslots::default();

        let datetime_1 = Utc::now();
        let notes_1 = String::from("First Timeslot");
        let datetime_2 = Utc::now();
        let notes_2 = String::from("Seconds Timeslot");
        let datetime_3 = Utc::now();
        let notes_3 = String::from("Third Timeslot");

        local_timeslots.add_timeslot(datetime_1, notes_1.clone());
        local_timeslots.add_timeslot(datetime_2, notes_2.clone());
        local_timeslots.add_timeslot(datetime_3, notes_3.clone());

        local_timeslots.remove_timeslot(Uuid::new_v4()).unwrap_err(); // try to delete not existing timeslot
        let timeslots = local_timeslots.timeslots();
        assert_eq!(timeslots.len(), 3);

        local_timeslots.remove_timeslot(timeslots[0].id).unwrap();
        let timeslots = local_timeslots.timeslots();
        assert_eq!(timeslots.len(), 2);

        local_timeslots.remove_all_timeslot();
        let timeslots = local_timeslots.timeslots();
        assert_eq!(timeslots.len(), 0);
    }

    #[test]
    fn cleanup_outdated_timeslots() {
        let local_timeslots = LocalTimeslots::default();

        let datetime_1 = Utc::now();
        let notes_1 = String::from("First Timeslot");
        let datetime_2 = Utc::now() - Duration::hours(2);
        let notes_2 = String::from("Seconds Timeslot");
        let datetime_3 = Utc::now() - Duration::days(2);
        let notes_3 = String::from("Third Timeslot");

        local_timeslots.add_timeslot(datetime_1, notes_1.clone());
        local_timeslots.add_timeslot(datetime_2, notes_2.clone());
        local_timeslots.add_timeslot(datetime_3, notes_3.clone());
        assert_eq!(local_timeslots.timeslots.lock().unwrap().len(), 3);

        local_timeslots.cleanup_outdated_timeslots(Duration::days(1));
        let timeslots = local_timeslots.timeslots.lock().unwrap();
        assert_eq!(timeslots.len(), 2);

        let mut expected_notes = vec!["First Timeslot", "Seconds Timeslot"];
        for (_, timeslot) in &*timeslots {
            let index = expected_notes
                .iter()
                .position(|&x| x == &timeslot.notes)
                .unwrap();
            expected_notes.remove(index);
        }
    }
}
