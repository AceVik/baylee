//! What a table is built from: the decks at it, the preset the engine is
//! handed, and the developer's board.

use crate::{
    Deck, ErrorBody, Json, LobbyState, Shared, StatusCode, auth, db_down, engine, err, err_saying,
    expand, lobby, parse_deck_lines, store,
};

/// The most seats a room may have.
///
/// The most seats a room may have — the same eight `GamePreset::validate`
/// allows, so the gateway refuses exactly what the engine would.
pub(crate) const MAX_SEATS: usize = 8;

/// Builds a `LoadedDeck` from a stored deck.
/// Uses the same parser as validation, so counts are already bounded.
pub(crate) fn loaded_deck(
    deck: &Deck,
) -> Result<baylee_cards::decks::LoadedDeck, (StatusCode, Json<ErrorBody>)> {
    let mut main = expand(&parse_deck_lines(&deck.cards)?);
    let side = expand(&parse_deck_lines(&deck.sideboard)?);
    // The commander is stored by name and resolved here, the same way the
    // rows are — and it is *moved* out of the list rather than copied.
    // `DeckBuilder::set_commander` seats the leader among the rows on
    // purpose (a commander outside the list is a deck nobody meant to
    // build), so what arrives here is a hundred rows with the commander
    // among them. Copying it would make a 101st card that sits in the
    // library and the command zone at once: drawable, and two of a legend.
    //
    // Moving it also keeps its printing. The player picked one for that
    // row, and the card that goes to the command zone is the piece of
    // cardboard they picked. A name that resolves to no row at all — a deck
    // posted straight to the API — still gets its commander, at the
    // registry's reference printing.
    //
    // Each leader in turn, so a deck led by two of them loses both from the
    // library and neither of them twice.
    let commanders = deck
        .commanders
        .iter()
        .filter_map(|name| baylee_cards::decks::by_name(name))
        .map(|index| match main.iter().position(|c| c.index == index) {
            Some(at) => main.remove(at),
            None => baylee_cards::decks::DeckCard::plain(index),
        })
        .collect();
    Ok(baylee_cards::decks::LoadedDeck {
        name: deck.name.clone(),
        main,
        sideboard: side,
        commanders,
    })
}

/// Every printing at a table, deduplicated, as the ids the art cache keys on.
///
/// The gateway is the only party that may hold this list whole: a *seat* is
/// entitled to its own deck's printings and earns the rest by seeing the cards,
/// which is why `GameStatic.prints` is a list of holes. Warming from here tells
/// no client anything — it only means the picture is already local by the time
/// the rules let that client ask for it. See `art.rs`.
pub(crate) fn table_prints(preset: &baylee_core::preset::GamePreset) -> Vec<String> {
    // Deduplicated because a deck plays four of a card and the four are one
    // picture — a hundred-card deck is about sixty distinct printings.
    let mut ids: Vec<String> = preset
        .prints
        .iter()
        .map(|p| p.scryfall_id.to_string())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// The acceptance decks, embedded when the gateway is built.
///
/// Not read at run time: a path relative to the working directory made every
/// gateway started anywhere but the repository root (or a directory a deploy
/// had copied the file into) answer an AI seat without a deck with a 500. The
/// decks name cards by the pool compiled into this binary, so the file is part
/// of the build in any case; editing it rebuilds the gateway, as it already
/// rebuilds the client and the seat bridge.
const ACCEPTANCE_DECKS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/acceptance-decks.txt"
));

/// The deck an AI seat plays when the host did not give it one.
pub(crate) fn house_deck() -> Result<baylee_cards::decks::LoadedDeck, (StatusCode, Json<ErrorBody>)>
{
    baylee_cards::decks::load_acceptance(ACCEPTANCE_DECKS, "Victory")
        .map_err(|_e| err(StatusCode::INTERNAL_SERVER_ERROR, "house deck missing"))
}

