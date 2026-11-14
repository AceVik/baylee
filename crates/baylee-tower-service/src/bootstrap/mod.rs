use baylee_tower_service::Error;
use baylee_tower_store::Store;
use sea_orm::{ConnectOptions, Database};
use std::{io::Read as _, time::Duration};
use zeroize::Zeroizing;

mod config;
mod serve;

pub(super) fn run() -> Result<(), Error> {
    let mode = mode(std::env::args().skip(1))?;
    let mut bytes = Zeroizing::new(Vec::new());
    std::io::stdin()
        .lock()
        .take((config::MAX_CONFIG + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Contract)?;
    let config = config::Config::parse(&bytes)?;
    drop(bytes);
    if mode == Mode::Check {
        return Ok(());
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|_| Error::Process)?;
    runtime.block_on(start(config, mode))
}

#[derive(PartialEq, Eq)]
enum Mode {
    Serve,
    Check,
    Migrate,
}

fn mode(mut args: impl Iterator<Item = String>) -> Result<Mode, Error> {
    let selected = match args.next().as_deref() {
        None => Mode::Serve,
        Some("--check") => Mode::Check,
        Some("--migrate") => Mode::Migrate,
        _ => return Err(Error::Contract),
    };
    if args.next().is_some() {
        return Err(Error::Contract);
    }
    Ok(selected)
}

async fn start(mut config: config::Config, mode: Mode) -> Result<(), Error> {
    let mut options = ConnectOptions::new(config.database_url.as_str());
    options
        .max_connections(8)
        .min_connections(1)
        .sqlx_logging(false)
        .connect_timeout(Duration::from_secs(5))
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(60));
    let db = Database::connect(options)
        .await
        .map_err(|_| Error::Process)?;
    let result = async {
        let store = Store::new(db.clone(), &config.schema)?;
        if mode == Mode::Migrate {
            return store.migrate().await.map_err(Error::from);
        }
        store.ready().await?;
        let service = config.service.take().ok_or(Error::Contract)?;
        serve::run(store, service, config.development).await
    }
    .await;
    let closed = db.close().await.map_err(|_| Error::Process);
    result.and(closed)
}
