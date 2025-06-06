use std::sync::{Arc, LazyLock, Weak};

use futures::{channel::mpsc, lock::Mutex, SinkExt, StreamExt};

use crate::realtime::proto::{ClientMessage, Result, ResultStream, ServerMessage};

static MAIN_ROOM: LazyLock<Arc<Mutex<Room>>> = LazyLock::new(|| Arc::default());

#[derive(Default)]
struct Room {
    clients: Vec<Weak<ClientHandle>>,
}

impl Room {
    fn add_client(&mut self, client: Weak<ClientHandle>) {
        self.clients.push(client);
    }

    async fn broadcast(&mut self, message: ServerMessage) -> Result<()> {
        for client in &self.clients {
            if let Some(client) = client.upgrade() {
                // TODO: Error handling - what if this fails?
                client.send(message.clone()).await?
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
            ClientMessage::Op(op) => {
                self.room
                    .lock()
                    .await
                    .broadcast(ServerMessage::Op(op))
                    .await?;
            }
        };
        Ok(())
    }
}

pub async fn connect(input: ResultStream<ClientMessage>) -> Result<ResultStream<ServerMessage>> {
    let room = MAIN_ROOM.clone();

    let (client, rx) = ClientHandle::new(room.clone());
    let client = Arc::new(client);
    tokio::spawn(client.clone().listen(input));
    room.lock().await.add_client(Arc::downgrade(&client));

    Ok(rx.into())
}