/// The cards this gateway puts on every named seat's battlefield before turn
/// one, read from `BAYLEE_DEV_SEAT_BOARD`.
///
/// Same variable and same parser as the client's offline harness
/// (`baylee_cards::decks::deal_named`) — `0:Reflecting Pool; 1:Reflecting
/// Pool` seats one on each side of a duel — because a board dealt here and a
/// board dealt there have to be the same board or neither is evidence about
/// the other.
///
/// Behind the `dev-table` feature, and not merely behind the variable. A
/// gateway is somebody's server, and one that seats cards from its own
/// environment is a table whose operator can stack it silently; the feature
/// is what keeps those routes out of a build meant to be run for other
/// people. `validate_the_dev_board` refuses to start a gateway whose spec
/// does not resolve, so a failure here is a name that stopped resolving
/// mid-run rather than a typo.
#[cfg(feature = "dev-table")]
pub(crate) fn deal_the_dev_board(
    preset: &mut baylee_core::preset::GamePreset,
) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    let Ok(spec) = std::env::var("BAYLEE_DEV_SEAT_BOARD") else {
        return Ok(());
    };
    baylee_cards::decks::deal_named(preset, &spec, baylee_cards::decks::DevZone::Battlefield)
        .map_err(|why| {
            tracing::error!("BAYLEE_DEV_SEAT_BOARD: {why}");
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "the dev board could not be dealt",
            )
        })
}

/// No board is dealt in a build without the `dev-table` feature.
#[cfg(not(feature = "dev-table"))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the shape is the feature-gated twin's"
)]
pub(crate) fn deal_the_dev_board(
    _preset: &mut baylee_core::preset::GamePreset,
) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    Ok(())
}

/// Refuses to start on a `BAYLEE_DEV_SEAT_BOARD` that does not resolve.
///
/// The spec is dealt into a throwaway table of `MAX_SEATS` chairs, which is
/// every name and every seat index the real thing could be asked for — so a
/// typo is a gateway that does not come up, and not a room that refuses to
/// start with two people already sitting at it. `dev-reload` sets the
/// precedent: a development switch that quietly does nothing is worse than
/// one that is loud.
///
/// # Panics
///
/// On a name the registry does not answer to, deliberately.
#[cfg(feature = "dev-table")]
pub(crate) fn validate_the_dev_board() {
    let Ok(spec) = std::env::var("BAYLEE_DEV_SEAT_BOARD") else {
        return;
    };
    // Eight chairs because `GamePreset::validate` bounds a table at eight,
    // and empty decks because only the names and the seat indices are being
    // resolved here — the real preset is built per room, from real decks.
    let empty = baylee_cards::decks::LoadedDeck {
        name: String::new(),
        main: vec![],
        sideboard: vec![],
        commanders: vec![],
    };
    let chairs = [&empty; 8];
    let mut probe = baylee_cards::decks::preset_for_all(0, &chairs);
    if let Err(why) = baylee_cards::decks::deal_named(
        &mut probe,
        &spec,
        baylee_cards::decks::DevZone::Battlefield,
    ) {
        panic!("BAYLEE_DEV_SEAT_BOARD: {why}");
    }
    tracing::info!("BAYLEE_DEV_SEAT_BOARD is set: every game starts with `{spec}` on the table");
}

/// Nothing to validate in a build without the `dev-table` feature.
#[cfg(not(feature = "dev-table"))]
pub(crate) fn validate_the_dev_board() {}

/// Preset for a human-vs-AI game (house AI plays Victory).
pub(crate) fn ai_preset(
    deck: &Deck,
    seed: u64,
) -> Result<baylee_core::preset::GamePreset, (StatusCode, Json<ErrorBody>)> {
    let house = house_deck()?;
    let player = loaded_deck(deck)?;
    let mut preset = baylee_cards::decks::preset_for(seed, &player, &house);
    deal_the_dev_board(&mut preset)?;
    Ok(preset)
}

/// Looks a deck up and checks it belongs to the account asking for it.
pub(crate) async fn own_deck(
    state: &Shared,
    account_id: &str,
    deck_id: &str,
) -> Result<(String, Deck), (StatusCode, Json<ErrorBody>)> {
    let deck = store::deck(&state.db, deck_id)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such deck"))?;
    if deck.account_id != account_id {
        return Err(err(StatusCode::FORBIDDEN, "not your deck"));
    }
    Ok((deck.name.clone(), deck))
}

