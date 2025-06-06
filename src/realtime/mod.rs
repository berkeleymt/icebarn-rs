pub mod status;

use std::sync::{Arc, Mutex};

use futures::{channel::mpsc, FutureExt, SinkExt};
use leptos::wasm_bindgen::JsValue;
use leptos::{prelude::*, task::spawn_local};
use serde::{Deserialize, Serialize};
use server_fn::{codec::MsgPackEncoding, BoxedStream, ServerFnError, Websocket};
use thiserror::Error;
use web_time::{Duration, SystemTime};

// TODO: Increase these numbers
const HEARTBEAT_DURATION: Duration = Duration::from_secs(1);
const TIMEOUT_DURATION: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ClientMessage {
    Heartbeat,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ServerMessage {
    HeartbeatAck,
}

#[derive(Debug)]
pub struct RealtimeClient {
    last_heartbeat: RwSignal<SystemTime>,
    last_heartbeat_ack: RwSignal<SystemTime>,
    pub is_connected: Signal<bool>,

    pub acks: RwSignal<u32>,
    pub errors: ReadSignal<Vec<RealtimeError>>,
    set_errors: WriteSignal<Vec<RealtimeError>>,

    tx: Mutex<mpsc::Sender<Result<ClientMessage, ServerFnError>>>,
}

#[derive(Debug, Clone, Error)]
pub enum RealtimeError {
    #[error("error communicating with server")]
    ServerFnError(String),

    #[error("error sending to websocket")]
    SendError(#[from] mpsc::SendError),

    #[error("TODO better error handling")]
    Todo,
}

impl From<ServerFnError> for RealtimeError {
    fn from(value: ServerFnError) -> Self {
        Self::ServerFnError(value.to_string())
    }
}

impl From<JsValue> for RealtimeError {
    fn from(_value: JsValue) -> Self {
        Self::Todo
    }
}

impl RealtimeClient {
    fn new(tx: mpsc::Sender<Result<ClientMessage, ServerFnError>>) -> Self {
        let last_heartbeat = RwSignal::new(SystemTime::now());
        let last_heartbeat_ack = RwSignal::new(SystemTime::UNIX_EPOCH);
        let is_connected = Signal::derive(move || {
            last_heartbeat.get() < last_heartbeat_ack.get() + TIMEOUT_DURATION
        });

        let (errors, set_errors) = signal(vec![]);

        Self {
            last_heartbeat,
            last_heartbeat_ack,
            is_connected,
            acks: RwSignal::default(),
            errors,
            set_errors,
            tx: Mutex::new(tx),
        }
    }

    fn collect_error<T, E: Into<RealtimeError>>(&self, result: Result<T, E>) -> Option<T> {
        result
            .map_err(|err| self.set_errors.write().push(err.into()))
            .ok()
    }

    fn collect_error_unit(&self, result: Result<(), RealtimeError>) -> () {
        let _ = self.collect_error(result);
    }

    fn spawn_heartbeat(self: Arc<Self>) {
        self.clone().spawn_send(ClientMessage::Heartbeat);
        self.last_heartbeat.set(SystemTime::now())
    }

    fn start_heartbeating(self: Arc<Self>) {
        self.clone().spawn_heartbeat();

        let result = self.clone().collect_error(set_interval_with_handle(
            move || self.clone().spawn_heartbeat(),
            HEARTBEAT_DURATION,
        ));

        if let Some(handle) = result {
            on_cleanup(move || handle.clear());
        };
    }

    fn on_message(self: Arc<Self>, message: ServerMessage) {
        match message {
            ServerMessage::HeartbeatAck => {
                *self.last_heartbeat_ack.write() = SystemTime::now();
            }
        }
    }

    pub async fn send(self: Arc<Self>, message: ClientMessage) -> Result<(), RealtimeError> {
        let mut tx = self.tx.lock().unwrap();
        Ok(tx.send(Ok(message)).await?)
    }

    pub fn spawn_send(self: Arc<Self>, message: ClientMessage) {
        spawn_local(
            self.clone()
                .send(message)
                .map(move |r| self.collect_error_unit(r)),
        )
    }
}

#[server(protocol = Websocket<MsgPackEncoding, MsgPackEncoding>)]
async fn realtime_websocket(
    input: BoxedStream<ClientMessage, ServerFnError>,
) -> Result<BoxedStream<ServerMessage, ServerFnError>, ServerFnError> {
    use futures::{SinkExt, StreamExt};

    let mut input = input;
    let (mut tx, rx) = mpsc::channel(1);

    tokio::spawn(async move {
        while let Some(msg) = input.next().await {
            match msg {
                Ok(ClientMessage::Heartbeat) => {
                    // TODO: Error handling
                    let _ = tx.send(Ok(ServerMessage::HeartbeatAck)).await;
                }
                _ => {}
            };
        }
    });

    Ok(rx.into())
}

pub fn provide_realtime_client() {
    use futures::channel::mpsc;
    use futures::StreamExt;
    let (tx, rx) = mpsc::channel(1);
    let client = Arc::new(RealtimeClient::new(tx));

    if cfg!(feature = "hydrate") {
        let client = client.clone();
        client.clone().start_heartbeating();

        spawn_local(async move {
            match realtime_websocket(rx.into()).await {
                Ok(mut messages) => {
                    while let Some(r) = messages.next().await {
                        let client = client.clone();
                        if let Some(msg) = client.collect_error(r) {
                            client.on_message(msg)
                        }
                    }
                }
                Err(e) => leptos::logging::warn!("{e}"),
            }
        });
    }

    provide_context(client);
}
