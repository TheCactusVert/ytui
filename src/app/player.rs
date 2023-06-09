use std::path::PathBuf;
use std::process::{Command, Stdio};

pub struct Player {
    bin: PathBuf,
}

impl Player {
    pub fn new() -> Self {
        Self {
            bin: "/usr/bin/celluloid".into(),
        }
    }

    pub fn play_video(&self, id: &str) {
        Command::new(&self.bin)
            .arg(format!("https://www.youtube.com/watch?v={id}"))
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .spawn();
    }
}
