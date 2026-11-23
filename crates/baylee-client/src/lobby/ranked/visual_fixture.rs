//! A private Play fixture for the passive rendering harness. Menus, layout,
//! controls and notifications are real. Only the account's data and the two
//! queue replies below are supplied locally, without credentials or a host.
use super::{Action, Format, League, LobbyState, api, queue};
use crate::lobby::{Hub, central};
use api::{QueueState, Rank};
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
        app.add_systems(PostUpdate, challenges);
    }
}

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
                QueueState::Waiting {
                    key: *Uuid::now_v7().as_bytes(),
                    format,
                    queued_at: time::OffsetDateTime::now_utc().unix_timestamp(),
                }
            }
        }
        _ => return false,
    };
    state.ranked_net.queue = result;
    state.ranked_net.revision = state.ranked_net.revision.wrapping_add(1);
    true
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
    }
}
