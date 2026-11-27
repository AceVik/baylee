//! A private Play fixture for the passive rendering harness. Menus, layout,
//! controls and notifications are real. Only the account's data and the two
//! queue replies below are supplied locally, without credentials or a host.
use super::{Action, Format, League, LobbyState, api, queue};
use crate::lobby::{Hub, central};
use api::ai::AiOpponents;
use api::{QueueState, Rank};
use baylee_client_core::aiplayers::Opponents;
use baylee_client_core::{
    challenges::api as tasks,
    lobby::{DeckSummary, GameListing, GameSeat, GameSummary, LobbyEvent},
};
use bevy::prelude::*;
use uuid::Uuid;

fn enabled() -> bool {
    std::env::var("BAYLEE_E2E_RANKED_FIXTURE").as_deref() == Ok("1")
        && std::env::var_os("BAYLEE_DEV_PASSIVE").is_some()
        && std::env::var_os("BAYLEE_DEV_CONTROL").is_some()
        && std::env::var("BAYLEE_GATEWAY").as_deref() == Ok("http://127.0.0.1:9")
}
fn fixture_session(state: &LobbyState) -> bool {
    // A real Central account has a UUIDv7 and a real gateway bearer.
    state.lobby.token() == Some("")
        && state
            .central_handle()
            .is_some_and(|h| h.public_id == Some([0; 16]))
}

pub(super) fn install(app: &mut App) {
    if enabled() {
        app.add_systems(PostUpdate, challenges)
            .add_systems(Update, seat_an_ai);
    }
}

/// A second after the fill-in time, the fixture seats an AI as Central would:
/// the house for a search taking the local AIs, a language model for one
/// taking every AI. The desk then shows the match found with its badge.
fn seat_an_ai(mut state: ResMut<LobbyState>) {
    if !fixture_session(&state) {
        return;
    }
    let QueueState::Waiting {
        key,
        format,
        ai_fill_in_at: Some(at),
        ..
    } = state.ranked_net.queue
    else {
        return;
    };
    if time::OffsetDateTime::now_utc().unix_timestamp() < at + 1 {
        return;
    }
    let (name, opponent) = match state.ranked.opponents {
        Some(Opponents::Any) => (
            "KI-Haiku-Hoch#0f3a",
            api::ai::AiBadge {
                kind: api::ai::AiKind::Model,
                label: "Claude Haiku · Hoch".into(),
            },
        ),
        _ => (HOUSE_NAME, house()),
    };
    let match_id = *Uuid::now_v7().as_bytes();
    let assigned = QueueState::Assigned {
        key,
        assignment: api::Assignment {
            match_id,
            host_id: *Uuid::now_v7().as_bytes(),
            origin: "https://host.invalid".into(),
            format,
            account: [0; 16],
            seat: 0,
            deck: [0; 32],
            game: None,
            opponent_ai: Some(opponent.clone()),
        },
    };
    let ai = state
        .ranked
        .opponents
        .map_or(AiOpponents::None, Opponents::wire);
    let net = &mut state.bypass_change_detection().ranked_net;
    net.show_for_fixture(assigned, ai);
    net.name_for_fixture(match_id, name, Some(opponent));
}

/// The house AI's account, as an administrator named it.
const HOUSE_NAME: &str = "KI-Haus-Expert#6699";

pub(crate) fn populate(state: &mut LobbyState) -> bool {
    if !enabled() || state.central_ready() || state.lobby.token().is_some() {
        return false;
    }
    central::enable(state);
    let Some(account) = &mut state.central else {
        return false;
    };
    account.handle = Some(central::Handle::inert_fixture_handle());
    account.profile_handle = Some("Laurel#0001".into());
    // This fixture supplies its two table rows in memory. As in the ordinary
    // headless lobby harness, no endpoint means there is no live feed to dial.
    state.gateway.clear();
    state.gateway_selected = true;
    state.hub = Hub::Play;
    state.lobby.apply(LobbyEvent::LoggedIn {
        token: String::new(),
        username: Some("Laurel".into()),
    });
    let decks = [
        ("Morgenrot", Format::Standard),
        ("Waldwache", Format::Standard),
        ("Sternenmeer", Format::Highlander),
        ("Solstice", Format::Commander),
    ];
    state.lobby.apply(LobbyEvent::Decks(
        decks
            .into_iter()
            .map(|(name, format)| DeckSummary {
                id: Uuid::now_v7().to_string(),
                name: name.into(),
                format: format.as_str().into(),
                copies: if format == Format::Standard { 60 } else { 100 },
                ..default()
            })
            .collect(),
    ));
    let tables = ["Abendrunde", "Offenes Duell"]
        .into_iter()
        .map(|name| GameSummary {
            id: Uuid::now_v7().to_string(),
            name: name.into(),
            host: Some("Mistral".into()),
            state: "waiting".into(),
            seats: vec![
                GameSeat {
                    seat: 0,
                    host: true,
                    player: Some("Mistral".into()),
                    taken: true,
                    ..default()
                },
                GameSeat {
                    seat: 1,
                    ..default()
                },
            ],
            ..default()
        })
        .collect();
    state
        .lobby
        .apply(LobbyEvent::Games(GameListing::of(tables)));
    true
}

