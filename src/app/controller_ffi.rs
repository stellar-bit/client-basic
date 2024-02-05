use super::*;
use dlopen::wrapper::{Container, WrapperApi};
use dlopen_derive::WrapperApi;

#[derive(WrapperApi)]
struct Computer {
    execute: extern "C" fn(
        pointers: *const (
            *mut Game,
            *const User,
            *const egui::Context,
            *mut Vec<GameCmd>,
        ),
    ),
}

pub struct Controller {
    computer: Container<Computer>,
    computers_dir: String,
    reload_file_interval: Interval,
    cur_computer_name: String,
}

impl Controller {
    pub fn new(computers_dir: &str) -> Self {
        let computer = unsafe {
            Container::<Computer>::load(Self::newest_computer_path(computers_dir)).unwrap()
        };

        Self {
            computer,
            reload_file_interval: Interval::new(std::time::Duration::from_secs(5)),
            computers_dir: computers_dir.into(),
            cur_computer_name: String::new(),
        }
    }
    pub fn retrieve_cmds(
        &mut self,
        game: &mut Game,
        user: &User,
        egui_context: &egui::Context,
    ) -> Vec<GameCmd> {
        if self.reload_file_interval.check() {
            unsafe {
                let newest_computer_name = Self::newest_computer_path(&self.computers_dir);
                if newest_computer_name != self.cur_computer_name {
                    self.cur_computer_name = newest_computer_name.clone();
                    self.computer = Container::<Computer>::load(newest_computer_name).unwrap()
                }
            };
        }
        let mut network_game_cmds = vec![];

        let game_ptr = game as *mut Game;
        let user_ptr = user as *const User;
        let egui_ctx_ptr = egui_context as *const egui::Context;
        let network_game_cmds_ptr = &mut network_game_cmds as *mut Vec<GameCmd>;
        let pointers: *const (
            *mut Game,
            *const User,
            *const egui::Context,
            *mut Vec<GameCmd>,
        ) = &(game_ptr, user_ptr, egui_ctx_ptr, network_game_cmds_ptr) as *const _;
        self.computer.execute(pointers);

        network_game_cmds
    }
    pub fn newest_computer_path(computers_dir: &str) -> String {
        let mut computer_paths: Vec<String> = std::fs::read_dir(computers_dir)
            .expect(&format!(
                "Make sure the computers directory exists: {}",
                computers_dir
            ))
            .into_iter()
            .map(|entry| entry.unwrap().path().to_str().unwrap().to_string())
            .collect();

        computer_paths.sort_by(|a, b| {
            if a.len() == b.len() {
                a.cmp(b)
            } else {
                a.len().cmp(&b.len())
            }
        });

        computer_paths.last().unwrap().to_string()
    }
}
