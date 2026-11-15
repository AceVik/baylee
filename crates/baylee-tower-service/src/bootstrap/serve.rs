use super::config::Service;
use baylee_tower_service::{Artifact, Courier, Error, Hosts, Players, Queue};
use baylee_tower_store::Store;
use std::sync::Arc;
use tokio::{net::TcpListener, sync::watch, task::JoinSet};

pub(super) async fn run(store: Store, service: Service, development: bool) -> Result<(), Error> {
    let shutdown = Shutdown::new()?;
    let endpoint = service.endpoint(development)?;
    let players = Players::new(
        store.clone(),
        &endpoint,
        &service.origin,
        service.record_bytes,
    )?;
    let courier = if let Some(awards) = service.awards {
        let principal = store.authenticate_operator(awards.courier.digest()).await?;
        Some(Courier::new(
            store.clone(),
            principal,
            &endpoint,
            &service.origin,
            awards.issuer,
        )?)
    } else {
        None
    };
    let mut queues = Vec::with_capacity(service.workers.len());
    for worker in service.workers {
        let verifier = store
            .authenticate_operator(worker.verifier.digest())
            .await?;
        let supervisor = store
            .authenticate_operator(worker.supervisor.digest())
            .await?;
        let artifact = Artifact::probe(&worker.artifact).await?;
        if store.published(worker.cohort).await?.cohort.executor != *artifact.identity() {
            return Err(Error::Contract);
        }
        queues.push((
            worker.cohort,
            Queue::new(
                store.clone(),
                verifier,
                supervisor,
                artifact,
                worker.resources,
            )?,
        ));
    }
    let router = Arc::new(players)
        .router()
        .merge(Arc::new(Hosts::new(store)).router())
        .route("/health", axum::routing::get(|| async { "ready" }));
    let listener = TcpListener::bind(service.listen)
        .await
        .map_err(|_| Error::Process)?;
    let address = listener.local_addr().map_err(|_| Error::Process)?;
    let (stop, receive) = watch::channel(false);
    let mut tasks = JoinSet::new();
    let http_stop = receive.clone();
    tasks.spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(stopped(http_stop))
            .await
            .map_err(|_| Error::Process)
    });
    for (cohort, queue) in queues {
        let stop = receive.clone();
        tasks.spawn(async move { queue.serve(cohort, stopped(stop)).await.map(|_| ()) });
    }
    if let Some(mut courier) = courier {
        let stop = receive.clone();
        tasks.spawn(async move {
            let summary = courier.serve(stopped(stop)).await;
            println!("Tower accomplishment delivery stopped: {summary:?}");
            Ok(())
        });
    }
    // Public listener identity only; no configuration, credential or database diagnostics.
    println!("Tower ready on {address}");
    let mut result = tokio::select! {
        result = shutdown.wait() => result,
        result = tasks.join_next() => joined(result),
    };
    let _ = stop.send(true);
    while let Some(done) = tasks.join_next().await {
        result = result.and(joined(Some(done)));
    }
    result
}

fn joined(result: Option<Result<Result<(), Error>, tokio::task::JoinError>>) -> Result<(), Error> {
    result.ok_or(Error::Process)?.map_err(|_| Error::Process)?
}

async fn stopped(mut stop: watch::Receiver<bool>) {
    while !*stop.borrow_and_update() {
        if stop.changed().await.is_err() {
            break;
        }
    }
}

struct Shutdown {
    #[cfg(unix)]
    term: tokio::signal::unix::Signal,
}
impl Shutdown {
    fn new() -> Result<Self, Error> {
        Ok(Self {
            #[cfg(unix)]
            term: tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .map_err(|_| Error::Process)?,
        })
    }
    async fn wait(self) -> Result<(), Error> {
        #[cfg(unix)]
        {
            let mut term = self.term;
            tokio::select! { result = tokio::signal::ctrl_c() => result.map_err(|_| Error::Process), _ = term.recv() => Ok(()) }
        }
        #[cfg(not(unix))]
        {
            tokio::signal::ctrl_c().await.map_err(|_| Error::Process)
        }
    }
}
