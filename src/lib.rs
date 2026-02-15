#![feature(let_chains)]

use log::warn;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use ellipsoid::prelude::*;
use stellar_bit_core::prelude::{vec2, Vec2, *};
use std::sync::{Arc,RwLock,Mutex};
use std::path::PathBuf;
use app::controller_select::Controller;
use stellar_bit_central_hub_api::HubAPI;

mod app;
pub use app::{SpacecraftApp, Txts};

mod network;
use network::NetworkConnection;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    ellipsoid::run::<Txts, SpacecraftApp>();
}

pub fn run_headless_hub(
    server_id: i64, 
    username: String, 
    password: String, 
    computer_path: PathBuf
) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    // Connect to central hub first
    let hub_conn = match rt.block_on(HubAPI::connect(username.clone(), password)) {
        Ok(conn) => {
            println!("Successfully connected to central hub as '{}'!", username);
            conn
        }
        Err(err) => {
            eprintln!("Failed to connect to central hub: {:?}", err);
            return;
        }
    };

    // Get user data and server access
    let user_id = rt.block_on(hub_conn.my_user_data()).id;

    let server_access = rt.block_on(hub_conn.access_server(server_id));

    let mut init_game = Game::new();
    init_game.execute_cmd(User::Server, GameCmd::AddPlayer(0)).unwrap();
    
    let game: Arc<RwLock<Game>> = Arc::new(RwLock::new(init_game));
    let user: Arc<RwLock<User>> = Arc::new(RwLock::new(User::Player(0)));

    let mut controller = Controller::new();
    controller.select_computer(computer_path);

    // Add sync intervals
    let mut cmds_sync_interval = Interval::new(time::Duration::from_millis(300));
    let mut game_sync_interval = Interval::new(time::Duration::from_millis(3000));

    // Connect to server
    let network_connection_res = rt.block_on(NetworkConnection::start(
        format!("ws://{}", server_access.server_addr),
        game.clone(),
        user.clone(),
    ));

    match network_connection_res {
        Ok(mut network_connection) => {
            network_connection.sync_clock();
            network_connection.send(ClientRequest::Join(user_id as u64, server_access.access_token));
            network_connection.send(ClientRequest::FullGameSync);
            println!("Successfully connected to server!");

            loop {
                let mut game = game.write().unwrap();
                let user = User::Player(user_id as u64);
                
                // Update game state
                if game.sync.last_update >= now() {
                    warn!(
                        "Last game update is in the future [+{:?}]??!",
                        game.sync.last_update - now()
                    );
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
                
                let game_dt = now() - game.sync.last_update;
                game.update(game_dt.as_secs_f32());

                // Get commands from computer
                let network_game_cmds = controller.retrieve_cmds(
                    &mut game,
                    &user,
                    &egui::Context::default()
                );

                if !network_game_cmds.is_empty() {
                    network_connection.send(ClientRequest::ExecuteGameCmds(network_game_cmds));
                }

                // Send sync requests
                if cmds_sync_interval.check() {
                    network_connection.send(ClientRequest::GameCmdsSync);
                }
                if game_sync_interval.check() {
                    network_connection.send(ClientRequest::FullGameSync);
                }

                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        Err(e) => eprintln!("Error connecting to server: {:?}", e),
    }
}

pub fn run_headless_direct(
    server_addr: String,
    access_token: String,
    user_id: i64,
    computer_path: PathBuf
) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut init_game = Game::new();
    init_game.execute_cmd(User::Server, GameCmd::AddPlayer(0)).unwrap();
    
    let game: Arc<RwLock<Game>> = Arc::new(RwLock::new(init_game));
    let user: Arc<RwLock<User>> = Arc::new(RwLock::new(User::Player(0)));

    let mut controller = Controller::new();
    controller.select_computer(computer_path);

    // Add sync intervals
    let mut cmds_sync_interval = Interval::new(time::Duration::from_millis(300));
    let mut game_sync_interval = Interval::new(time::Duration::from_millis(3000));

    // Connect to server directly
    let network_connection_res = rt.block_on(NetworkConnection::start(
        server_addr,
        game.clone(),
        user.clone(),
    ));

    match network_connection_res {
        Ok(mut network_connection) => {
            network_connection.sync_clock();
            network_connection.send(ClientRequest::Join(user_id as u64, access_token));
            network_connection.send(ClientRequest::FullGameSync);
            println!("Successfully connected to server!");

            loop {
                let mut game = game.write().unwrap();
                let user = User::Player(user_id as u64);
                
                // Update game state
                if game.sync.last_update >= now() {
                    warn!(
                        "Last game update is in the future [+{:?}]??!",
                        game.sync.last_update - now()
                    );
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
                
                let game_dt = now() - game.sync.last_update;
                game.update(game_dt.as_secs_f32());

                // Get commands from computer
                let network_game_cmds = controller.retrieve_cmds(
                    &mut game,
                    &user,
                    &egui::Context::default()
                );

                if !network_game_cmds.is_empty() {
                    network_connection.send(ClientRequest::ExecuteGameCmds(network_game_cmds));
                }

                // Send sync requests
                if cmds_sync_interval.check() {
                    network_connection.send(ClientRequest::GameCmdsSync);
                }
                if game_sync_interval.check() {
                    network_connection.send(ClientRequest::FullGameSync);
                }

                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        Err(e) => eprintln!("Error connecting to server: {:?}", e),
    }
}
