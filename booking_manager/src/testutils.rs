use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use uuid::Uuid;

use crate::{backend::TimeslotBackend, types::Timeslot};

pub struct MockTimeslotBackendInner {
    pub success: AtomicBool,
    pub calls_to_insert_example_timeslots: AtomicU64,
    pub calls_to_timeslots: AtomicU64,
    pub calls_to_book_timeslot: AtomicU64,
    pub calls_to_add_timeslot: AtomicU64,
    pub calls_to_remove_timeslot: AtomicU64,
    pub calls_to_remove_all_timeslot: AtomicU64,
    pub timeslots: Mutex<HashMap<Uuid, Timeslot>>,
}

#[derive(Clone)]
pub struct MockTimeslotBackend(pub Arc<MockTimeslotBackendInner>);

impl MockTimeslotBackendInner {
    fn new() -> Self {
        Self {
            success: AtomicBool::new(true),
            calls_to_insert_example_timeslots: AtomicU64::default(),
            calls_to_timeslots: AtomicU64::default(),
            calls_to_book_timeslot: AtomicU64::default(),
            calls_to_add_timeslot: AtomicU64::default(),
            calls_to_remove_timeslot: AtomicU64::default(),
            calls_to_remove_all_timeslot: AtomicU64::default(),
            timeslots: Mutex::default(),
        }
    }
}

impl MockTimeslotBackend {
    pub fn new() -> Self {
        Self(Arc::new(MockTimeslotBackendInner::new()))
    }

    fn result(&self) -> Result<(), String> {
        match self.0.success.load(Ordering::SeqCst) {
            true => Ok(()),
            false => Err("Supposed to fail".into()),
        }
    }
}

impl TimeslotBackend for MockTimeslotBackend {
    fn insert_example_timeslots(&self) {
        self.0
            .calls_to_insert_example_timeslots
            .fetch_add(1, Ordering::SeqCst);
    }

    fn timeslots(&self) -> HashMap<Uuid, Timeslot> {
        self.0.calls_to_timeslots.fetch_add(1, Ordering::SeqCst);
        self.0.timeslots.lock().unwrap().clone()
    }

    fn book_timeslot(&self, _id: uuid::Uuid, _booker_name: String) -> Result<(), String> {
        self.0.calls_to_book_timeslot.fetch_add(1, Ordering::SeqCst);
        self.result()
    }

    fn add_timeslot(&self, _datetime: chrono::DateTime<chrono::Local>, _notes: String) {
        self.0.calls_to_add_timeslot.fetch_add(1, Ordering::SeqCst);
    }

    fn remove_timeslot(&self, _id: uuid::Uuid) -> Result<(), String> {
        self.0
            .calls_to_remove_timeslot
            .fetch_add(1, Ordering::SeqCst);
        self.result()
    }

    fn remove_all_timeslot(&self) {
        self.0
            .calls_to_remove_all_timeslot
            .fetch_add(1, Ordering::SeqCst);
    }
}
