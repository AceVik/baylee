//! How a room is named, and how many one account may keep waiting.

use super::*;

fn waiting(lobby: &mut lobby::Lobby, id: &str, host: &str, state: LobbyState) {
    let mut game = LobbyGame::blank(id.into(), 0);
    game.host = Some(host.into());
    game.state = state;
    lobby.games.insert(id.into(), game);
}

#[test]
fn an_account_keeps_only_so_many_rooms_waiting() {
    let mut lobby = lobby::Lobby::default();
    for n in 0..MAX_WAITING_ROOMS - 1 {
        waiting(&mut lobby, &format!("w{n}"), "host", LobbyState::Waiting);
    }
    // What is played, over or another's counts for nobody here.
    waiting(&mut lobby, "p", "host", LobbyState::Playing);
    waiting(&mut lobby, "o", "host", LobbyState::Over);
    waiting(&mut lobby, "x", "other", LobbyState::Waiting);
    assert!(room_for_a_room(&lobby, "host").is_ok());
    waiting(&mut lobby, "last", "host", LobbyState::Waiting);
    let (status, _) = room_for_a_room(&lobby, "host").expect_err("full");
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert!(room_for_a_room(&lobby, "other").is_ok());
}

#[test]
fn a_listed_name_is_one_plain_line_of_at_most_sixty_characters() {
    assert_eq!(listed_name("Friday night"), "Friday night");
    assert_eq!(listed_name("a\nb\r\tc\0d\u{85}e"), "abcde");
    assert_eq!(listed_name("\u{202e}evil\u{2066}x\u{2069}"), "evilx");
    assert_eq!(listed_name(&"ä".repeat(61)), "ä".repeat(60));
    // What is dropped does not count against the length.
    let padded = format!("{}{}", "\u{202e}".repeat(100), "a".repeat(60));
    assert_eq!(listed_name(&padded), "a".repeat(60));
}
