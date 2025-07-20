use crate::schema::timeslots::dsl::*;
use crate::types::Timeslot;
use crate::{backend::TimeslotBackend, schema::timeslots};
use chrono::{DateTime, Utc};
use diesel::{Connection, ConnectionError, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Insertable)]
#[table_name = "timeslots"]
pub struct NewTimeslot {
    pub datetime: DateTime<Utc>,
    pub notes: String,
}

#[derive(Clone)]
pub struct DatabaseInterface {
    connection: Arc<Mutex<PgConnection>>,
}

impl DatabaseInterface {
    pub fn new(database_url: &str) -> Result<Self, ConnectionError> {
        let connection = Self::establish_connection(database_url)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    fn establish_connection(database_url: &str) -> Result<PgConnection, diesel::ConnectionError> {
        PgConnection::establish(database_url)
    }
}

impl TimeslotBackend for DatabaseInterface {
    fn timeslots(&self) -> Vec<Timeslot> {
        let mut connection = self.connection.lock().unwrap();

        diesel::sql_query("DELETE FROM timeslots WHERE datetime < (NOW() - INTERVAL '1 day')")
            .execute(&mut *connection)
            .unwrap_or_else(|err| {
                eprintln!("Cleanup failed: {}", err);
                0
            });

        let result = timeslots.load::<Timeslot>(&mut *connection);
        match result {
            Ok(result) => result,
            Err(err) => {
                println!("{err} Failed to read timeslots from Database");
                vec![]
            }
        }
    }

    fn book_timeslot(&self, timeslot_id: Uuid, new_booker_name: String) -> Result<(), String> {
        let mut connection = self.connection.lock().unwrap();
        let result = diesel::update(timeslots::table.find(timeslot_id))
            .set((available.eq(false), booker_name.eq(new_booker_name)))
            .execute(&mut *connection);

        if let Err(err) = result {
            println!("{err} Timeslot can't be booked");
            return Err("Database Error. Timeslot can't be booked".into());
        }
        Ok(())
    }

    fn add_timeslot(&self, new_datetime: DateTime<Utc>, new_notes: String) {
        let mut connection = self.connection.lock().unwrap();

        let timeslot = NewTimeslot {
            datetime: new_datetime,
            notes: new_notes,
        };

        let result = diesel::insert_into(timeslots::table)
            .values(&timeslot)
            .execute(&mut *connection);

        if let Err(err) = result {
            println!("{err} Timeslot can't be added");
        }
    }

    fn remove_timeslot(&self, new_id: Uuid) -> Result<(), String> {
        let mut connection = self.connection.lock().unwrap();
        let result = diesel::delete(timeslots::table.find(new_id)).execute(&mut *connection);

        match result {
            Ok(0) => {
                println!("Deletion failed. 0 database lines were changed");
                return Err("Database Error. Deletion of timeslot failed".into());
            }
            Ok(_) => Ok(()),
            Err(err) => {
                println!("{err} Deletion of timeslot failed");
                return Err("Database Error. Deletion of timeslot failed".into());
            }
        }
    }

    fn remove_all_timeslot(&self) {
        let mut connection = self.connection.lock().unwrap();
        let result = diesel::delete(timeslots::table).execute(&mut *connection);

        if let Err(err) = result {
            println!("{err} Failed to clear Database");
        }
    }
}

#[cfg(test)]
mod test {
    //! # Integration Tests for Timeslot Booking
    //!
    //! ATTENTION: Running any of these tests leads to an cleared database!!!
    //!
    //! ## Database Requirements
    //! Test requirements:
    //! 1. A running PostgreSQL server
    //! 2. Database connection URL: `postgres://username:password@localhost/booking_manager`
    //! 3. Proper table schema (run migrations first)
    //!  
    //! More information can be found in README.md

    use super::*;
    use chrono::Duration;

    const TEST_DATABASE_URL: &str = "postgres://username:password@localhost/booking_manager";

