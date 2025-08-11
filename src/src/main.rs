#[macro_use]
extern crate diesel;
use std::time::Duration;

use crate::{
    configuration::Configuration, configuration_handler::ConfigurationHandler,
    database_interface::DatabaseInterface, http::create_app, local_timeslots::LocalTimeslots,
};
use tokio::time::sleep;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

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
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    println!("###################");
    println!("# Booking Manager #");
    println!("###################");

    let configuration = ConfigurationHandler::parse_arguments();

    let address = format!("0.0.0.0:{}", configuration.port());
    println!("Accessable at:\n{}", address.clone());
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    let app = if let Some(database_url) = configuration.database_url() {
        let backend = loop {
            match DatabaseInterface::new(&database_url) {
                Ok(backend) => {
                    info!("Successfully connected to database");
                    break backend;
                }
                Err(err) => {
                    error!(?err, "Failed to establish database connection: {database_url}. Retry in 1 sec. You may want to restart it with database disabled (impersistent timeslots).");
                    sleep(Duration::from_secs(1)).await;
                }
            }
        };
        create_app(backend, configuration)
    } else {
        let backend = LocalTimeslots::default();
        create_app(backend, configuration)
    };

    axum::serve(listener, app).await.unwrap();
}
