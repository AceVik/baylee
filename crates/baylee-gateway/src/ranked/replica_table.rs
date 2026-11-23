//! A replica has a private engine and no usable player capabilities until its
//! caught-up primary holds a live Central lease for the current generation.
use super::{Binding, Failure, Format, Id, Shared, StatusCode, auth, err};
use crate::lobby::{Lobby, LobbyGame, LobbySeat, LobbyState};
use axum::response::{IntoResponse, Response};
use baylee_core::preset::GamePreset;
use std::time::Instant;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Placement {
    pub(super) match_id: Uuid,
    pub(super) game: Uuid,
    pub(super) format: Format,
    pub(super) accounts: [Id; 2],
    pub(super) decks: [[u8; 32]; 2],
    pub(super) generation: u64,
    pub(super) primary: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Role {
    pub(super) generation: u64,
    primary: bool,
    pub(super) accounts: [Id; 2],
    lease: Option<Instant>,
}

impl Role {
    pub(crate) fn generation(self) -> u64 {
        self.generation
    }
    pub(super) fn serving(self) -> bool {
        self.primary && self.lease.is_some_and(|until| Instant::now() < until)
    }
}

impl Placement {
    fn role(self) -> Role {
        Role {
            generation: self.generation,
            primary: self.primary,
            accounts: self.accounts,
            lease: None,
        }
    }
    fn binding(self) -> Binding {
        Binding {
            id: self.match_id,
            format: self.format,
            decks: self.decks,
            replica: Some(self.role()),
        }
    }
    fn matches(self, binding: Binding) -> bool {
        binding.id == self.match_id
            && binding.format == self.format
            && binding.decks == self.decks
            && binding.replica.is_some_and(|r| r.accounts == self.accounts)
    }
}

fn conflict() -> Failure {
    err(StatusCode::CONFLICT, "ranked replica assignment changed")
}

fn chair(binding: Binding, generation: u64, alias: Id) -> Result<usize, Failure> {
    binding
        .replica
        .filter(|r| r.primary && r.generation == generation)
        .ok_or_else(conflict)?
        .accounts
        .iter()
        .position(|id| *id == alias)
        .ok_or_else(|| err(StatusCode::FORBIDDEN, "no matching ranked seat"))
}

/// Resolve the frozen deck for Central's online check when the client resumes
/// a known game. Grant checks the same assignment again after that round trip.
pub(super) fn assigned(
    state: &Shared,
    id: Uuid,
    generation: u64,
    alias: Id,
) -> Result<Option<(usize, Binding)>, Failure> {
    let lobby = state.lobby.lock();
    let Some(game) = lobby.games.get(&id.to_string()) else {
        return Ok(None);
    };
    let binding = game.ranked.filter(|b| b.id == id).ok_or_else(conflict)?;
    Ok(Some((chair(binding, generation, alias)?, binding)))
}

/// Drop every seat's previous capability and its subscription under the lobby
/// lock. Delayed detach and resync from the old sockets cannot reach this engine.
fn revoke(game: &mut LobbyGame) {
    if let Some(role) = game.ranked.as_mut().and_then(|b| b.replica.as_mut()) {
        role.lease = None;
    }
    for seat in &mut game.seats {
        seat.account_id = None;
        seat.central_session = None;
        seat.seat_token_hash = None;
        seat.socket_pending = None;
        seat.socket_owner = None;
    }
    game.outboxes.clear();
}

/// Realize a verified Central work item and tell the worker whether to start an engine.
/// Installation creates neither an ordinary room nor an authenticated player session.
pub(super) async fn ensure(
    state: &Shared,
    placement: &Placement,
    preset: &GamePreset,
) -> Result<bool, Failure> {
    state.ranked.require_owner()?;
    if placement.accounts[0] == placement.accounts[1]
        || placement.generation == 0
        || placement.match_id != placement.game
        || !super::deck::safe_setup(placement.format, preset)
    {
        return Err(conflict());
    }
    if !baylee_db::ranked::replica::bind(
        &state.db,
        placement.match_id,
        placement.game,
        placement.generation,
        placement.primary,
    )
    .await
    .map_err(|e| crate::db_down(&e.into()))?
    {
        return Err(conflict());
    }
    let mut lobby = state.lobby.lock();
    state.ranked.require_owner()?;
    install(&mut lobby, *placement, preset)
}

/// A restored table has no local account IDs until each player presents a
/// fresh, signed admission on this gateway. Bind only that player's seat.
pub(super) fn grant(
    state: &Shared,
    live: &crate::sessions::Live,
    match_id: Uuid,
    game_id: &str,
    generation: u64,
) -> Result<Response, Failure> {
    let signed = live
        .signed()
        .filter(|s| {
            !s.admin
                && s.ranked_match == Some(*match_id.as_bytes())
                && s.ranked_generation == Some(generation)
        })
        .ok_or_else(conflict)?;
    state.ranked.require_owner()?;
    let mut lobby = state.lobby.lock();
    if !live.stands(crate::sessions::now()) {
        return Err(err(StatusCode::UNAUTHORIZED, "ranked session expired"));
    }
    let Some(game) = lobby.games.get(game_id) else {
        return Ok(StatusCode::ACCEPTED.into_response());
    };
    let binding = game
        .ranked
        .filter(|b| b.id == match_id)
        .ok_or_else(conflict)?;
    let seat = chair(binding, generation, signed.central)?;
    if !binding.seats_live() || game.state != LobbyState::Playing || game.engine.is_none() {
        return Ok(StatusCode::ACCEPTED.into_response());
    }
    crate::routes::participation::account(&lobby, live.account(), Some(game_id))?;
    let chair = lobby
        .games
        .get_mut(game_id)
        .and_then(|g| g.seats.get_mut(seat))
        .filter(|s| {
            !s.participation_ended && s.account_id.as_deref().is_none_or(|a| a == live.account())
        })
        .ok_or_else(conflict)?;
    let token = auth::new_token();
    chair.account_id = Some(live.account().to_owned());
    chair.central_session = Some(live.key().to_vec());
    chair.seat_token_hash = Some(auth::token_hash(&token));
    Ok(crate::Json(crate::routes::SeatGrant::ordinary(
        game_id.to_owned(),
        seat,
        token,
    ))
    .into_response())
}

fn install(lobby: &mut Lobby, placement: Placement, preset: &GamePreset) -> Result<bool, Failure> {
    let id = placement.game.to_string();
    if let Some(game) = lobby.games.get_mut(&id) {
        let binding = game
            .ranked
            .filter(|&binding| placement.matches(binding))
            .ok_or_else(conflict)?;
        let old = binding.replica.ok_or_else(conflict)?;
        if old.generation > placement.generation
            || (old.generation == placement.generation && old.primary != placement.primary)
        {
            return Err(conflict());
        }
        if old.generation != placement.generation {
            revoke(game);
            game.ranked = Some(placement.binding());
        }
        if game.state == LobbyState::Playing {
            return Ok(game.agent_id.is_none());
        }
        // An engine's old teardown must release its agent before a successor
        // can use the same Central game UUID on this host.
        if game.agent_id.is_some() {
            return Ok(false);
        }
    }
    let seats = (0..2).map(LobbySeat::open).collect();
    let mut game = LobbyGame::playing(id.clone(), seats, preset.clone(), auth::now_secs());
    game.house_rules.clone_from(&preset.house_rules);
    game.allow_spectators = false;
    game.ranked = Some(placement.binding());
    lobby.games.insert(id, game);
    Ok(true)
}

/// A successful heartbeat renews only this caught-up primary. If a previous
/// lease already lapsed, never revive the capabilities that expired with it.
pub(super) fn activate(
    state: &Shared,
    placement: &Placement,
    until: Instant,
) -> Result<(), Failure> {
    state.ranked.require_owner()?;
    if !placement.primary || until <= Instant::now() {
        return Err(conflict());
    }
    let mut lobby = state.lobby.lock();
    let game = lobby
        .games
        .get_mut(&placement.game.to_string())
        .filter(|g| g.state == LobbyState::Playing && g.engine.is_some())
        .ok_or_else(conflict)?;
    let role = game
        .ranked
        .filter(|&b| placement.matches(b))
        .and_then(|b| b.replica)
        .filter(|r| r.primary && r.generation == placement.generation)
        .ok_or_else(conflict)?;
    if role.lease.is_some() && !role.serving() {
        revoke(game);
    }
    if let Some(role) = game.ranked.as_mut().and_then(|b| b.replica.as_mut()) {
        role.lease = Some(until);
    }
    Ok(())
}

pub(super) fn fence(state: &Shared, game: Uuid, generation: u64) {
    if let Some(table) = state.lobby.lock().games.get_mut(&game.to_string())
        && table
            .ranked
            .and_then(|b| b.replica)
            .is_some_and(|r| r.generation == generation)
    {
        revoke(table);
    }
}

/// Drop the fenced local incarnation but preserve its durable generation.
/// The worker stops the exact engine before withdrawing it.
pub(super) fn withdraw(state: &Shared, placement: &Placement) {
    let mut lobby = state.lobby.lock();
    let id = placement.game.to_string();
    if lobby.games.get(&id).is_some_and(|game| {
        game.agent_id.is_none()
            && game.ranked.is_some_and(|binding| {
                placement.matches(binding)
                    && binding
                        .replica
                        .is_some_and(|r| r.generation == placement.generation)
            })
    }) {
        lobby.games.remove(&id);
    }
}

/// The matching terminal event also retires the durable local binding.
pub(super) async fn finish(state: &Shared, placement: &Placement) -> Result<(), Failure> {
    fence(state, placement.game, placement.generation);
    let deleted = baylee_db::ranked::replica::finish(
        &state.db,
        placement.match_id,
        placement.game,
        placement.generation,
    )
    .await
    .map_err(|e| crate::db_down(&e.into()))?;
    if deleted {
        withdraw(state, placement);
    }
    Ok(())
}

/// Run on a recent, complete authenticated work list, including pending roles.
pub(super) async fn reconcile_absent(state: &Shared, assigned: &[Uuid]) {
    if state.ranked.require_owner().is_ok()
        && assigned.len() < 64
        && let Err(error) = baylee_db::ranked::replica::reap_absent(&state.db, assigned).await
    {
        tracing::warn!(%error,"could not retire expired ranked host bindings");
    }
}
#[cfg(test)]
mod tests;
