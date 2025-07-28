use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use tokio::sync::watch::{self, Sender};
use tokio_stream::{wrappers::WatchStream, StreamExt};
use uuid::Uuid;

use crate::{backend::TimeslotBackend, configuration::Configuration, types::Timeslot};

pub async fn read_from_timeslot_stream(
    timeslot_stream: &mut WatchStream<Vec<Timeslot>>,
) -> Vec<Timeslot> {
    tokio::time::timeout(
        std::time::Duration::from_millis(100),
        timeslot_stream.next(),
    )
    .await
    .unwrap()
    .unwrap()
}

pub struct MockTimeslotBackendInner {
    pub success: AtomicBool,
    pub calls_to_timeslots: AtomicU64,
    pub calls_to_book_timeslot: AtomicU64,
    pub calls_to_add_timeslot: AtomicU64,
    pub calls_to_remove_timeslot: AtomicU64,
    pub calls_to_remove_all_timeslot: AtomicU64,
    pub timeslot_sender: Sender<Vec<Timeslot>>,
}

#[derive(Clone)]
pub struct MockTimeslotBackend(pub Arc<MockTimeslotBackendInner>);

impl MockTimeslotBackendInner {
    fn new() -> Self {
        let (sender, _) = watch::channel(vec![]);
        Self {
            success: AtomicBool::new(true),
            calls_to_timeslots: AtomicU64::default(),
            calls_to_book_timeslot: AtomicU64::default(),
            calls_to_add_timeslot: AtomicU64::default(),
            calls_to_remove_timeslot: AtomicU64::default(),
            calls_to_remove_all_timeslot: AtomicU64::default(),
            timeslot_sender: sender,
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
    fn book_timeslot(&self, _id: uuid::Uuid, _booker_name: String) -> Result<(), String> {
        self.0.calls_to_book_timeslot.fetch_add(1, Ordering::SeqCst);
        self.result()
    }

    fn add_timeslot(
        &self,
        _datetime: chrono::DateTime<chrono::Utc>,
        _notes: String,
    ) -> Result<(), String> {
        self.0.calls_to_add_timeslot.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn remove_timeslot(&self, _id: uuid::Uuid) -> Result<(), String> {
        self.0
            .calls_to_remove_timeslot
            .fetch_add(1, Ordering::SeqCst);
        self.result()
    }

    fn remove_all_timeslot(&self) -> Result<(), String> {
        self.0
            .calls_to_remove_all_timeslot
            .fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn timeslot_stream(&self) -> tokio_stream::wrappers::WatchStream<Vec<Timeslot>> {
        WatchStream::new(self.0.timeslot_sender.subscribe())
    }
}

pub struct MockConfigurationInner {
    pub password: Mutex<String>,
    pub frontend_path: Mutex<PathBuf>,
}

impl MockConfigurationInner {
    fn new() -> Self {
        Self {
            password: Mutex::default(),
            frontend_path: Mutex::new(PathBuf::new()),
        }
    }
}

#[derive(Clone)]
pub struct MockConfiguration(pub Arc<MockConfigurationInner>);

impl MockConfiguration {
    pub fn new() -> Self {
        Self(Arc::new(MockConfigurationInner::new()))
    }
}

impl Configuration for MockConfiguration {
    fn password(&self) -> String {
        self.0.password.lock().unwrap().clone()
    }

    fn frontend_path(&self) -> PathBuf {
        self.0.frontend_path.lock().unwrap().clone()
    }

    fn port(&self) -> String {
        "1234".into()
    }

    fn database_url(&self) -> Option<String> {
        unimplemented!()
    }
}
