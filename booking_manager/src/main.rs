use crate::{
    backend::TimeslotBackend, configuration::Configuration,
    configuration_handler::ConfigurationHandler, http::start_server,
    timeslot_manager::TimeslotManager,
};

mod backend;
mod configuration;
mod configuration_handler;
mod http;
#[cfg(test)]
mod testutils;
mod timeslot_manager;
mod types;

#[derive(Clone)]
pub struct AppState<T: TimeslotBackend, S: Configuration> {
    pub timeslot_manager: T,
    pub configuration_handler: S,
}

#[tokio::main]
async fn main() {
    let timeslot_manager = TimeslotManager::default();
    let configuration_handler = ConfigurationHandler {};
    let state = AppState {
        timeslot_manager,
        configuration_handler,
    };
    state.timeslot_manager.insert_example_timeslots();
    start_server(state).await;
}