    #[test]
    fn test_add_book_remove_single_timeslot() {
        let database_interface = DatabaseInterface::new(TEST_DATABASE_URL).unwrap();
        database_interface.remove_all_timeslot();
        let current_timeslots = database_interface.timeslots();
        assert_eq!(current_timeslots.len(), 0);

        let current_time = Utc::now();
        let example_notes = "Test timeslot";
        database_interface.add_timeslot(current_time, example_notes.into());

        let current_timeslots = database_interface.timeslots();
        assert_eq!(current_timeslots.len(), 1);
        assert_eq!(current_timeslots[0].available, true);
        assert_eq!(current_timeslots[0].booker_name, "");
        let new_timeslot_id = current_timeslots[0].id;

        database_interface
            .book_timeslot(new_timeslot_id, "Stefan".into())
            .unwrap();

        let current_timeslots = database_interface.timeslots();
        assert_eq!(current_timeslots.len(), 1);
        assert_eq!(current_timeslots[0].available, false);
        assert_eq!(current_timeslots[0].booker_name, "Stefan");
        assert_eq!(current_timeslots[0].id, new_timeslot_id);

        database_interface
            .book_timeslot(new_timeslot_id, "Peter".into())
            .unwrap_err();

        database_interface.remove_timeslot(new_timeslot_id).unwrap();
        let current_timeslots = database_interface.timeslots();
        assert_eq!(current_timeslots.len(), 0);
    }

    #[test]
    fn test_remove_multiple_timeslots() {
        let database_interface = DatabaseInterface::new(TEST_DATABASE_URL).unwrap();
        database_interface.remove_all_timeslot();

        let datetime_1 = Utc::now();
        let notes_1 = String::from("First Timeslot");
        let datetime_2 = Utc::now();
        let notes_2 = String::from("Seconds Timeslot");
        let datetime_3 = Utc::now();
        let notes_3 = String::from("Third Timeslot");

        database_interface.add_timeslot(datetime_1, notes_1);
        database_interface.add_timeslot(datetime_2, notes_2);
        database_interface.add_timeslot(datetime_3, notes_3);

        database_interface // try to delete not existing timeslot
            .remove_timeslot(Uuid::new_v4())
            .unwrap_err();
        let current_timeslots = database_interface.timeslots();
        assert_eq!(current_timeslots.len(), 3);

        database_interface
            .remove_timeslot(current_timeslots[0].id)
            .unwrap();
        let current_timeslots = database_interface.timeslots();
        assert_eq!(current_timeslots.len(), 2);

        database_interface.remove_all_timeslot();
        let current_timeslots = database_interface.timeslots();
        assert_eq!(current_timeslots.len(), 0);
    }

    #[test]
    fn test_database_persistency() {
        let database_interface = DatabaseInterface::new(TEST_DATABASE_URL).unwrap();
        database_interface.remove_all_timeslot();

        let datetime_1 = Utc::now();
        let notes_1 = String::from("First Timeslot");
        let datetime_2 = Utc::now();
        let notes_2 = String::from("Seconds Timeslot");
        let datetime_3 = Utc::now();
        let notes_3 = String::from("Third Timeslot");

        database_interface.add_timeslot(datetime_1, notes_1);
        database_interface.add_timeslot(datetime_2, notes_2);
        database_interface.add_timeslot(datetime_3, notes_3);

        let current_timeslots = database_interface.timeslots();
        assert_eq!(current_timeslots.len(), 3);

        drop(database_interface);

        let database_interface = DatabaseInterface::new(TEST_DATABASE_URL).unwrap();
        let current_timeslots = database_interface.timeslots();
        assert_eq!(current_timeslots.len(), 3);
        database_interface.remove_all_timeslot();
    }

    #[test]
    fn cleanup_outdated_timeslots() {
        let database_interface = DatabaseInterface::new(TEST_DATABASE_URL).unwrap();
        database_interface.remove_all_timeslot();

        let datetime_1 = Utc::now();
        let notes_1 = String::from("First Timeslot");
        let datetime_2 = Utc::now() - Duration::hours(2);
        let notes_2 = String::from("Seconds Timeslot");
        let datetime_3 = Utc::now() - Duration::days(2);
        let notes_3 = String::from("Third Timeslot");

        database_interface.add_timeslot(datetime_1, notes_1);
        database_interface.add_timeslot(datetime_2, notes_2);
        database_interface.add_timeslot(datetime_3, notes_3);

        let current_timeslots = database_interface.timeslots();
        assert_eq!(current_timeslots.len(), 2);

        let mut expected_notes = vec!["First Timeslot", "Seconds Timeslot"];
        for timeslot in current_timeslots {
            let index = expected_notes
                .iter()
                .position(|&x| x == &timeslot.notes)
                .unwrap();
            expected_notes.remove(index);
        }
    }
}
