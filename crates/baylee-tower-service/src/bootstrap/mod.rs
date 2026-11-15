use baylee_tower_service::Error;
use baylee_tower_store::Store;
use sea_orm::{ConnectOptions, Database};
use std::{io::Read as _, time::Duration};
use zeroize::Zeroizing;

mod admin;
mod config;
mod serve;

pub(super) fn run() -> Result<(), Error> {
    let mode = mode(std::env::args().skip(1))?;
    let bound = if matches!(mode, Mode::Admin | Mode::CheckAdmin) {
        admin::MAX_INPUT
    } else {
        config::MAX_CONFIG
    };
    let mut bytes = Zeroizing::new(Vec::new());
    std::io::stdin()
        .lock()
        .take((bound + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Contract)?;
    let (config, operation) = if matches!(mode, Mode::Admin | Mode::CheckAdmin) {
        let input = admin::Input::parse(&bytes)?;
        (input.configuration, Some(input.operation))
    } else {
        (config::Config::parse(&bytes)?, None)
    };
    drop(bytes);
    if matches!(mode, Mode::Check | Mode::CheckAdmin) {
        return Ok(());
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|_| Error::Process)?;
    runtime.block_on(start(config, mode, operation))
}

#[derive(PartialEq, Eq)]
enum Mode {
    Serve,
    Check,
    Migrate,
    Admin,
    CheckAdmin,
}

fn mode(mut args: impl Iterator<Item = String>) -> Result<Mode, Error> {
    let selected = match args.next().as_deref() {
        None => Mode::Serve,
        Some("--check") => Mode::Check,
        Some("--migrate") => Mode::Migrate,
        Some("--admin") => Mode::Admin,
        Some("--check-admin") => Mode::CheckAdmin,
        _ => return Err(Error::Contract),
    };
    if args.next().is_some() {
        return Err(Error::Contract);
    }
    Ok(selected)
}

async fn start(
    mut config: config::Config,
    mode: Mode,
    operation: Option<admin::Operation>,
) -> Result<(), Error> {
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
        if let Some(operation) = operation {
            let reply = operation.execute(&store, config.development).await?;
            // Only closed public result variants reach stdout; never the config,
            // credentials, private house decks or database diagnostics.
            println!(
                "{}",
                serde_json::to_string(&reply).map_err(|_| Error::Contract)?
            );
            return Ok(());
        }
        store.ready().await?;
        let service = config.service.take().ok_or(Error::Contract)?;
        serve::run(store, service, config.development).await
    }
    .await;
    let closed = db.close().await.map_err(|_| Error::Process);
    result.and(closed)
}
