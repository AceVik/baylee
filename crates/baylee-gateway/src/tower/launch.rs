use super::{Arc, Claimed, Game, Request, Shared, Tower, Uuid, routes};
use crate::{ErrorBody, Json, StatusCode, auth, engine, lobby};
use tokio::sync::watch;

pub(super) fn start(
    state: &Shared,
    tower: &Arc<Tower>,
    run: Uuid,
    player: routes::Player,
    claimed: Claimed,
) -> Result<crate::routes::SeatGrant, (StatusCode, Json<ErrorBody>)> {
    let identity = &claimed.package.cohort.executor;
    if state.agents.lock().pick_exact(identity).is_none() {
        return Err(routes::unavailable());
    }
    let preset = claimed
        .package
        .preset(&claimed.catalog, identity, |card| {
            baylee_cards::by_index(card).map(|_| baylee_cards::decks::reference_print(card))
        })
        .map_err(|_| routes::unavailable())?;
    // A fresh execution gets a distinct transport identity. Late stop/status/
    // socket callbacks from its predecessor must never address the new table.
    // The authority reservation and retained rules prefix remain unchanged.
    let game_id = claimed.lease.claim.to_string();
    let token = auth::new_token();
    let mut seats = vec![lobby::LobbySeat::open(0), lobby::LobbySeat::open(1)];
    seats[0].account_id = Some(player.local.clone());
    seats[0].seat_token_hash = Some(auth::token_hash(&token));
    seats[0].central_session = Some(player.session);
    seats[1].kind = lobby::SeatKind::Ai;
    seats[1].ai = Some("expert".into());
    let mut game = lobby::LobbyGame::playing(game_id.clone(), seats, preset, auth::now_secs());
    game.execution = Some(identity.clone());
    game.allow_spectators = false;
    let (stop, receive) = watch::channel(false);
    let lease = claimed.lease;
    let entry = Arc::new(Game { run, claimed, stop });
    {
        let mut lobby = state.lobby.lock();
        crate::routes::participation::account(&lobby, &player.local, None)?;
        if lobby
            .games
            .get(&game_id)
            .is_some_and(|game| game.state == lobby::LobbyState::Playing)
        {
            return Err(routes::unavailable());
        }
        lobby.games.insert(game_id.clone(), game);
        tower.games.lock().insert(game_id.clone(), entry);
    }
    renew(
        state.clone(),
        tower.clone(),
        game_id.clone(),
        lease,
        receive,
    );
    if engine::start_engine(state, &game_id).is_err() {
        engine::end_game(state, &game_id);
        return Err(routes::unavailable());
    }
    state.lobby_moved();
    Ok(crate::routes::SeatGrant {
        game_id,
        seat: 0,
        seat_token: token,
        challenge: None,
    })
}

fn renew(
    state: Shared,
    tower: Arc<Tower>,
    game: String,
    lease: baylee_tower_api::host::Lease,
    mut stop: watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = stop.changed() => return,
                () = tokio::time::sleep(std::time::Duration::from_secs(20)) => {},
            }
            let request = Request::Renew { lease };
            let result = tokio::select! {
                biased;
                _ = stop.changed() => return,
                result = tower.client.control(&request) => result,
            };
            if result.is_err() {
                engine::end_game(&state, &game);
                return;
            }
        }
    });
}
