use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Weak},
};

use futures::{channel::mpsc, lock::Mutex, SinkExt, StreamExt};
use indexmap::IndexMap;

use crate::{
    bpz::Puzzle,
    editor::board::multiplayer::{MultiplayerBoardState, Op},
    puzzles::PUZZLES,
    realtime::proto::{ClientMessage, Result, ResultStream, ServerMessage},
};

static ROOMS: LazyLock<HashMap<String, Arc<Mutex<Room>>>> = LazyLock::new(|| {
    let mut rooms = HashMap::new();
    rooms.insert("bmtream".to_owned(), Arc::new(Mutex::new(Room::new())));
    rooms
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
            .send(ServerMessage::JoinAck(self.puzzles.clone()))
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
    room: Arc<Mutex<Option<Arc<Mutex<Room>>>>>,
}

impl ClientHandle {
    fn new() -> (Self, mpsc::Receiver<Result<ServerMessage>>) {
        let (tx, rx) = mpsc::channel(1);
        let client = Self {
            tx: Mutex::new(tx),
            room: Default::default(),
        };
        (client, rx)
    }

    async fn listen(self: Arc<Self>, mut input: ResultStream<ClientMessage>) {
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

    async fn send(&self, message: ServerMessage) -> Result<()> {
        Ok(self.tx.lock().await.send(Ok(message)).await?)
    }

    async fn fatal_error(&self, message: String) -> Result<()> {
        self.send(ServerMessage::FatalError(message)).await?;
        self.tx.lock().await.close_channel();
        Ok(())
    }

    async fn recv(self: Arc<Self>, message: ClientMessage) -> Result<()> {
        match message {
            ClientMessage::Join(room) => {
                let mut self_room = self.room.lock().await;
                if let None = &*self_room {
                    if let Some(room) = ROOMS.get(&room) {
                        room.lock().await.add_client(self.clone()).await?;
                        *self_room = Some(room.clone());
                    } else {
                        self.fatal_error("No room exists with the given password.".to_owned())
                            .await?;
                    }
                } else {
                    self.fatal_error("Attempted to join a room twice!".to_owned())
                        .await?;
                }
            }
            ClientMessage::Heartbeat => {
                self.send(ServerMessage::HeartbeatAck).await?;
            }
            ClientMessage::Op(key, op) => {
                if let Some(room) = &*self.room.lock().await {
                    room.lock().await.recv_op(key, op).await?;
                }
            }
        };
        Ok(())
    }
}

pub async fn connect(input: ResultStream<ClientMessage>) -> Result<ResultStream<ServerMessage>> {
    let (client, rx) = ClientHandle::new();
    let client = Arc::new(client);
    tokio::spawn(client.listen(input));
    Ok(rx.into())
}
