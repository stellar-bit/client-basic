use super::*;

use std::io::prelude::*;
use std::process::{Command, Stdio};

pub struct Controller {
    pub computer_path: String,
}

impl Controller {
    /// This function took approximately 2 ms
    pub fn retrieve_cmds(&self, game_data: &GameData) -> Vec<GameCmd> {
        let mut child = Command::new(&self.computer_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to query computer.");

        let mut stdin = child.stdin.take().unwrap();

        let game_data_raw = serialize_bytes(&game_data).unwrap();
        stdin.write_all(&game_data_raw).unwrap();

        // signal eof to stdin
        drop(stdin);

        let output = child.wait_with_output().unwrap();

        let (cmds, logs) =
            deserialize_bytes::<(Vec<GameCmd>, Vec<String>)>(&output.stdout).unwrap();

        // for log in logs {
        //     println!("{}", log);
        // }

        cmds
    }
}
