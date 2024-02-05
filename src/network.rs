use std::sync::mpsc;

use log::error;

use super::*;

#[cfg(target_arch = "wasm32")]
mod web_network_client;
#[cfg(target_arch = "wasm32")]
type NetworkClient = web_network_client::WebNetworkClient;

#[cfg(not(target_arch = "wasm32"))]
mod desktop_network_client;
#[cfg(not(target_arch = "wasm32"))]
type NetworkClient = desktop_network_client::DesktopNetworkClient;

pub struct NetworkConnection {
    client: NetworkClient,
    pub server_addr: String,
}

impl NetworkConnection {
    pub async fn start(
        server_addr: String,
        game: Arc<RwLock<Game>>,
        user: Arc<RwLock<User>>,
    ) -> Result<Self, NetworkError> {
        let client = NetworkClient::connect(&server_addr, game.clone(), user).await?;
        Ok(Self {
            client,
            server_addr,
        })
    }
    pub fn send_multiple(&mut self, msgs: Vec<ClientRequest>) -> Result<(), NetworkError> {
        self.client.send_multiple(msgs)
    }
    pub async fn send(&mut self, msg: ClientRequest) -> Result<(), NetworkError> {
        self.client.send(msg)
    }
}

fn handle_server_response(
    response: ServerResponse,
    game: Arc<RwLock<Game>>,
    time_delay: Arc<RwLock<i64>>,
    user: Arc<RwLock<User>>,
    sync_response_sender: &mpsc::Sender<ServerResponse>,
) {
    match response {
        ServerResponse::SyncFullGame(new_game) => {
            let mut game = game.write().unwrap();
            *game = new_game;
            let last_update =
                (game.sync.last_update.as_millis() as i64 - *time_delay.read().unwrap()) as u64;
            let last_update = time::Duration::from_millis(last_update);
            game.sync.last_update = last_update;
        }
        ServerResponse::SyncGameCmds(cmds) => {
            let mut game = game.write().unwrap();
            let user = *user.read().unwrap();
            cmds.into_iter().for_each(|(cmd_user, cmd)| {
                if let (User::Player(player_id), User::Player(cmd_player_id)) = (user, cmd_user) {
                    if cmd_player_id == player_id {
                        return;
                    }
                }
                game.execute_cmd(cmd_user, cmd).unwrap();
            });
        }
        ServerResponse::SetUser(new_user) => {
            *user.write().unwrap() = new_user;
        }
        ServerResponse::SlowDown => {
            error!("Messages are getting ignored (sending too fast)!");
        }
        ServerResponse::SyncClock(remote_clock) => {
            sync_response_sender.send(ServerResponse::SyncClock(remote_clock)).unwrap();
        }
        _ => (),
    }
}
