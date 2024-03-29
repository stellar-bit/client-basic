use std::{time::Duration};

use futures_util::{sink::SinkExt, stream::SplitSink, StreamExt};
use tokio::{net::TcpStream, task::yield_now, sync::mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use super::*;

pub struct DesktopNetworkClient {
    receive_task: tokio::task::JoinHandle<()>,
    send_task: tokio::task::JoinHandle<()>,
    time_delay: Arc<RwLock<i64>>,
    sync_response_receiver: std::sync::mpsc::Receiver<ServerResponse>,
    send_tx: mpsc::Sender<Vec<ClientRequest>>
}

impl DesktopNetworkClient {
    pub async fn connect(
        server_addr: &str,
        game: Arc<RwLock<Game>>,
        user: Arc<RwLock<User>>,
    ) -> Result<Self, NetworkError> {
        let ws_stream = connect_async(server_addr)
            .await
            .map_err(|_e| NetworkError::WebsocketTrouble)?
            .0;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let (sync_response_sender, sync_response_receiver) = std::sync::mpsc::channel();

        let time_delay = Arc::new(RwLock::new(0));

        let (send_tx, mut rx) = mpsc::channel::<Vec<ClientRequest>>(5);

        let receive_task = {
            let game = game.clone();
            let time_delay = time_delay.clone();
            let user = user.clone();
            tokio::spawn(async move {
                while let Some(msg) = ws_receiver.next().await {
                    let msg = msg.unwrap();
                    let response: ServerResponse = deserialize_bytes(&msg.into_data()).unwrap();
                    handle_server_response(
                        response,
                        game.clone(),
                        time_delay.clone(),
                        user.clone(),
                        &sync_response_sender,
                    );
                }
                // loop {
                //     tokio::time::sleep(Duration::from_millis(50)).await;
                // }
            })
        };

        let send_task = {
            tokio::spawn(async move {
                while let Some(msgs) = rx.recv().await {
                    for msg in &msgs {
                        ws_sender
                            .feed(Message::Binary(serialize_bytes(&msg).unwrap()))
                            .await
                            .unwrap()
                    }
                    ws_sender
                        .flush()
                        .await
                        .unwrap();
                }
            })
        };



        Ok(Self {
            receive_task,
            sync_response_receiver,
            send_tx,
            time_delay,
            send_task
        })
    }
    pub fn sync_clock(&mut self) {
        let mut time_delays = vec![0; 15];
        for time_delay in &mut time_delays {
            let start = std::time::Instant::now();
            self.send(ClientRequest::SyncClock);
            let ServerResponse::SyncClock(mut remote_clock) =
                self.sync_response_receiver.recv().unwrap()
            else {
                panic!("The only time we should get message from sync_response_receiver is when he sends SyncClock resp.");
            };
            remote_clock += start.elapsed() / 2;
            *time_delay = remote_clock.as_millis() as i64 - now().as_millis() as i64;
            std::thread::sleep(time::Duration::from_millis(100));
        }

        println!("{:?}", time_delays);

        time_delays.sort();

        let median = time_delays[time_delays.len() / 2];
        let average = time_delays.iter().sum::<i64>() / time_delays.len() as i64;

        let mut sum_of_squares = 0;
        for time_delay in &time_delays {
            sum_of_squares += (time_delay - average) * (time_delay - average);
        }

        let standard_deviation = ((sum_of_squares / time_delays.len() as i64) as f64).sqrt() as i64;
        let _ = time_delays
            .extract_if(|time_delay| (*time_delay - median).abs() > standard_deviation)
            .collect::<Vec<_>>();

        *self.time_delay.write().unwrap() =
            time_delays.iter().sum::<i64>() / time_delays.len() as i64;
        println!(
            "Time delay between client and server: {} ms",
            self.time_delay.read().unwrap()
        );
    }
    pub fn send(&mut self, msg: ClientRequest) {
        self.send_multiple(vec![msg]);
    }
    pub fn send_multiple(&mut self, msgs: Vec<ClientRequest>) {
        self.send_tx.blocking_send(msgs).unwrap();
    }
}

impl Drop for DesktopNetworkClient {
    fn drop(&mut self) {
        self.receive_task.abort();
    }
}
