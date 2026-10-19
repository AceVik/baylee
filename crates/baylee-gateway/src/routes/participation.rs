//! One running participation per account on this gateway, independent of sockets.

use crate::{ErrorBody, Json, StatusCode, err, lobby};

/// Check under the same lobby lock as the subsequent create/join/start mutation.
pub(super) fn account(
    lobby: &lobby::Lobby,
    account: &str,
    except: Option<&str>,
) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    if lobby.running().any(|game| {
        Some(game.id.as_str()) != except
            && game.seats.iter().any(|chair| {
                chair.account_id.as_deref() == Some(account) && !chair.participation_ended
            })
    }) {
        return Err(err(
            StatusCode::CONFLICT,
            "finish or concede your running game before joining another",
        ));
    }
    Ok(())
}

/// Recheck every participant, not only the host who requests the start.
pub(super) fn room(lobby: &lobby::Lobby, id: &str) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    if let Some(game) = lobby.games.get(id) {
        for chair in &game.seats {
            if let Some(player) = &chair.account_id {
                account(lobby, player, Some(id))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> lobby::Lobby {
        let mut lobby = lobby::Lobby::default();
        let mut game = lobby::LobbyGame::blank("running".into(), 0);
        let mut seat = lobby::LobbySeat::open(0);
        seat.account_id = Some("player".into());
        game.seats.push(seat);
        game.state = lobby::LobbyState::Playing;
        lobby.games.insert(game.id.clone(), game);
        lobby
    }

    #[test]
    fn disconnected_participation_blocks_other_games_but_not_its_own_recovery() {
        let lobby = fixture();
        assert!(account(&lobby, "player", None).is_err());
        assert!(account(&lobby, "player", Some("running")).is_ok());
        assert!(account(&lobby, "stranger", None).is_ok());
        assert!(
            account(&lobby::Lobby::default(), "player", None).is_ok(),
            "another gateway is independent"
        );
    }

    #[test]
    fn only_authoritative_participation_end_or_game_end_releases_the_player() {
        let mut lobby = fixture();
        lobby.games.get_mut("running").unwrap().seats[0].participation_ended = true;
        assert!(account(&lobby, "player", None).is_ok());
        let game = lobby.games.get_mut("running").unwrap();
        game.seats[0].participation_ended = false;
        game.state = lobby::LobbyState::Over;
        assert!(account(&lobby, "player", None).is_ok());
    }

    #[test]
    fn a_free_host_cannot_start_a_room_with_an_already_playing_guest() {
        let mut lobby = fixture();
        let mut queued = lobby::LobbyGame::blank("queued".into(), 0);
        let mut seat = lobby::LobbySeat::open(0);
        seat.account_id = Some("host".into());
        queued.seats.push(seat);
        let mut guest = lobby::LobbySeat::open(1);
        guest.account_id = Some("player".into());
        queued.seats.push(guest);
        lobby.games.insert(queued.id.clone(), queued);
        assert!(account(&lobby, "host", None).is_ok());
        assert!(room(&lobby, "queued").is_err());
        lobby.games.get_mut("running").unwrap().state = lobby::LobbyState::Waiting;
        assert!(
            room(&lobby, "queued").is_ok(),
            "multiple waiting rooms remain allowed"
        );
    }

    #[test]
    fn engine_reports_are_atomic_monotonic_and_do_not_follow_a_rematch() {
        let mut lobby = fixture();
        let game = lobby.games.get_mut("running").unwrap();
        let mut other = lobby::LobbySeat::open(1);
        other.account_id = Some("other".into());
        game.seats.push(other);
        assert!(!game.end_participation(&[0, u32::MAX]));
        assert!(game.seats.iter().all(|s| !s.participation_ended));
        assert!(
            !game.end_participation(&[0, 1, 0]),
            "oversized reports are refused whole"
        );
        assert!(game.end_participation(&[0]));
        assert!(!game.end_participation(&[]));
        assert!(!game.end_participation(&[0]));
        assert!(game.seats[0].participation_ended);
        assert!(!game.seats[1].participation_ended);
        let next = lobby::LobbyGame::rematch_of(game, "next".into(), 1);
        assert!(next.seats.iter().all(|s| !s.participation_ended));
        game.state = lobby::LobbyState::Waiting;
        assert!(!game.end_participation(&[1]));
        assert!(!game.seats[1].participation_ended);
    }
}
