//! The kept games read back as the gateway starts, and the engines they are
//! resumed on.
//!
//! Nothing here may stop the gateway from starting: a row that cannot be
//! read, a record whose header is not there, an engine nobody knows each end
//! that one game, with the reason logged, and the start goes on. Only the
//! header of each record is read here; the whole record is read when the
//! game's engine attaches (`engine/game.rs`).

use super::row::{self, Row};
use crate::Shared;
use crate::lobby::{LobbyGame, LobbyState};
use baylee_core::preset::GamePreset;
use baylee_protocol::execution::{Identity, Variant};
use std::io::BufRead as _;
use std::io::Read as _;

/// How long a restored game waits for its engine to attach, in seconds:
/// longer than the agent waits between dials (30 s), and an engine's start,
/// several times over. A game still without an engine then ends, as every
/// game did at a restart before games were kept.
pub(crate) const RESTORE_WAIT_SECS: u64 = 180;

/// The most a record's header may unpack to: a preset of four seats' decks
/// and printings is far less.
const MAX_HEADER_BYTES: u64 = 64 << 20;

/// What a record's first line says that a resumed game needs.
#[derive(serde::Deserialize)]
pub(super) struct Header {
    kind: String,
    pub(super) preset: GamePreset,
    #[serde(default)]
    pub(super) execution: Option<Identity>,
}

/// Reads every kept game back into the lobby, running and waiting for its
/// engine, and starts the clock each waits on. Before the gateway serves.
pub(crate) async fn restore(state: &Shared) {
    match baylee_db::running::forget_finished(&state.db).await {
        Ok(0) => {}
        Ok(gone) => tracing::info!(gone, "rows of finished games removed"),
        Err(e) => tracing::warn!("removing the rows of finished games: {e}"),
    }
    let rows = match baylee_db::running::all(&state.db).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("reading the running games: {e}; none is resumed");
            return;
        }
    };
    // The writer owns every row read: one whose game does not come back
    // goes at its first sweep, whatever stops it here.
    state.keeper.found(rows.iter().map(|row| row.game_id.clone()));
    let mut games = Vec::new();
    for kept in rows {
        match one(state, &kept).await {
            Ok((row, game)) => games.push((row, game)),
            Err(why) => {
                tracing::warn!(game_id = kept.game_id, why, "a running game is not resumed");
                if let Err(e) = baylee_db::running::forget(&state.db, &kept.game_id).await {
                    tracing::warn!(game_id = kept.game_id, "forgetting it: {e}");
                }
            }
        }
    }
    let accounts: Vec<uuid::Uuid> = games
        .iter()
        .flat_map(|(row, _)| row.accounts())
        .filter_map(|account| uuid::Uuid::parse_str(&account).ok())
        .collect();
    let gone = match baylee_db::running::gone_accounts(&state.db, &accounts).await {
        Ok(gone) => gone,
        Err(e) => {
            tracing::error!("checking the accounts of the running games: {e}; none is resumed");
            return;
        }
    };
    let ids: Vec<String> = games.iter().map(|(_, game)| game.id.clone()).collect();
    {
        let mut lobby = state.lobby.lock();
        for (_, game) in games {
            lobby.games.insert(game.id.clone(), game);
        }
        // As the deletion would have done to them in a gateway that ran.
        let now = crate::auth::now_secs();
        for account in &gone {
            lobby.forget_account(&account.to_string(), now);
        }
    }
    for id in &ids {
        wait_for_engine(state.clone(), id.clone());
    }
    if !ids.is_empty() {
        tracing::info!(
            games = ids.len(),
            "running games restored; they wait for their engines"
        );
    }
}

/// One kept game, read back.
async fn one(
    state: &Shared,
    kept: &baylee_db::running::Running,
) -> Result<(Row, LobbyGame), &'static str> {
    if kept.kept != row::VERSION {
        return Err("its row is of another version");
    }
    let row: Row = serde_json::from_str(&kept.body).map_err(|_| "its row does not read")?;
    let piece = baylee_db::records::first_piece(&state.db, &kept.game_id)
        .await
        .map_err(|_| "its record could not be read")?
        .ok_or("its record has no piece")?;
    let header = tokio::task::spawn_blocking(move || header_of(&piece))
        .await
        .map_err(|_| "its record could not be read")??;
    let execution = header
        .execution
        .filter(Identity::valid)
        .ok_or("its record names no engine")?;
    if execution.variant == Variant::Tower {
        return Err("it is a Tower run");
    }
    let game = row
        .clone()
        .game(kept.game_id.clone(), header.preset, execution);
    Ok((row, game))
}

/// The header of a record whose first piece is `piece`.
pub(super) fn header_of(piece: &[u8]) -> Result<Header, &'static str> {
    let mut line = Vec::new();
    std::io::BufReader::new(flate2::read::GzDecoder::new(piece))
        .take(MAX_HEADER_BYTES)
        .read_until(b'\n', &mut line)
        .map_err(|_| "its record does not unpack")?;
    if line.last() != Some(&b'\n') {
        return Err("its record's header is cut off");
    }
    let header: Header = serde_json::from_slice(&line).map_err(|_| "its header does not read")?;
    if header.kind != "header" {
        return Err("its record does not begin with a header");
    }
    Ok(header)
}

/// The ids of the games restored and still waiting for their engine.
pub(super) fn restored_ids(state: &Shared) -> Vec<String> {
    let lobby = state.lobby.lock();
    let mut ids: Vec<String> = lobby
        .games
        .values()
        .filter(|game| game.restored())
        .map(|game| game.id.clone())
        .collect();
    ids.sort();
    ids
}

/// Orders an engine for every restored game that has none ordered: when an
/// agent says hello, and at every sweep. A game no agent can run yet waits.
pub(crate) fn order_restored(state: &Shared) {
    let waiting: Vec<String> = {
        let lobby = state.lobby.lock();
        let mut ids: Vec<String> = lobby
            .games
            .values()
            .filter(|game| {
                game.restored() && game.state == LobbyState::Playing && game.agent_id.is_none()
            })
            .map(|game| game.id.clone())
            .collect();
        ids.sort();
        ids
    };
    for id in waiting {
        if let Err(why) = crate::engine::start_engine(state, &id) {
            tracing::debug!(game_id = id, why, "a restored game waits for an engine");
        }
    }
}

/// Ends `game_id` if its engine has not attached [`RESTORE_WAIT_SECS`] from
/// now.
fn wait_for_engine(state: Shared, game_id: String) {
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(RESTORE_WAIT_SECS)).await;
        let ended = {
            let mut lobby = state.lobby.lock();
            lobby.games.get_mut(&game_id).is_some_and(|game| {
                if game.state != LobbyState::Playing || game.engine.is_some() || !game.restored() {
                    return false;
                }
                game.finish(crate::auth::now_secs());
                true
            })
        };
        if ended {
            tracing::warn!(
                game_id,
                "no engine resumed the game after the restart; game closed"
            );
            state.keeper.touch(&game_id);
            state.lobby_moved();
            crate::engine::stop_engine(&state, &game_id);
        }
    });
}
