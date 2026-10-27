//! One lobby presence socket per guest session, spanning waiting rooms/games.
use anyhow::{anyhow, bail};
use baylee_seat::link::Socket;
use futures_util::{SinkExt as _, StreamExt as _};
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;

pub struct Presence {
    stop: Option<oneshot::Sender<()>>,
    worker: Option<JoinHandle<anyhow::Result<()>>>,
}

impl Presence {
    pub fn start(mut socket: Socket) -> Self {
        let (stop, mut stopped) = oneshot::channel();
        let worker = tokio::spawn(async move {
            let result = async {
                loop {
                    tokio::select! {
                        _ = &mut stopped => return Ok(()),
                        next = socket.next() => match next {
                            Some(Ok(Message::Ping(bytes))) => socket.send(Message::Pong(bytes)).await
                                .map_err(|_| anyhow!("lobby presence heartbeat failed"))?,
                            Some(Ok(Message::Close(_))) | None => bail!("lobby presence closed; neural seat stopped"),
                            Some(Err(_)) => bail!("lobby presence failed; neural seat stopped"),
                            Some(Ok(_)) => {},
                        }
                    }
                }
            }.await;
            let _ = tokio::time::timeout(Duration::from_secs(2), socket.close(None)).await;
            result
        });
        Self {
            stop: Some(stop),
            worker: Some(worker),
        }
    }

    pub fn check(&self) -> anyhow::Result<()> {
        if self.worker.as_ref().is_none_or(JoinHandle::is_finished) {
            bail!("lobby presence ended; neural seat stopped");
        }
        Ok(())
    }

    pub async fn lost(&mut self) -> anyhow::Result<()> {
        if let Some(worker) = self.worker.as_mut() {
            let _ = worker.await;
        }
        self.worker = None;
        bail!("lobby presence ended; neural seat stopped")
    }

    pub async fn close(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(mut worker) = self.worker.take()
            && tokio::time::timeout(Duration::from_secs(3), &mut worker)
                .await
                .is_err()
        {
            worker.abort();
            let _ = worker.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn presence_answers_ping_and_reports_remote_close_without_remote_text() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(tcp).await.unwrap();
            socket
                .send(Message::Ping(vec![17, 23].into()))
                .await
                .unwrap();
            let reply = tokio::time::timeout(Duration::from_secs(2), socket.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(reply, Message::Pong(vec![17, 23].into()));
            socket.close(None).await.unwrap();
        });
        let (socket, _) = tokio_tungstenite::connect_async(format!("ws://{address}"))
            .await
            .unwrap();
        let mut presence = Presence::start(socket);
        let result = tokio::time::timeout(Duration::from_secs(5), presence.lost())
            .await
            .unwrap();
        assert!(result.is_err());
        assert!(presence.check().is_err());
        presence.close().await;
        server.await.unwrap();
    }

    #[tokio::test]
    async fn explicit_shutdown_closes_the_socket_and_joins_the_worker() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(tcp).await.unwrap();
            assert!(matches!(socket.next().await, Some(Ok(Message::Close(_)))));
        });
        let (socket, _) = tokio_tungstenite::connect_async(format!("ws://{address}"))
            .await
            .unwrap();
        let mut presence = Presence::start(socket);
        presence.close().await;
        assert!(presence.worker.is_none());
        server.await.unwrap();
    }
}
