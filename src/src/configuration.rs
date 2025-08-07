use std::path::PathBuf;

pub trait Configuration: Clone + Send + Sync + 'static {
    fn website_title(&self) -> String;
    fn password(&self) -> String;
    fn frontend_path(&self) -> PathBuf;
    fn port(&self) -> String;
    fn database_url(&self) -> Option<String>;
}
