use std::sync::{Arc, LazyLock, Weak};

use futures::{channel::mpsc, lock::Mutex, SinkExt, StreamExt};
use indexmap::IndexMap;

use crate::{
    bpz::Puzzle,
    editor::board::multiplayer::{MultiplayerBoardState, Op},
    realtime::proto::{ClientMessage, Result, ResultStream, ServerMessage},
};

static MAIN_ROOM: LazyLock<Arc<Mutex<Room>>> = LazyLock::new(|| Arc::new(Mutex::new(Room::new())));

macro_rules! bpz {
    ($key:literal, $path:literal) => {
        ($key, include_str!($path))
    };
}

static PUZZLES: LazyLock<Vec<(&'static str, Puzzle)>> = LazyLock::new(|| {
    [
        // bpz!("Basic 1", "../../puzzles/basic-1.bpz"),
        // bpz!("Basic 2", "../../puzzles/basic-2.bpz"),
        // bpz!("Basic 3", "../../puzzles/basic-3.bpz"),
        // bpz!("World Tour 1", "../../puzzles/world-tour-1.bpz"),
        // bpz!("World Tour 2", "../../puzzles/world-tour-2.bpz"),
        // bpz!("World Tour 3", "../../puzzles/world-tour-3.bpz"),
        bpz!("Drive-Thru 1", "../../puzzles/drive-thru-1.bpz"),
        bpz!("Drive-Thru 2", "../../puzzles/drive-thru-2.bpz"),
        bpz!("Drive-Thru 3", "../../puzzles/drive-thru-3.bpz"),
        // bpz!("Black Ice 1", "../../puzzles/black-ice-1.bpz"),
        // bpz!("Black Ice 2", "../../puzzles/black-ice-2.bpz"),
        // bpz!("Black Ice 3", "../../puzzles/black-ice-3.bpz"),
        // bpz!("Challenge 1 (Basic)", "../../puzzles/challenge-1.bpz"),
        // bpz!("Challenge 2 (World Tour)", "../../puzzles/challenge-2.bpz"),
        bpz!("Challenge 3 (Drive-Thru)", "../../puzzles/challenge-3.bpz"),
        // bpz!("Challenge 4 (Black Ice)", "../../puzzles/challenge-4.bpz"),
    ]
    .into_iter()
    .map(|(name, src)| {
        (
            name,
            src.parse().expect(&format!("Failed to parse {}", name)),
        )
    })
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
