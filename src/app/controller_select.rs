use std::path::PathBuf;

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

struct ComputerContainer {
    pub computer: Container<Computer>,
    pub path: PathBuf,
}

impl ComputerContainer {
    pub fn load(path: PathBuf) -> Self {
        let computer = unsafe { Container::<Computer>::load(&path).unwrap() };
        Self { path, computer }
    }
}

pub struct Controller {
    computer_cont: Option<ComputerContainer>,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            computer_cont: None,
        }
    }
    pub fn retrieve_cmds(
        &mut self,
        game: &mut Game,
        user: &User,
        egui_context: &egui::Context,
    ) -> Vec<GameCmd> {
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
        if let Some(computer_cont) = &self.computer_cont {
            computer_cont.computer.execute(pointers);
        }

        network_game_cmds
    }
    pub fn select_computer(&mut self, computer_path: PathBuf) {
        self.computer_cont = Some(ComputerContainer::load(computer_path));
    }
    pub fn computer_path(&self) -> Option<PathBuf> {
        self.computer_cont.as_ref().map(|cc| cc.path.clone())
    }
}
