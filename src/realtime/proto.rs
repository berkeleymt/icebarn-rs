use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use server_fn::{BoxedStream, ServerFnError};

use crate::{
    bpz::Puzzle,
    editor::board::multiplayer::{MultiplayerBoardState, Op},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ClientMessage {
    Heartbeat,
    Op(String, Op),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ServerMessage {
    HeartbeatAck,
    Op(String, Op),
    State(String, MultiplayerBoardState),
    Init(IndexMap<String, (Puzzle, MultiplayerBoardState)>),
}

pub type Result<T, E = ServerFnError> = std::result::Result<T, E>;
pub type ResultStream<T, E = ServerFnError> = BoxedStream<T, E>;
