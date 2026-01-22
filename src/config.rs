use crate::error::{Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const DEFAULT_CONFIG_FILE_NAME: &str = ".adi.yml";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {}

impl Config {
    pub fn new() -> Result<Self> {
        let config_path = path_with_home_dir(DEFAULT_CONFIG_FILE_NAME);
        let config_path = Path::new(&config_path);
        config::Config::builder()
            .add_source(config::File::from(config_path).required(false))
            .add_source(config::Environment::with_prefix("ADI"))
            .build()
            .wrap_err("Failed to build config")?
            .try_deserialize()
            .wrap_err("Failed to deserialize config")
    }
}

pub fn path_with_home_dir(path: &str) -> String {
    let home_dir = dirs::home_dir()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|| "/home/user".to_string());
    format!("{home_dir}/{path}")
}
