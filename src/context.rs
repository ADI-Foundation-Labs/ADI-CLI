use crate::error::{Result, WrapErr};

use crate::{config::Config, log::Logger};

#[derive(Clone)]
pub struct Context {
    #[allow(dead_code)] // Will be used when commands need config access
    cfg: Config,
    log: Logger,
}

impl Context {
    pub fn new_from_options(_options: &super::Opts) -> Result<Self> {
        let cfg = Config::new().wrap_err("Failed to load config")?;

        Ok(Self {
            cfg,
            log: Logger::new(false), // TODO: Debug?
        })
    }

    #[allow(dead_code)] // Will be used when commands need config access
    pub fn config(&self) -> &Config {
        &self.cfg
    }

    #[allow(dead_code)]
    pub fn error(&self, msg: &str) {
        self.log.error(msg);
    }

    #[allow(dead_code)]
    pub fn error_fmt(&self, msg: &str) {
        self.log.error_fmt(msg);
    }

    pub fn info(&self, msg: &str) {
        self.log.info(msg);
    }

    #[allow(dead_code)] // Will be used by commands for success messages
    pub fn success(&self, msg: &str) {
        self.log.success(msg);
    }

    #[allow(dead_code)] // Will be used by commands for formatted success messages
    pub fn success_fmt(&self, msg: &str) {
        self.log.success_fmt(msg);
    }

    #[allow(dead_code)]
    pub fn warning(&self, msg: &str) {
        self.log.warning(msg);
    }

    #[allow(dead_code)]
    pub fn debug(&self, msg: &str) {
        self.log.debug(msg);
    }
}
