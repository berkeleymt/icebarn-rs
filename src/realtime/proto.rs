use serde::{Deserialize, Serialize};
use server_fn::{BoxedStream, ServerFnError};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ClientMessage {
    Heartbeat,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ServerMessage {
    HeartbeatAck,
}

pub type Result<T, E = ServerFnError> = std::result::Result<T, E>;
pub type ResultStream<T, E = ServerFnError> = BoxedStream<T, E>;
