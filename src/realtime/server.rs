use std::sync::{Arc, LazyLock, Weak};

use futures::{channel::mpsc, lock::Mutex, SinkExt, StreamExt};
use indexmap::IndexMap;

use crate::{
    bpz::Puzzle,
    editor::board::multiplayer::{MultiplayerBoardState, Op},
    realtime::proto::{ClientMessage, Result, ResultStream, ServerMessage},
};

static MAIN_ROOM: LazyLock<Arc<Mutex<Room>>> = LazyLock::new(|| Arc::new(Mutex::new(Room::new())));

static PUZZLES: LazyLock<Vec<(&'static str, Puzzle)>> = LazyLock::new(|| {
    [
        ("Basic 1", include_str!("../../puzzles/basic-1.bpz")),
        ("Basic 2", include_str!("../../puzzles/basic-2.bpz")),
        ("Basic 3", include_str!("../../puzzles/basic-3.bpz")),
    ]
    .into_iter()
    .map(|(name, src)| (name, src.parse().unwrap()))
    .collect()
});

struct Room {
    puzzles: IndexMap<String, (Puzzle, MultiplayerBoardState)>,
    clients: Vec<Weak<ClientHandle>>,
}

impl Room {
    fn new() -> Self {
        Self {
            puzzles: PUZZLES
                .iter()
                .map(|(key, puzzle)| {
                    (
                        key.to_string(),
                        (puzzle.clone(), MultiplayerBoardState::default()),
                    )
                })
                .collect(),
            clients: Vec::new(),
        }
    }

    async fn add_client(&mut self, client: Arc<ClientHandle>) -> Result<()> {
        client
            .send(ServerMessage::Init(self.puzzles.clone()))
            .await?;
        self.clients.push(Arc::downgrade(&client));
        Ok(())
    }

    async fn recv_op(&mut self, key: String, op: Op) -> Result<()> {
        if let Some((_, state)) = self.puzzles.get_mut(&key) {
            state.apply_op(op.clone())
        };
        for client in &self.clients {
            if let Some(client) = client.upgrade() {
                // TODO: Error handling - what if this fails?
                client
                    .send(ServerMessage::Op(key.clone(), op.clone()))
                    .await?
            }
        }
        Ok(())
    }
}

struct ClientHandle {
    tx: Mutex<mpsc::Sender<Result<ServerMessage>>>,
    room: Arc<Mutex<Room>>,
}

impl ClientHandle {
    fn new(room: Arc<Mutex<Room>>) -> (Self, mpsc::Receiver<Result<ServerMessage>>) {
        let (tx, rx) = mpsc::channel(1);
        let client = Self {
            tx: Mutex::new(tx),
            room,
        };
        (client, rx)
    }

    async fn listen(self: Arc<Self>, mut input: ResultStream<ClientMessage>) {
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

    async fn send(&self, message: ServerMessage) -> Result<()> {
        Ok(self.tx.lock().await.send(Ok(message)).await?)
    }

    async fn recv(&self, message: ClientMessage) -> Result<()> {
        match message {
            ClientMessage::Heartbeat => {
                self.send(ServerMessage::HeartbeatAck).await?;
            }
            ClientMessage::Op(key, op) => {
                self.room.lock().await.recv_op(key, op).await?;
            }
        };
        Ok(())
    }
}

pub async fn connect(input: ResultStream<ClientMessage>) -> Result<ResultStream<ServerMessage>> {
    let room = MAIN_ROOM.clone();

    let (client, rx) = ClientHandle::new(room.clone());
    let client = Arc::new(client);
    room.lock().await.add_client(client.clone()).await?;
    tokio::spawn(client.listen(input));

    Ok(rx.into())
}
