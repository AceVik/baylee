mod decks;
mod fields;
mod offline;
mod rooms;
mod seating;
mod sign_in;
mod status;
mod table_list;

use super::*;

use crate::deckbuilder::{Coverage, PoolCard, Zone};

/// A signed-in lobby with one deck, without walking the whole flow.
fn seated_lobby() -> Lobby {
    let mut lobby = Lobby::new();
    lobby.set_field(Field::Username, "alice");
    lobby.set_field(Field::Password, "hunter22");
    assert!(lobby.submit().is_some());
    assert_eq!(
        lobby.apply(LobbyEvent::LoggedIn {
            token: "tok".to_string(),
            username: None
        }),
        Some(LobbyRequest::ListDecks)
    );
    assert_eq!(
        lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            sideboard: 0,
            id: "d1".to_string(),
            name: "Allytifact".to_string(),
            cards: 60,
            commanders: Vec::new(),
        }])),
        Some(LobbyRequest::ListGames(lobby.query()))
    );
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    lobby
}

/// The same lobby offline: no account, one deck, the tables listed.
fn offline_lobby() -> Lobby {
    let mut lobby = Lobby::new();
    assert_eq!(lobby.play_offline(), Some(LobbyRequest::ListDecks));
    assert_eq!(
        lobby.apply(LobbyEvent::Decks(vec![DeckSummary {
            sideboard: 0,
            id: "d1".to_string(),
            name: "Allytifact".to_string(),
            cards: 60,
            commanders: Vec::new(),
        }])),
        Some(LobbyRequest::ListGames(lobby.query()))
    );
    lobby.apply(LobbyEvent::Games(GameListing::default()));
    lobby
}

/// A page of `n` nameless tables, enough to page through.
fn a_page(n: usize) -> Vec<GameSummary> {
    (0..n)
        .map(|i| GameSummary {
            id: format!("g{i}"),
            state: "waiting".to_string(),
            ..GameSummary::default()
        })
        .collect()
}
