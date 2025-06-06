use serde::{Deserialize, Serialize};
use server_fn::{BoxedStream, ServerFnError};

use crate::editor::board::multiplayer::Op;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ClientMessage {
    Heartbeat,
    Op(Op),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ServerMessage {
    HeartbeatAck,
    Op(Op),
}

pub type Result<T, E = ServerFnError> = std::result::Result<T, E>;
pub type ResultStream<T, E = ServerFnError> = BoxedStream<T, E>;
