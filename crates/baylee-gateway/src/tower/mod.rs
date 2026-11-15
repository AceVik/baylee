//! Gateway execution authority; progression and records remain in the global service.
use crate::{Mutex, Shared};
use baylee_account::{Error, TowerHost};
use baylee_tower_api::host::{Claimed, Request, Response};
use std::{collections::BTreeMap, sync::Arc};
use uuid::Uuid;

mod config;
mod heartbeat;
mod launch;
pub(crate) mod routes;
pub(crate) use heartbeat::Heartbeat;
const CAPACITY: usize = 32;

pub(crate) struct Tower {
    pub client: TowerHost,
    origin: String,
    starts: tokio::sync::Mutex<()>,
    pending: Mutex<BTreeMap<Uuid, Pending>>,
    games: Mutex<BTreeMap<String, Arc<Game>>>,
}

struct Pending {
    account: Uuid,
    claim: Uuid,
}

pub(crate) struct Game {
    run: Uuid,
    pub claimed: Claimed,
    stop: tokio::sync::watch::Sender<bool>,
}

impl Game {
    pub(crate) fn run_id(&self) -> Uuid {
        self.run
    }

    pub(crate) fn sealed(&self) {
        let _ = self.stop.send(true);
    }
}

impl Tower {
    pub(crate) fn game(&self, id: &str) -> Option<Arc<Game>> {
        self.games.lock().get(id).cloned()
    }

    pub(crate) fn ended(&self, id: &str) {
        if let Some(game) = self.games.lock().remove(id) {
            let _ = game.stop.send(true);
            self.pending.lock().remove(&game.run);
        }
    }

    async fn claim(&self, account: Uuid, run: Uuid) -> Result<Claimed, Error> {
        let claim = {
            let mut pending = self.pending.lock();
            if pending.len() >= CAPACITY && !pending.contains_key(&run) {
                return Err(Error::Busy);
            }
            let entry = pending.entry(run).or_insert_with(|| Pending {
                account,
                claim: Uuid::now_v7(),
            });
            if entry.account != account {
                return Err(Error::Rejected);
            }
            entry.claim
        };
        match self
            .client
            .control(&Request::Claim {
                account,
                run,
                claim,
            })
            .await
        {
            Ok(Response::Claimed { game }) => Ok(*game),
            Ok(_) => Err(Error::Unavailable),
            Err(error) => {
                if matches!(
                    error,
                    Error::Rejected
                        | Error::Conflict
                        | Error::TermsRequired
                        | Error::UpdateRequired
                ) {
                    self.pending.lock().remove(&run);
                }
                Err(error)
            }
        }
    }

    async fn advertise(&self, state: &Shared) -> Result<routes::Availability, Error> {
        let inventory = state
            .agents
            .lock()
            .tower_inventory()
            .ok_or(Error::Unavailable)?;
        let Response::Advertised { host } = self
            .client
            .control(&Request::Advertise {
                engines: inventory.installed,
                available: if state.admission.held() {
                    Vec::new()
                } else {
                    inventory.available.clone()
                },
            })
            .await?
        else {
            return Err(Error::Unavailable);
        };
        Ok(routes::Availability {
            host,
            origin: self.origin.clone(),
            engines: if state.admission.held() {
                Vec::new()
            } else {
                inventory.available
            },
        })
    }
}

pub(crate) fn ended(state: &Shared, game: &str) {
    if let Some(tower) = &state.tower {
        tower.ended(game);
    }
}
