use std::sync::Arc;

use futures::{channel::mpsc, lock::Mutex, SinkExt, StreamExt};
use leptos::{prelude::*, server as leptos_server_fn};
use server_fn::{codec::MsgPackEncoding, Websocket};
use web_time::{Duration, SystemTime};

use crate::{
    editor::{board::multiplayer::MultiplayerBoard, State},
    realtime::proto::{ClientMessage, Result, ResultStream, ServerMessage},
};

mod proto;
pub mod status;

#[cfg(feature = "ssr")]
mod server;

const HEARTBEAT_DURATION: Duration = Duration::from_secs(1);
const TIMEOUT_DURATION: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct HeartbeatState {
    #[allow(dead_code)]
    last_heartbeat: RwSignal<SystemTime>,
    last_heartbeat_ack: RwSignal<SystemTime>,
    pub is_connected: Signal<bool>,
}

impl HeartbeatState {
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

    fn recv_ack(&self) {
        *self.last_heartbeat_ack.write() = SystemTime::now();
    }
}

pub struct Client {
    pub tx: Mutex<mpsc::Sender<Result<ClientMessage>>>,
    pub heartbeat_state: HeartbeatState,
    pub editor_state: RwSignal<Option<RwSignal<State<MultiplayerBoard>>>>,
}

impl Client {
    fn new() -> (Arc<Self>, mpsc::Receiver<Result<ClientMessage>>) {
        let (tx, rx) = mpsc::channel(1);
        let client = Arc::new(Self {
            tx: Mutex::new(tx),
            heartbeat_state: HeartbeatState::new(),
            editor_state: RwSignal::new(None),
        });

        {
            let client = client.clone();
            let (tx, rx) = mpsc::channel(1000);
            let board = MultiplayerBoard::new(tx);

            leptos::task::spawn({
                let client = client.clone();
                let mut rx = rx;
                async move {
                    while let Some(op) = rx.next().await {
                        client.send(ClientMessage::Op(op)).await.unwrap()
                    }
                }
            });

            client
                .clone()
                .editor_state
                .set(Some(RwSignal::new(State::new(board))));
        }

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
        Ok(self.tx.lock().await.send(Ok(message)).await?)
    }

    async fn recv(&self, message: ServerMessage) -> Result<()> {
        match message {
            ServerMessage::HeartbeatAck => {
                self.heartbeat_state.recv_ack();
            }
            // TODO: Fix non-reactive warning
            ServerMessage::Op(op) => match &*self.editor_state.read() {
                Some(state) => state.write().board.apply_op(op),
                None => {}
            },
        };
        Ok(())
    }
}

pub fn provide_client() {
    let (client, rx) = Client::new();

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

    provide_context::<Arc<Client>>(client);
}

pub fn use_client() -> Option<Arc<Client>> {
    use_context::<Arc<Client>>()
}

#[leptos_server_fn(protocol = Websocket<MsgPackEncoding, MsgPackEncoding>)]
async fn connect_stub(input: ResultStream<ClientMessage>) -> Result<ResultStream<ServerMessage>> {
    server::connect(input).await
}
