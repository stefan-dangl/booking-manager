#[macro_use]
extern crate diesel;
use crate::{
    configuration::Configuration, configuration_handler::ConfigurationHandler,
    database_interface::DatabaseInterface, http::create_app, local_timeslots::LocalTimeslots,
};

mod backend;
mod configuration;
mod configuration_handler;
mod database_interface;
mod http;
mod local_timeslots;
mod schema;
#[cfg(test)]
mod testutils;
mod types;

#[tokio::main]
async fn main() {
    println!("###################");
    println!("# Booking Manager #");
    println!("###################");

    let configuration = ConfigurationHandler::parse_arguments();

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", configuration.port()))
        .await
        .unwrap();

    let app = if let Some(database_url) = configuration.database_url() {
        let backend = match DatabaseInterface::new(&database_url) {
            Err(err) => panic!("{err} Failed to establish database connection. Terminating the program. You may want to restart it with database disabled (impersistent timeslots)."),
            Ok(backend) => backend,
        };
        create_app(backend, configuration)
    } else {
        let backend = LocalTimeslots::default();
        create_app(backend, configuration)
    };

    axum::serve(listener, app).await.unwrap();
}
