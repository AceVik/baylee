//! What the client says it is doing, for Baylee Central's friends
//! (`docs/client.md` §"The contact status").
//!
//! Only two things can be said, and neither names a game, a table or a
//! gateway: sitting in a game that is running, and sitting at a table whose
//! game has not begun. Watching a game, a replay, the deck builder and a
//! result screen are none of them: online.

use central_api::presence::Activity;

use crate::lobby::Screen;

/// What the client is doing: `screen` is the lobby's, `finished` whether
/// the duel on screen has ended (its result is shown), and `waiting` whether
/// this player has a chair at a table still waiting to start.
///
/// A game counts from the moment its seat is handed over, online or against
/// the house AI in this process, until its result stands.
#[must_use]
pub fn doing(screen: &Screen, finished: bool, waiting: bool) -> Option<Activity> {
    match screen {
        Screen::Seated(seat) if !seat.watching() && !finished => Some(Activity::Playing),
        Screen::Seated(_) | Screen::Replay(_) => None,
        Screen::SignIn { .. } | Screen::Table | Screen::Build => waiting.then_some(Activity::Table),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lobby::SeatHandover;

    fn seat(seat: u32, local: bool) -> Screen {
        Screen::Seated(SeatHandover {
            game_id: "g".into(),
            seat,
            seat_token: String::new(),
            local,
        })
    }

    #[test]
    fn a_seat_in_a_running_game_plays_online_or_against_the_house() {
        for local in [false, true] {
            assert_eq!(
                doing(&seat(0, local), false, false),
                Some(Activity::Playing)
            );
            // The chair in the listing may still read as waiting while the
            // game opens: the game wins.
            assert_eq!(doing(&seat(1, local), false, true), Some(Activity::Playing));
        }
    }

    #[test]
    fn a_result_a_watched_game_and_a_replay_are_neither() {
        assert_eq!(doing(&seat(0, false), true, false), None);
        assert_eq!(doing(&seat(SeatHandover::WATCH, false), false, false), None);
        assert_eq!(doing(&seat(SeatHandover::WATCH, false), false, true), None);
    }

    #[test]
    fn a_chair_at_a_waiting_table_is_at_a_table_wherever_the_lobby_stands() {
        for screen in [Screen::Table, Screen::Build] {
            assert_eq!(doing(&screen, false, true), Some(Activity::Table));
            assert_eq!(doing(&screen, false, false), None);
        }
    }
}
