use futures::{channel::mpsc, SinkExt, StreamExt};

use crate::realtime::proto::{ClientMessage, Result, ResultStream, ServerMessage};

struct ClientHandle {
    tx: mpsc::Sender<Result<ServerMessage>>,
}

impl ClientHandle {
    fn new() -> (Self, mpsc::Receiver<Result<ServerMessage>>) {
        let (tx, rx) = mpsc::channel(1);
        (Self { tx }, rx)
    }

    async fn listen(mut self, mut input: ResultStream<ClientMessage>) {
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

    async fn send(&mut self, message: ServerMessage) -> Result<()> {
        Ok(self.tx.send(Ok(message)).await?)
    }

    async fn recv(&mut self, message: ClientMessage) -> Result<()> {
        match message {
            ClientMessage::Heartbeat => self.send(ServerMessage::HeartbeatAck).await,
        }
    }
}

pub async fn connect(input: ResultStream<ClientMessage>) -> Result<ResultStream<ServerMessage>> {
    let (client, rx) = ClientHandle::new();
    tokio::spawn(client.listen(input));
    Ok(rx.into())
}
