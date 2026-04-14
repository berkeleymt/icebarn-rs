use std::sync::Arc;

use crdts::CvRDT;
use futures::{channel::mpsc, lock::Mutex, SinkExt, StreamExt};
use indexmap::IndexMap;
use leptos::{prelude::*, server as leptos_server_fn};
use server_fn::{codec::MsgPackEncoding, Websocket};
use web_time::{Duration, SystemTime};

use crate::{
    bpz::Puzzle,
    editor::{board::multiplayer::MultiplayerBoard, State},
    realtime::proto::{ClientMessage, Result, ResultStream, ServerMessage},
};

mod proto;

#[cfg(feature = "ssr")]
mod server;

const HEARTBEAT_DURATION: Duration = Duration::from_secs(1);
const TIMEOUT_DURATION: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct HeartbeatState {
    last_heartbeat: RwSignal<SystemTime>,
    last_heartbeat_ack: RwSignal<SystemTime>,
    pub is_connected: Signal<bool>,
    pub fatal_error: RwSignal<Option<String>>,
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
            fatal_error: Default::default(),
        }
    }

    fn recv_ack(&self) {
        *self.last_heartbeat_ack.write() = SystemTime::now();
    }
}

pub struct Client {
    pub tx: Mutex<mpsc::Sender<Result<ClientMessage>>>,
    pub heartbeat_state: HeartbeatState,
    pub editor_state:
        RwSignal<Option<IndexMap<String, (Puzzle, RwSignal<State<MultiplayerBoard>>)>>>,
}

impl Client {
    fn new() -> (Arc<Self>, mpsc::Receiver<Result<ClientMessage>>) {
        let (tx, rx) = mpsc::channel(1);
        let client = Arc::new(Self {
            tx: Mutex::new(tx),
            heartbeat_state: HeartbeatState::new(),
            editor_state: RwSignal::new(None),
        });
        (client, rx)
    }

    pub async fn close(&self) {
        self.tx.lock().await.close_channel();
    }

    async fn listen(self: Arc<Self>, mut input: ResultStream<ServerMessage>) {
        while let Some(message) = input.next().await {
            let result = match message {
                Ok(message) => self.clone().recv(message).await,
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

    async fn recv(self: Arc<Self>, message: ServerMessage) -> Result<()> {
        match message {
            ServerMessage::FatalError(error) => {
                self.heartbeat_state.fatal_error.set(Some(error));
                self.tx.lock().await.close_channel();
            }
            ServerMessage::HeartbeatAck => {
                self.heartbeat_state.recv_ack();
            }
            ServerMessage::Op(key, op) => match &*self.editor_state.read_untracked() {
                Some(state) => {
                    if let Some((_, state)) = state.get(&key) {
                        state.write().board.state.apply_op(op)
                    }
                }
                None => {}
            },
            ServerMessage::State(key, other) => match &*self.editor_state.read_untracked() {
                Some(state) => {
                    if let Some((_, state)) = state.get(&key) {
                        state.write().board.state.0.merge(other.0)
                    }
                }
                None => {}
            },
            ServerMessage::JoinAck(state) => {
                let client = self.clone();
                self.editor_state.set(Some(
                    state
                        .into_iter()
                        .map(move |(key, (puzzle, state))| {
                            let puzzle_type = puzzle.puzzle_type;
                            (
                                key.clone(),
                                (puzzle, {
                                    let client = client.clone();
                                    let (tx, mut rx) = mpsc::channel(1000);
                                    let mut board = MultiplayerBoard::new(tx);
                                    board.state.0.merge(state.0);

                                    leptos::task::spawn(async move {
                                        while let Some(op) = rx.next().await {
                                            client
                                                .send(ClientMessage::Op(key.clone(), op))
                                                .await
                                                .unwrap()
                                        }
                                    });

                                    RwSignal::new(State::new(board, puzzle_type))
                                }),
                            )
                        })
                        .collect(),
                ));
            }
        };
        Ok(())
    }
}

pub fn connect_client(room: String) -> Arc<Client> {
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
            leptos::task::spawn_local(async move {
                client
                    .clone()
                    .send(ClientMessage::Join(room))
                    .await
                    .unwrap()
            });
        }
        {
            let client = client.clone();
            let heartbeat = move || {
                let client = client.clone();
                client.heartbeat_state.last_heartbeat.set(SystemTime::now());
                leptos::task::spawn_local(async move {
                    client.clone().send(ClientMessage::Heartbeat).await.unwrap()
                })
            };
            heartbeat();
            let result = set_interval_with_handle(heartbeat, HEARTBEAT_DURATION);
            result.unwrap();
        }
    }

    client
}

pub fn use_client() -> Option<Arc<Client>> {
    use_context::<Arc<Client>>()
}

#[leptos_server_fn(protocol = Websocket<MsgPackEncoding, MsgPackEncoding>)]
async fn connect_stub(input: ResultStream<ClientMessage>) -> Result<ResultStream<ServerMessage>> {
    server::connect(input).await
}