/// Builds the preset a room's seats add up to.
///
/// The controller is set per seat here and nowhere else: the deck helper
/// presets every chair as an AI, which is right for the print table and wrong
/// for everyone at the table who is a person.
pub(crate) fn room_preset(
    seats: &[lobby::LobbySeat],
    seed: u64,
) -> Result<baylee_core::preset::GamePreset, (StatusCode, Json<ErrorBody>)> {
    let mut loaded = Vec::with_capacity(seats.len());
    for seat in seats {
        match &seat.deck {
            Some(deck) => loaded.push(loaded_deck(deck)?),
            // Only reachable for an AI seat the host left alone; a human seat
            // with no deck is not ready and the room has not started.
            None => loaded.push(house_deck()?),
        }
    }
    let refs: Vec<&baylee_cards::decks::LoadedDeck> = loaded.iter().collect();
    let mut preset = baylee_cards::decks::preset_for_all(seed, &refs);
    for (spec, seat) in preset.seats.iter_mut().zip(seats) {
        spec.controller = match seat.kind {
            // `Open` rather than `Human`: the gateway knows the account, the
            // engine knows only that a person answers for this chair.
            lobby::SeatKind::Human => baylee_core::preset::SeatController::Open,
            lobby::SeatKind::Ai => baylee_core::preset::SeatController::Ai(
                seat.ai
                    .as_deref()
                    .and_then(baylee_core::preset::AIProfile::named)
                    .unwrap_or_default(),
            ),
        };
        spec.team = seat.team;
    }
    deal_the_dev_board(&mut preset)?;
    // The engine refuses a table with only one side on it, and so does the
    // lobby — here rather than at the first state-based action, so the room
    // says why instead of starting a game that is already over.
    if preset.validate().is_err() {
        return Err(err(
            StatusCode::CONFLICT,
            "every seat is on the same team; a game needs at least two sides",
        ));
    }
    Ok(preset)
}

/// Starts a room whose seats are all settled.
///
/// A room used to start itself the moment the last chair became ready, which
/// read well until "ready" stopped meaning "has a deck": a player who picked
/// a deck to look at it was already in a game. Now every chair says it is
/// ready and the host says go, which is two different statements by two
/// different people and needs both.
///
/// Returns whether the game was started. The engine is ordered *outside* the
/// lobby lock, and a failure to order one puts the room back the way it was
/// rather than leaving a table nobody can play at.
pub(crate) fn try_start(state: &Shared, id: &str) -> Result<bool, (StatusCode, Json<ErrorBody>)> {
    let prints;
    {
        let mut lobby = state.lobby.lock();
        let Some(game) = lobby.games.get_mut(id) else {
            return Ok(false);
        };
        if game.state != LobbyState::Waiting || !game.all_ready() {
            return Ok(false);
        }
        let mut preset = room_preset(&game.seats, auth::new_game_seed())?;
        // The room's clock, onto the preset the engine is about to be given.
        // This is the one line the whole ticket was missing: the wire already
        // carried `HouseRules` — the gateway sends the preset as JSON and
        // gamehost has always decoded it — so every table ran the default
        // purely because nothing here ever wrote to this field.
        preset.house_rules = game.house_rules.clone();
        baylee_cards::decks::apply_room_setup(&mut preset, &game.setup)
            .map_err(|why| err_saying(StatusCode::BAD_REQUEST, why))?;
        prints = table_prints(&preset);
        game.preset = Some(preset);
        game.state = LobbyState::Playing;
        game.started_at = Some(auth::now_secs());
    }
    // Outside the lobby lock: it spawns a task rather than doing the work, but
    // a mutex held across anything that touches the network is how a lobby
    // route starts waiting on Scryfall.
    state.art.warm(prints);
    if let Err(reason) = engine::start_engine(state, id) {
        let mut lobby = state.lobby.lock();
        if let Some(game) = lobby.games.get_mut(id) {
            game.state = LobbyState::Waiting;
            game.preset = None;
            game.started_at = None;
        }
        tracing::error!(game_id = id, reason, "could not start a game");
        return Err(err(StatusCode::SERVICE_UNAVAILABLE, "no engine available"));
    }
    Ok(true)
}
