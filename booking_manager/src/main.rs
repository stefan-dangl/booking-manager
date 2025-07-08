use crate::{
    backend::TimeslotBackend, configuration::Configuration,
    configuration_handler::ConfigurationHandler, http::create_app,
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
    // TODO_SD: Add argument parsing

    let timeslot_manager = TimeslotManager::default();
    let configuration_handler = ConfigurationHandler {};
    let state = AppState {
        timeslot_manager,
        configuration_handler,
    };
    state.timeslot_manager.insert_example_timeslots();
    let app = create_app(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
