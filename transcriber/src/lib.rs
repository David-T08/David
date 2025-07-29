use std::process::Command;
use ureq;

pub enum WhisperModel {

}

pub struct TranscriberConfig {
    port: u16,
    host: Option<String>,
}

impl TranscriberConfig {
    pub fn from_file() {
        todo!();
    }
}

pub struct Transcriber {
    port: u16,
    host: Option<String>,
}

impl Transcriber {
    pub fn connect(port: u16, host: Option<String>) -> Result<TranscriberServer, String> {
        todo!();
    }
}

pub struct TranscriberServer {}

impl TranscriberServer {}
