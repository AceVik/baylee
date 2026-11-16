//! One PostgreSQL LISTEN connection per service, with bounded coalescing fan-out.
use baylee_tower_store::Store;
use std::time::Duration;
use tokio::{sync::broadcast, task::JoinHandle};
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
pub(super) enum Change {
    Run(Uuid),
    Draft(Uuid),
    Ranking,
    Catalog,
    Resync,
}

pub(super) struct Changes {
    send: broadcast::Sender<Change>,
    task: JoinHandle<()>,
}

impl Changes {
    pub(super) fn start(store: Store) -> Self {
        let (send, _) = broadcast::channel(128);
        let news = send.clone();
        let task = tokio::spawn(async move {
            loop {
                if let Ok(mut listener) = store.changes().await {
                    // A reconnect may have missed commits; snapshots are authoritative.
                    let _ = news.send(Change::Resync);
                    loop {
                        match listener.try_recv().await {
                            Ok(Some(event)) => {
                                let hint = match event.payload() {
                                    "ranking" => Some(Change::Ranking),
                                    "catalog" => Some(Change::Catalog),
                                    payload => payload
                                        .strip_prefix("run:")
                                        .and_then(|id| Uuid::parse_str(id).ok())
                                        .map(Change::Run)
                                        .or_else(|| {
                                            payload
                                                .strip_prefix("draft:")
                                                .and_then(|id| Uuid::parse_str(id).ok())
                                                .map(Change::Draft)
                                        }),
                                };
                                if let Some(hint) = hint {
                                    let _ = news.send(hint);
                                }
                            }
                            Ok(None) => {
                                let _ = news.send(Change::Resync);
                            }
                            Err(_) => break,
                        }
                    }
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });
        Self { send, task }
    }

    pub(super) fn subscribe(&self) -> broadcast::Receiver<Change> {
        self.send.subscribe()
    }
}

impl Drop for Changes {
    fn drop(&mut self) {
        self.task.abort();
    }
}
