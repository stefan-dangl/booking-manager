use std::path::PathBuf;

pub trait Configuration: Clone + Send + Sync + 'static {
    fn website_title(&self) -> String;
    fn password(&self) -> String;
    fn frontend_path(&self) -> PathBuf;
    fn database_url(&self) -> Option<String>;
    fn port(&self) -> String;
}
