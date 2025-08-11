use crate::{backend::TimeslotBackend, types::Timeslot};
use chrono::{DateTime, Duration, Utc};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::watch::{self, Sender};
use tokio_stream::wrappers::WatchStream;
use tracing::error;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct LocalTimeslots {
    timeslots: Arc<Mutex<HashMap<Uuid, Timeslot>>>,
    sender: Sender<Vec<Timeslot>>,
}

impl LocalTimeslots {
    pub fn default() -> LocalTimeslots {
        let (sender, _) = watch::channel(vec![]);
        Self {
            timeslots: Arc::new(Mutex::default()),
            sender,
        }
    }

    fn cleanup_outdated_timeslots(&self, max_age: Duration) {
        let current_time = Utc::now();
        let cutoff_time = current_time - max_age;
        let mut timeslots = self.timeslots.lock().unwrap();

        timeslots.retain(|_, timeslot| timeslot.datetime >= cutoff_time);
    }

    fn timeslots(&self) -> Vec<Timeslot> {
        self.cleanup_outdated_timeslots(Duration::days(1));

        let mut timeslots: Vec<Timeslot> = self
            .timeslots
            .lock()
            .unwrap()
            .clone()
            .values()
            .cloned()
            .collect();
        timeslots.sort_unstable_by(|a, b| a.datetime.cmp(&b.datetime));
        timeslots
    }

    fn send_timeslots(&self) {
        let timeslots = self.timeslots();

        if let Err(err) = self.sender.send(timeslots) {
            error!(?err, "Failed to send current timeslots");
        }
    }
}

impl TimeslotBackend for LocalTimeslots {
    fn timeslot_stream(&self) -> WatchStream<Vec<Timeslot>> {
        let stream = WatchStream::new(self.sender.subscribe());
        self.send_timeslots();
        stream
    }

    fn book_timeslot(&self, id: Uuid, booker_name: String) -> Result<(), String> {
        if let Some(timeslot) = self.timeslots.lock().unwrap().get_mut(&id) {
            if !timeslot.available {
                let err = "Timeslot was already booked";
                error!(err);
                return Err(err.into());
            }
            if timeslot.datetime < Utc::now() {
                let err = "Timeslot already passed";
                error!(err);
                return Err(err.into());
            }
            timeslot.available = false;
            timeslot.booker_name = booker_name;
        } else {
            let err = "Timeslot does not exist and can't therefore not be booked";
            error!(err);
            return Err(err.into());
        }
        self.send_timeslots();
        Ok(())
    }

    fn add_timeslot(&self, datetime: DateTime<Utc>, notes: String) -> Result<(), String> {
        let id = Uuid::new_v4();
        self.timeslots.lock().unwrap().insert(
            id,
            Timeslot {
                id,
                datetime,
                available: true,
                booker_name: String::new(),
                notes,
            },
        );
        self.send_timeslots();
        Ok(())
    }

    fn remove_timeslot(&self, id: Uuid) -> Result<(), String> {
        if self.timeslots.lock().unwrap().remove(&id).is_none() {
            let err = "Timeslot does not exist and can't therefore not be removed";
            error!(err);
            return Err(err.into());
        }
        self.send_timeslots();
        Ok(())
    }

    fn remove_all_timeslot(&self) -> Result<(), String> {
        self.timeslots.lock().unwrap().clear();
        self.send_timeslots();
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        backend::TimeslotBackend, local_timeslots::LocalTimeslots,
        testutils::read_from_timeslot_stream,
    };

    #[tokio::test]
    async fn test_add_book_remove_single_timeslot() {
        let local_timeslots = LocalTimeslots::default();
        let mut timeslot_stream = local_timeslots.timeslot_stream();

        let datetime = Utc::now() + Duration::hours(1);
        let notes = String::from("First Timeslot");
        local_timeslots
            .add_timeslot(datetime, notes.clone())
            .unwrap();

        let timeslots = read_from_timeslot_stream(&mut timeslot_stream).await;
        let timeslot_id = timeslots[0].id;
        assert_eq!(timeslots.len(), 1);
        assert_eq!(timeslots[0].notes, notes);
        assert!(timeslots[0].available);
        assert_eq!(timeslots[0].booker_name, "");

        let booker_name = String::from("Stefan");
        local_timeslots
            .book_timeslot(timeslot_id, booker_name.clone())
            .unwrap();

        let timeslots = read_from_timeslot_stream(&mut timeslot_stream).await;
        assert_eq!(timeslots.len(), 1);
        assert!(!timeslots[0].available);
        assert_eq!(timeslots[0].booker_name, booker_name);

        let booker_name = String::from("Peter");
        local_timeslots
            .book_timeslot(timeslot_id, booker_name.clone())
            .unwrap_err();

        local_timeslots.remove_timeslot(timeslot_id).unwrap();
        let timeslots = read_from_timeslot_stream(&mut timeslot_stream).await;
        assert_eq!(timeslots.len(), 0);

        local_timeslots.remove_timeslot(timeslot_id).unwrap_err();
    }

    #[test]
    fn test_try_book_outdated_timeslot() {
        let local_timeslots = LocalTimeslots::default();

        let datetime = Utc::now() - Duration::hours(2);
        let notes = String::from("First Timeslot");
        local_timeslots
            .add_timeslot(datetime, notes.clone())
            .unwrap();

        let timeslots = local_timeslots.timeslots();
        let timeslot_id = timeslots[0].id;
        assert_eq!(timeslots.len(), 1);
        assert!(timeslots[0].available);

        let booker_name = String::from("Stefan");
        local_timeslots
            .book_timeslot(timeslot_id, booker_name.clone())
            .unwrap_err();
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

        local_timeslots
            .add_timeslot(datetime_1, notes_1.clone())
            .unwrap();
        local_timeslots
            .add_timeslot(datetime_2, notes_2.clone())
            .unwrap();
        local_timeslots
            .add_timeslot(datetime_3, notes_3.clone())
            .unwrap();

        local_timeslots.remove_timeslot(Uuid::new_v4()).unwrap_err(); // try to delete not existing timeslot
        let timeslots = local_timeslots.timeslots();
        assert_eq!(timeslots.len(), 3);

        local_timeslots.remove_timeslot(timeslots[0].id).unwrap();
        let timeslots = local_timeslots.timeslots();
        assert_eq!(timeslots.len(), 2);

        local_timeslots.remove_all_timeslot().unwrap();
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

        local_timeslots
            .add_timeslot(datetime_1, notes_1.clone())
            .unwrap();
        local_timeslots
            .add_timeslot(datetime_2, notes_2.clone())
            .unwrap();
        local_timeslots
            .add_timeslot(datetime_3, notes_3.clone())
            .unwrap();

        let timeslots = local_timeslots.timeslots();
        assert_eq!(timeslots.len(), 2);
        assert_eq!(timeslots[0].notes, "Seconds Timeslot");
        assert_eq!(timeslots[1].notes, "First Timeslot");
    }
}
