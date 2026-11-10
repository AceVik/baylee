//! A team game is one game, however many of its seats eval4 writes a row for:
//! the rows of the net's team share a game key and, at temperature 0, replay
//! one game (`baylee_train_ops::evaluation::games`). Engine-backed through
//! the evaluator's own `play_one_table`, with the lookahead tests' stand-in
//! for the net: NOT evidence about playing strength.
use crate::lookahead::tests::{Fixture, flat};
use crate::{is_net_seat, play_one_table};
use baylee_core::preset::AIProfile;
use baylee_train::housedeck::HouseDeck;
use baylee_train_ops::evaluation::games::{game_key, side};

/// The 2v2 table 0 dealt as eval4's loop deals it with `focal` as its focal
/// seat (deal `focal`): the stand-in on the focal seat's team, the house on
/// the other. Its result for the net's side, and the row's game key.
fn deal(focal: usize) -> (f32, String) {
    let deck = HouseDeck::named("victory").unwrap();
    let teams_u8 = [1, 2, 1, 2];
    let teams = teams_u8.map(Some);
    let profiles: Vec<AIProfile> = (0..4)
        .map(|seat| {
            if is_net_seat(seat, focal, &teams) {
                AIProfile::STEADY
            } else {
                AIProfile::SHARP
            }
        })
        .collect();
    let mut player = Fixture::new(false, flat);
    let outcome = play_one_table(
        "team-game-test",
        focal as u64,
        focal,
        12,
        34,
        &[&deck; 4],
        &profiles,
        &teams_u8,
        &teams,
        Some(&mut player),
        true,
        false,
        // eval4 gives a table 5000 decisions per two seats.
        10_000,
        false,
        None,
        None,
        None,
        crate::DecisionLogs::default(),
        None,
    )
    .unwrap();
    assert!(
        !outcome.1,
        "focal {focal}: the deal ends before the answer cap ({} answers)",
        outcome.4
    );
    (outcome.0, game_key(0, &side(focal, &teams)))
}

#[test]
fn the_seats_of_one_team_write_one_game() {
    let (first, second, other) = (deal(0), deal(2), deal(1));
    assert_eq!(first.1, "0/team1");
    assert_eq!(second.1, first.1, "seats 0 and 2 are one game's rows");
    assert_eq!(second.0.to_bits(), first.0.to_bits(), "and replay it");
    assert_eq!(other.1, "0/team2", "seat 1 is the other side's game");
}
