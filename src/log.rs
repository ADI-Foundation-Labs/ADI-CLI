use chrono::Local;
use colored::Colorize;

#[derive(Debug, Clone)]
pub struct Logger {
    debug: bool,
}

impl Logger {
    pub fn new(debug: bool) -> Self {
        Self { debug }
    }

    pub fn error(&self, msg: &str) {
        eprintln!("{}", MessageBuilder::new().error(msg).build());
    }

    #[allow(dead_code)]
    pub fn error_fmt(&self, msg: &str) {
        eprintln!("{}", MessageBuilder::new().error_fmt(msg).build());
    }

    pub fn info(&self, msg: &str) {
        println!("{}", MessageBuilder::new().info(msg).build());
    }

    pub fn success_fmt(&self, msg: &str) {
        println!("{}", MessageBuilder::new().success_fmt(msg).build());
    }

    pub fn success(&self, msg: &str) {
        println!("{}", MessageBuilder::new().success(msg).build());
    }

    #[allow(dead_code)]
    pub fn warning(&self, msg: &str) {
        println!("{}", MessageBuilder::new().warning(msg).build());
    }

    #[allow(dead_code)]
    pub fn debug(&self, msg: &str) {
        if !self.debug {
            return;
        }
        println!("{}", MessageBuilder::new().debug(msg).build());
    }
}

pub struct MessageBuilder {
    msg: String,
}

impl MessageBuilder {
    pub fn new() -> Self {
        let timestamp = Local::now().format("[%Y-%m-%d %H:%M:%S]").to_string();
        Self {
            msg: format!(
                "{} {}",
                timestamp.bright_black(),
                "[adi-cli]: ".yellow().bold()
            ),
        }
    }

    pub fn error_fmt(self, msg: &str) -> Self {
        self.add(&format!("{}", "[Error]: ".red().bold())).add(msg)
    }

    pub fn error(self, msg: &str) -> Self {
        self.error_fmt(&format!("{}", msg.red()))
    }

    pub fn info(self, msg: &str) -> Self {
        self.add(&format!("{}", "[Info]: ".bright_cyan().bold()))
            .add(msg)
    }

    pub fn success_fmt(self, msg: &str) -> Self {
        self.add(&format!("{}", "[Success]: ".green().bold()))
            .add(msg)
    }

    pub fn success(self, msg: &str) -> Self {
        self.success_fmt(&format!("{}", msg.green()))
    }

    pub fn warning(self, msg: &str) -> Self {
        self.add(&format!("{}", "[Warning]: ".yellow().bold()))
            .add(msg)
    }

    pub fn debug(self, msg: &str) -> Self {
        self.add(&format!("{}", "[Debug]: ".blue().bold())).add(msg)
    }

    pub fn add(self, message: &str) -> Self {
        let mut msg = self.msg;
        msg.push_str(message);
        Self { msg }
    }

    pub fn build(&self) -> String {
        self.msg.clone()
    }
}
