//! The house plays as a team at a real table (owner, 10.11.2026): two
//! against two on the house decks, every level, played to the end.
//!
//! What a table of teams must never see: an attack at an ally or at an
//! ally's planeswalker, or a house chair shown an opponent's hand. What it
//! must see: house teammates shown each other's hands where their level
//! plays as a team, and the frozen reference shown none.

use baylee_cards::decks::{house_deck, preset_for_all};
use baylee_core::ids::{Defender, PlayerId};
use baylee_core::preset::{AIProfile, GamePreset, SeatController};
use baylee_engine::choice::Pending;
use baylee_gamehost::Session;
use baylee_gamehost::record::{Line, Source};

/// Seats 0 and 2 on one side, 1 and 3 on the other, as eval4's `1,2,1,2`.
const TEAMS: [u8; 4] = [1, 2, 1, 2];

fn table(profile: AIProfile, seed: u64) -> GamePreset {
    let decks: Vec<_> = ["allytifact", "victory", "maik", "weltenbaum"]
        .iter()
        .map(|name| {
            let path = format!("{}/../../data/decks/{name}.txt", env!("CARGO_MANIFEST_DIR"));
            let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
            house_deck(&text).unwrap_or_else(|e| panic!("{name}: {e}"))
        })
        .collect();
    let refs: Vec<_> = decks.iter().collect();
    let mut preset = preset_for_all(seed, &refs);
    for (spec, team) in preset.seats.iter_mut().zip(TEAMS) {
        spec.controller = SeatController::Ai(profile);
        spec.team = Some(team);
    }
    preset
}

fn ally(a: u8, b: u8) -> bool {
    TEAMS[usize::from(a)] == TEAMS[usize::from(b)]
}

/// Plays `preset` to its end, or to `cap` answers, checking every question
/// a house chair is asked on the way, and returns how many answers the
/// house gave and how many of its views showed a teammate's hand.
fn play(name: &str, preset: &GamePreset, cap: usize) -> (usize, usize) {
    let mut session = Session::new_recorded(preset, "team-play").expect("the table starts");
    session.describe("team-play".into(), (0..4).map(|s| s.to_string()).collect());
    session.discard_frames();
    let mut shown = 0;
    let mut answered = 0;
    while !matches!(session.pending(), Pending::GameOver(_)) && answered < cap {
        for seat in (0..4).map(PlayerId::new) {
            let Some((pending, view)) = session.view_for(seat) else {
                continue;
            };
            for hand in &view.shared_hands {
                assert!(
                    ally(hand.player.get(), seat.get()) || hand.public,
                    "{name}: seat {seat:?} was shown opponent {:?}'s hand",
                    hand.player
                );
            }
            shown += usize::from(!view.shared_hands.is_empty());
            if let Pending::ChooseAttackers { defenders, .. } = &pending {
                for defender in defenders {
                    let attacked = match defender {
                        Defender::Player(p) => Some(*p),
                        Defender::Planeswalker(w) => {
                            session.state().object(*w).map(|o| o.controller)
                        }
                        Defender::Battle(_) => None,
                    };
                    assert!(
                        !attacked.is_some_and(|p| ally(p.get(), seat.get())),
                        "{name}: seat {seat:?} was offered an attack at its own side"
                    );
                }
            }
        }
        let _ = session.pump_at_most(1);
        answered += 1;
    }
    let record = String::from_utf8(session.take_record()).expect("a record is text");
    let house = record
        .lines()
        .filter(|line| {
            matches!(
                serde_json::from_str::<Line>(line),
                Ok(Line::Input {
                    by: Source::House,
                    ..
                })
            )
        })
        .count();
    (house, shown)
}

/// No level is ever offered an attack at its own side, so none makes one
/// (the engine offers opponents and their planeswalkers alone, and every
/// answer is held to its offer); no house chair is shown an opponent's
/// hand; house teammates at a level that plays as a team are shown each
/// other's, and the frozen reference is shown none.
#[test]
fn no_level_attacks_its_side_and_house_teammates_see_each_others_hands() {
    for (name, profile) in AIProfile::NAMED.iter().chain(&AIProfile::REFERENCES) {
        let (answers, shown) = play(name, &table(*profile, 7), 1_500);
        assert!(answers > 100, "{name}: the table asked little");
        if profile.plays(AIProfile::TEAM_PLAY) {
            assert!(
                shown > 0,
                "{name}: no house chair was shown its teammate's hand"
            );
        } else {
            assert_eq!(shown, 0, "{name}: the frozen reference was shown a hand");
        }
    }
}