// The first account tick invalidates any previous account's challenge data.
// Seed this fixture's catalogue after that tick, as an actual reply arrives.
fn challenges(mut state: ResMut<LobbyState>, mut stage: Local<u8>) {
    if *stage >= 2 || !fixture_session(&state) {
        return;
    }
    *stage += 1;
    if *stage == 1 {
        return;
    }
    let model = &mut state.challenges.model;
    model.busy = false;
    model.loaded = true;
    model.now = time::OffsetDateTime::now_utc().unix_timestamp();
    model.catalogue = [
        (tasks::Kind::Daily, "Weggabelung", "Crossroads"),
        (tasks::Kind::Weekly, "Auf der Passhöhe", "Over the pass"),
    ]
    .into_iter()
    .filter_map(|(kind, de, en)| {
        Some(tasks::Challenge {
            id: *Uuid::now_v7().as_bytes(),
            digest: [0; 32],
            withdrawn: false,
            description: tasks::Description {
                period: tasks::Period::containing(kind, model.now).ok()?,
                title: tasks::Words {
                    de: de.into(),
                    en: en.into(),
                },
                instructions: tasks::Words {
                    de: String::new(),
                    en: String::new(),
                },
                deck: String::new(),
                executor: String::new(),
                target_turn: 1,
                expected_minutes: 10,
            },
        })
    })
    .collect();
}

pub(super) fn reply(state: &mut LobbyState, action: Action) -> bool {
    if !matches!(action, Action::Join | Action::Cancel) || !enabled() || !fixture_session(state) {
        return false;
    }
    let format = queue::effective_format(state);
    let result = match action {
        Action::Cancel if matches!(state.ranked_net.queue, QueueState::Waiting { .. }) => {
            QueueState::Idle
        }
        Action::Join if queue::selected(state).is_some() => {
            if format == Format::Commander {
                let match_id = *Uuid::now_v7().as_bytes();
                state.ranked_net.played = Some(match_id);
                settled(format, match_id)
            } else {
                let ai = state
                    .ranked
                    .opponents
                    .map_or(AiOpponents::None, Opponents::wire);
                searching(format, ai)
            }
        }
        _ => return false,
    };
    let ai = state
        .ranked
        .opponents
        .map_or(AiOpponents::None, Opponents::wire);
    let settled_against = match &result {
        QueueState::Finished { match_id, .. } => Some(*match_id),
        _ => None,
    };
    state.ranked_net.show_for_fixture(result, ai);
    if let Some(match_id) = settled_against {
        state
            .ranked_net
            .name_for_fixture(match_id, HOUSE_NAME, Some(house()));
    }
    true
}

/// A waiting search, as Central would answer it: an AI may be seated in
/// eighteen seconds when the search accepts one, so the countdown fades in
/// three seconds after the search began.
fn searching(format: Format, ai: AiOpponents) -> QueueState {
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    QueueState::Waiting {
        key: *Uuid::now_v7().as_bytes(),
        format,
        queued_at: now,
        ai_fill_in_at: (!ai.is_none()).then_some(now + 18),
    }
}

/// The desk's choice moved while the fixture's search waits: the search
/// moves with it at once, as Central's cancel and join would.
pub(super) fn moved(state: &mut LobbyState, choice: Opponents) -> bool {
    if !enabled() || !fixture_session(state) {
        return false;
    }
    let QueueState::Waiting { format, .. } = state.ranked_net.queue else {
        return false;
    };
    state
        .ranked_net
        .show_for_fixture(searching(format, choice.wire()), choice.wire());
    true
}

/// The house AI the fixture's settled game was played against.
fn house() -> api::ai::AiBadge {
    api::ai::AiBadge {
        kind: api::ai::AiKind::House,
        label: "Haus-KI · Expert".into(),
    }
}

fn settled(format: Format, match_id: api::Id) -> QueueState {
    let rank = |league, division| Rank {
        league,
        division: Some(division),
        champion_place: None,
    };
    QueueState::Finished {
        key: *Uuid::now_v7().as_bytes(),
        match_id,
        format,
        outcome: api::Outcome::FirstWins,
        seat: 0,
        elo: 1810,
        delta: 12,
        before: rank(League::Gold, 2),
        after: rank(League::Gold, 1),
        deck: api::DeckResult {
            before: rank(League::Gold, 1),
            after: rank(League::Platinum, 4),
            elo: 1845,
            delta: 11,
            placement_games: 10,
        },
        opponent_ai: Some(house()),
    }
}
