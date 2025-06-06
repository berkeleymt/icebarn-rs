use std::sync::{Arc, Mutex};

use futures::{channel::mpsc, SinkExt, StreamExt};
use leptos::{prelude::*, server as leptos_server_fn};
use server_fn::{codec::MsgPackEncoding, Websocket};
use web_time::{Duration, SystemTime};

use crate::realtime::proto::{ClientMessage, Result, ResultStream, ServerMessage};

mod proto;
pub mod status;

#[cfg(feature = "ssr")]
mod server;

const HEARTBEAT_DURATION: Duration = Duration::from_secs(1);
const TIMEOUT_DURATION: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct ClientState {
    #[allow(dead_code)]
    last_heartbeat: RwSignal<SystemTime>,
    last_heartbeat_ack: RwSignal<SystemTime>,
    pub is_connected: Signal<bool>,
}

impl ClientState {
    fn new() -> Self {
        let last_heartbeat = RwSignal::new(SystemTime::now());
        let last_heartbeat_ack = RwSignal::new(SystemTime::UNIX_EPOCH);
        let is_connected = Signal::derive(move || {
            last_heartbeat.get() < last_heartbeat_ack.get() + TIMEOUT_DURATION
        });

        Self {
            last_heartbeat,
            last_heartbeat_ack,
            is_connected,
        }
    }

    fn recv(&self, message: ServerMessage) -> () {
        match message {
            ServerMessage::HeartbeatAck => {
                *self.last_heartbeat_ack.write() = SystemTime::now();
            }
        }
    }
}

pub struct Client {
    pub state: ClientState,
    pub tx: Mutex<mpsc::Sender<Result<ClientMessage>>>,
}

impl Client {
    fn new() -> (Self, mpsc::Receiver<Result<ClientMessage>>) {
        let (tx, rx) = mpsc::channel(1);
        let client = Self {
            state: ClientState::new(),
            tx: Mutex::new(tx),
        };
        (client, rx)
    }

    async fn listen(&self, mut input: ResultStream<ServerMessage>) {
        while let Some(message) = input.next().await {
            let result = match message {
                Ok(message) => self.recv(message).await,
                Err(error) => Err(error),
            };
            match result {
                Ok(()) => {}
                Err(error) => leptos::logging::error!("{:?}", error),
            }
        }
    }

    async fn send(&self, message: ClientMessage) -> Result<()> {
        Ok(self.tx.lock().unwrap().send(Ok(message)).await?)
    }

    async fn recv(&self, message: ServerMessage) -> Result<()> {
        self.state.recv(message);
        Ok(())
    }
}

pub fn provide_client() {
    let (client, rx) = Client::new();
    let client = Arc::new(client);

    if cfg!(feature = "hydrate") {
        // TODO: Remove these unwraps
        {
            let client = client.clone();
            leptos::task::spawn(async move {
                let input = connect_stub(rx.into()).await.unwrap();
                client.listen(input).await;
            });
        }
        {
            let client = client.clone();
            let heartbeat = move || {
                let client = client.clone();
                leptos::task::spawn_local(async move {
                    client.clone().send(ClientMessage::Heartbeat).await.unwrap()
                })
            };
            heartbeat();
            let result = set_interval_with_handle(heartbeat, HEARTBEAT_DURATION);
            result.unwrap();
        }
    }

    provide_context(client);
}

#[leptos_server_fn(protocol = Websocket<MsgPackEncoding, MsgPackEncoding>)]
async fn connect_stub(input: ResultStream<ClientMessage>) -> Result<ResultStream<ServerMessage>> {
    server::connect(input).await
}
