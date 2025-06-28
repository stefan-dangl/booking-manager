use crate::{http::start_server, timeslot_manager::TimeslotManager};

mod http;
mod timeslot_manager;

#[derive(Clone)]
struct AppState {
    timeslot_manager: TimeslotManager,
}

#[tokio::main]
async fn main() {
    let timeslot_manager = TimeslotManager::default();
    let state = AppState { timeslot_manager };
    state.timeslot_manager.insert_example_timeslots();
    start_server(state).await;
}
