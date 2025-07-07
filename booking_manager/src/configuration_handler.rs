use crate::configuration::Configuration;
use std::path::PathBuf;

#[derive(Clone)]
pub struct ConfigurationHandler;

impl Configuration for ConfigurationHandler {
    fn password(&self) -> String {
        "123".into()
    }

    fn frontend_path(&self) -> PathBuf {
        PathBuf::from("../frontend/index.html")
    }
}
