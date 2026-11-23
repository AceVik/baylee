//! Each local engine is a replica of Central's authenticated private journal.
use super::{
    Arrival, Failure, HostAssignment, HostRequest, HostResponse, RankedDeck, Shared, StatusCode,
    deck, err, replica_table,
};
use baylee_core::preset::GamePreset;
use baylee_protocol::{execution::Identity, v1};
use central_api::ranked::replica::{self, Digest, Head, Phase, Role, Work};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use uuid::Uuid;

mod inbox;
pub(crate) use inbox::lease;
pub(super) use inbox::poll;
type Placement = replica_table::Placement;
pub(super) type Registry = Mutex<HashMap<Uuid, Arc<Run>>>;
pub(crate) const LEASE: Duration = Duration::from_secs(12);

/// An instance is never reused by another term, or by a replacement engine.
pub(crate) struct Run {
    pub(crate) job: String,
    pub(crate) dead: std::sync::atomic::AtomicBool,
    pub(crate) terminal: std::sync::atomic::AtomicBool,
    pub(crate) work: Work,
    pub(crate) setup: v1::GameSetup,
    pub(crate) opening: Head,
    placement: Placement,
    status: Mutex<(Instant, Phase)>,
    pub(crate) engine_head: Mutex<Option<Head>>,
}

impl Run {
    pub(crate) fn deadline(&self) -> Option<Instant> {
        let (at, phase) = *self.status.lock();
        let until = at + LEASE;
        (phase == Phase::Playing && until > Instant::now()).then_some(until)
    }
    pub(crate) fn activate(&self, state: &Shared, head: Head) {
        let registry = state.ranked.replicas.lock();
        if !self.registered(&registry) {
            return;
        }
        if let Some(deadline) = self.deadline()
            && self.placement.primary
            && *self.engine_head.lock() == Some(head)
            && replica_table::activate(state, &self.placement, deadline).is_ok()
            && let Some(game) = state.lobby.lock().games.get(&self.setup.game_id)
        {
            game.ready.send_replace(true);
        }
    }
    pub(crate) fn is_current(&self, state: &Shared) -> bool {
        state.ranked.require_owner().is_ok() && self.registered(&state.ranked.replicas.lock())
    }
    fn registered(&self, registry: &HashMap<Uuid, Arc<Run>>) -> bool {
        registry
            .get(&self.placement.game)
            .is_some_and(|run| std::ptr::eq(run.as_ref(), self))
    }
    pub(crate) fn fence(&self, state: &Shared) {
        let registry = state.ranked.replicas.lock();
        if self.registered(&registry) {
            replica_table::fence(state, self.placement.game, self.placement.generation);
        }
    }
}

pub(crate) const JOB_PREFIX: &str = "ranked-job-";

pub(crate) fn by_job(state: &Shared, job: &str) -> Option<Arc<Run>> {
    state
        .ranked
        .replicas
        .lock()
        .values()
        .find(|r| r.job == job)
        .cloned()
}

pub(crate) fn identity(state: &Shared, compatibility: Digest) -> Option<Identity> {
    state
        .agents
        .lock()
        .ranked_replica_capacity()
        .map(|(identity, _)| identity)
        .filter(|identity| identity.ranked_compatibility() == Some(compatibility))
}

pub(crate) async fn rpc(
    bridge: &crate::central::Bridge,
    request: replica::HostRequest,
) -> Result<replica::HostResponse, StatusCode> {
    let result = tokio::time::timeout(
        Duration::from_secs(4),
        bridge.ranked(HostRequest::Replicate { request }),
    )
    .await
    .map_err(|_| StatusCode::GATEWAY_TIMEOUT)??;
    match result {
        HostResponse::Replica { response } => Ok(response),
        _ => Err(StatusCode::BAD_GATEWAY),
    }
}

fn invalid() -> Failure {
    err(
        StatusCode::CONFLICT,
        "ranked replication assignment changed",
    )
}

/// Transfer registrations instead of a host-chosen executable preset. Both
/// hosts derive every rule, printing and playable seat from the frozen inputs.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Opening {
    registrations: [RankedDeck; 2],
}

pub(super) async fn freeze(
    state: &Shared,
    bridge: &crate::central::Bridge,
    assignment: &HostAssignment,
    players: [&Arrival; 2],
    generation: u64,
) -> Result<String, Failure> {
    let match_id = assignment.match_id;
    let replica::HostResponse::Assignment { work } =
        rpc(bridge, replica::HostRequest::Inspect { match_id })
            .await
            .map_err(|s| err(s, "ranked assignment unavailable"))?
    else {
        return Err(invalid());
    };
    if work.fence.match_id != match_id
        || work.fence.generation != generation
        || work.role != Role::Primary
        || work.format != assignment.format
        || work.accounts != assignment.accounts
        || work.decks != assignment.decks
        || identity(state, work.compatibility).is_none()
    {
        return Err(invalid());
    }
    if work.game.is_none() {
        if work.phase != Phase::Preparing {
            return Err(invalid());
        }
        let canonical = |player: &Arrival| -> Result<RankedDeck, Failure> {
            let [cards, sideboard, commanders] =
                baylee_cards::ranked::playable_rows(&player.deck).map_err(|_| invalid())?;
            Ok(RankedDeck {
                format: work.format.as_str().to_owned(),
                cards,
                sideboard,
                commanders,
            })
        };
        let frozen = Opening {
            registrations: [canonical(players[0])?, canonical(players[1])?],
        };
        let body = replica::SetupBytes::new(serde_json::to_vec(&frozen).map_err(|_| invalid())?)
            .map_err(|_| err(StatusCode::PAYLOAD_TOO_LARGE, "ranked opening too large"))?;
        let response = rpc(
            bridge,
            replica::HostRequest::Setup {
                fence: work.fence,
                game: match_id,
                body,
                pools: players.map(|p| p.pool.clone()),
            },
        )
        .await
        .map_err(|code| err(code, "ranked setup refused"))?;
        if response != replica::HostResponse::Done {
            return Err(invalid());
        }
    } else if work.game != Some(match_id) {
        return Err(invalid());
    }
    Ok(Uuid::from_bytes(match_id).to_string())
}

async fn reconstruct(
    state: &Shared,
    work: &Work,
    body: &replica::SetupBytes,
    pools: &[Vec<central_api::ranked::PoolEntry>; 2],
) -> Result<GamePreset, Failure> {
    let frozen: Opening = serde_json::from_slice(body.as_slice()).map_err(|_| invalid())?;
    let check = |i: usize| {
        let reg = &frozen.registrations[i];
        async move {
            if reg.format()? != work.format {
                return Err(invalid());
            }
            let (deck, digest, pool) = deck::validate(
                state,
                work.format,
                &reg.cards,
                &reg.sideboard,
                &reg.commanders,
            )
            .await?;
            if digest != work.decks[i] || pool != pools[i] {
                return Err(invalid());
            }
            Ok(deck)
        }
    };
    let (first, second) = tokio::try_join!(check(0), check(1))?;
    deck::preset(state, work.format, [&first, &second], work.seed).await
}
