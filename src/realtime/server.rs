use std::{
    collections::HashMap,
    sync::{Arc, OnceLock, Weak},
};

use futures::{channel::mpsc, lock::Mutex, SinkExt, StreamExt};
use indexmap::IndexMap;
use leptos::prelude::use_context;
use sqlx::PgPool;

use crate::{
    auth::TeamSession,
    bpz::Puzzle,
    editor::board::multiplayer::{MultiplayerBoardState, Op},
    puzzles::PUZZLES,
    realtime::proto::{ClientMessage, Result, ResultStream, ServerMessage},
};

static ROOM_MANAGER: OnceLock<RoomManager> = OnceLock::new();

#[derive(Debug)]
struct RoomManager {
    inner: Mutex<HashMap<String, Arc<Mutex<Room>>>>,
    pool: sqlx::PgPool,
}

impl RoomManager {
    /// Fetch a team's room by key, creating a fresh one (seeded with the
    /// current puzzle set) if it doesn't exist yet. With ContestDojo sign-in,
    /// the key is the team id, so every authenticated team transparently gets
    /// its own room on first join.
    async fn get_or_create(&self, pwd: &str) -> Option<Arc<Mutex<Room>>> {
        if let Some(room) = self.inner.lock().await.get(pwd) {
            return Some(room.clone());
        }

        let stored: Option<Vec<u8>> =
            match sqlx::query_as("SELECT state FROM rooms WHERE pwd = $1 ORDER BY ts DESC LIMIT 1")
                .bind(&pwd)
                .fetch_one(&self.pool)
                .await
            {
                Ok((state,)) => state,
                Err(sqlx::Error::RowNotFound) => None,
                Err(err) => {
                    leptos::logging::warn!("error fetching from database: {}", err);
                    return None;
                }
            };

        let room = match stored {
            None => Room::new(pwd.to_owned(), self.pool.clone()),
            Some(state) => Room::from_state(
                pwd.to_owned(),
                rmp_serde::from_slice(&state[..])
                    .map_err(|err| {
                        leptos::logging::warn!("error deserializing from database: {}", err)
                    })
                    .ok()?,
                self.pool.clone(),
            ),
        };

        let room = Arc::new(Mutex::new(room));

        {
            let room = room.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    let mut thing = room.lock().await;
                    if thing.saved {
                        continue;
                    }
                    let value = match rmp_serde::to_vec_named(&thing.puzzles) {
                        Ok(value) => value,
                        Err(err) => {
                            leptos::logging::warn!("error serializing to database: {}", err);
                            continue;
                        }
                    };
                    match sqlx::query("INSERT INTO rooms (pwd, state) VALUES ($1, $2)")
                        .bind(&thing.pwd)
                        .bind(&value)
                        .execute(&thing.pool)
                        .await
                    {
                        Ok(_) => (),
                        Err(err) => leptos::logging::warn!("error writing to database: {}", err),
                    };
                    thing.saved = true;
                }
            });
        }

        self.inner.lock().await.insert(pwd.to_owned(), room.clone());
        Some(room)
    }
}

struct Room {
    pwd: String,
    puzzles: IndexMap<String, (Puzzle, MultiplayerBoardState)>,
    clients: Vec<Weak<ClientHandle>>,
    saved: bool,
    pool: PgPool,
}

impl Room {
    fn new(pwd: String, pool: PgPool) -> Self {
        Self::from_state(
            pwd,
            PUZZLES
                .iter()
                .map(|(key, puzzle)| {
                    (
                        key.to_string(),
                        (puzzle.clone(), MultiplayerBoardState::default()),
                    )
                })
                .collect(),
            pool,
        )
    }

    fn from_state(
        pwd: String,
        state: IndexMap<String, (Puzzle, MultiplayerBoardState)>,
        pool: PgPool,
    ) -> Self {
        Self {
            pwd,
            puzzles: state,
            clients: Vec::new(),
            pool,
            saved: true,
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
        self.saved = false;
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
    /// The authenticated team for this connection, resolved from the session
    /// cookie at connect time. `None` means the client is not signed in.
    team: Option<TeamSession>,
}

impl ClientHandle {
    fn new(team: Option<TeamSession>) -> (Self, mpsc::Receiver<Result<ServerMessage>>) {
        let (tx, rx) = mpsc::channel(1);
        let client = Self {
            tx: Mutex::new(tx),
            room: Default::default(),
            team,
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
            ClientMessage::Join(_) => {
                // The room is keyed by the authenticated team id from the
                // session cookie, never by a value the client supplies, so a
                // user can only ever join their own team's room.
                let Some(team) = self.team.clone() else {
                    self.fatal_error(
                        "You must sign in with ContestDojo to join your team.".to_owned(),
                    )
                    .await?;
                    return Ok(());
                };

                let mut self_room = self.room.lock().await;
                if let None = &*self_room {
                    if let Some(room) = ROOM_MANAGER.get().unwrap().get_or_create(&team.team_id).await
                    {
                        room.lock().await.add_client(self.clone()).await?;
                        *self_room = Some(room.clone());
                    } else {
                        self.fatal_error(
                            "Could not open your team's room. Please try again.".to_owned(),
                        )
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
    let pool = use_context::<PgPool>().unwrap();
    ROOM_MANAGER.get_or_init(|| RoomManager {
        inner: Default::default(),
        pool,
    });

    // Resolve the team from the session cookie on the upgrade request so the
    // join is authorized server-side.
    let team = crate::auth::server::team_from_request().await;

    let (client, rx) = ClientHandle::new(team);
    let client = Arc::new(client);
    tokio::spawn(client.listen(input));
    Ok(rx.into())
}
