use crate::{backend::TimeslotBackend, http::start_server, timeslot_manager::TimeslotManager};

mod backend;
mod http;
mod timeslot_manager;
mod types;

#[derive(Clone)]
struct AppState<T: TimeslotBackend> {
    timeslot_manager: T,
}

#[tokio::main]
async fn main() {
    let timeslot_manager = TimeslotManager::default();
    let state = AppState { timeslot_manager };
    state.timeslot_manager.insert_example_timeslots();
    start_server(state).await;
}
