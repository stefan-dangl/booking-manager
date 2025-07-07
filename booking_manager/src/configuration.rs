use std::path::PathBuf;

pub trait Configuration: Clone + Send + Sync + 'static {
    fn password(&self) -> String;
    fn frontend_path(&self) -> PathBuf;
}
