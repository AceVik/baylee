//! The side value on real tables, against the engine's own questions.

use baylee_core::ids::PlayerId;
use baylee_core::types::TargetSide;
use baylee_engine::choice::{Pending, PlayerAction};

use super::TARGET_SIDE_SLOT;
use crate::encoder_contract::{Encoder, P0, P1, Seat, Table};
use crate::features4::EncodedCandidates4;

/// P0 with `spell` and enough Plains and Forests to cast it, over Grizzly Bears
/// against P1's Hill Giant.
fn table(spell: &str, seed: u64) -> Table {
    Table::new(
        seed,
        "Plains",
        [
            Seat {
                hand: std::slice::from_ref(&spell),
                battlefield: &[
                    "Plains",
                    "Plains",
                    "Plains",
                    "Forest",
                    "Forest",
                    "Forest",
                    "Swamp",
                    "Swamp",
                    "Mountain",
                    "Grizzly Bears",
                ],
            },
            Seat {
                hand: &[],
                battlefield: &["Hill Giant"],
            },
        ],
    )
}

/// Answers P0's mode question, if the cast asks one, with its `mode`th option.
fn cast(table: &mut Table, spell: &str, mode: Option<usize>) {
    table.reach_main(P0);
    table.tap_lands(P0);
    let card = table.in_hand(P0, spell);
    table
        .session
        .act(P0, PlayerAction::CastSpell { card })
        .expect("the spell is cast");
    for _ in 0..4 {
        match (table.session.pending(), mode) {
            (Pending::ChooseTargets { .. }, _) => return,
            (Pending::ChooseCastMode { .. }, Some(mode)) => {
                table
                    .session
                    .act(P0, PlayerAction::ChooseMode(mode))
                    .expect("the mode is offered");
            }
            (pending, _) => panic!("a target question, not {pending:?}"),
        }
    }
}

/// The slot's value on the candidate naming `name` (or `player`).
fn slot(
    table: &Table,
    candidates: &EncodedCandidates4,
    name: Option<&str>,
    player: Option<PlayerId>,
) -> f32 {
    let view = table.session.observe_view(P0);
    candidates
        .candidates
        .iter()
        .find(|candidate| match &candidate.action {
            Some(PlayerAction::ChooseTargets { objects, players }) => {
                let named = objects
                    .first()
                    .and_then(|id| view.object(*id))
                    .map(|o| o.name.as_str());
                named == name && players.first().copied() == player
            }
            _ => false,
        })
        .unwrap_or_else(|| panic!("{name:?} {player:?} is a candidate"))
        .feat[TARGET_SIDE_SLOT]
}

fn side_values(table: &Table) -> (Option<TargetSide>, EncodedCandidates4) {
    let (pending, view) = Encoder::V10.view_for(&table.session, P0);
    let side = view.targeting.as_ref().and_then(|t| t.side);
    (side, Encoder::V10.encode(&view, &pending, None).1)
}

fn reads_opponents_creature_as_right(spell: &str, seed: u64, side: TargetSide, right: f32) {
    let mut table = table(spell, seed);
    cast(&mut table, spell, None);
    let (read, candidates) = side_values(&table);
    assert_eq!(read, Some(side), "{spell}");
    assert_eq!(slot(&table, &candidates, Some("Hill Giant"), None), right);
    assert_eq!(
        slot(&table, &candidates, Some("Grizzly Bears"), None),
        -right
    );
}

#[test]
fn removal_wants_the_other_side_of_the_table() {
    reads_opponents_creature_as_right("Swords to Plowshares", 3, TargetSide::Harmful, 1.0);
}

#[test]
fn a_pump_wants_this_seats_own_side() {
    reads_opponents_creature_as_right("Giant Growth", 5, TargetSide::Helpful, -1.0);
}

#[test]
fn an_aura_reads_by_what_it_does_to_its_creature() {
    reads_opponents_creature_as_right("Rancor", 7, TargetSide::Helpful, -1.0);
    reads_opponents_creature_as_right("Weight of the Underworld", 9, TargetSide::Harmful, 1.0);
}

#[test]
fn a_player_stands_on_their_own_side() {
    let mut table = table("Lightning Bolt", 11);
    cast(&mut table, "Lightning Bolt", None);
    let (side, candidates) = side_values(&table);
    assert_eq!(side, Some(TargetSide::Harmful));
    assert_eq!(slot(&table, &candidates, None, Some(P1)), 1.0);
    assert_eq!(slot(&table, &candidates, None, Some(P0)), -1.0);
}

#[test]
fn a_modal_spell_reads_by_the_selected_mode() {
    let spell = "Selesnya Charm";
    let mut pump = table(spell, 13);
    cast(&mut pump, spell, Some(0));
    assert_eq!(side_values(&pump).0, Some(TargetSide::Helpful));
    let mut exile = Table::new(
        15,
        "Plains",
        [
            Seat {
                hand: &[spell],
                battlefield: &["Plains", "Plains", "Forest", "Forest"],
            },
            Seat {
                hand: &[],
                battlefield: &["Craw Wurm"],
            },
        ],
    );
    cast(&mut exile, spell, Some(1));
    assert_eq!(side_values(&exile).0, Some(TargetSide::Harmful));
}

#[test]
fn a_question_with_no_side_writes_nothing() {
    let mut table = table("Khalni Ambush", 17);
    cast(&mut table, "Khalni Ambush", Some(0));
    let (side, candidates) = side_values(&table);
    assert_eq!(side, None, "a fight wants both sides");
    for candidate in &candidates.candidates {
        assert_eq!(candidate.feat[TARGET_SIDE_SLOT], 0.0);
    }
}

#[test]
fn only_the_chooser_is_told() {
    let mut table = table("Swords to Plowshares", 19);
    cast(&mut table, "Swords to Plowshares", None);
    assert!(table.session.view_for_with_context(P1).is_none());
    assert!(
        table
            .session
            .observe_view_with_context(P1)
            .targeting
            .is_none()
    );
}

#[test]
fn every_other_encoder_reads_zero_and_otherwise_encoder_seven_is_unchanged() {
    let mut table = table("Swords to Plowshares", 21);
    cast(&mut table, "Swords to Plowshares", None);
    let (pending, view) = Encoder::V7.view_for(&table.session, P0);
    let (state7, seven) = Encoder::V7.encode(&view, &pending, None);
    let (state10, ten) = Encoder::V10.encode(&view, &pending, None);
    assert_eq!(
        serde_json::to_vec(&state7).expect("state"),
        serde_json::to_vec(&state10).expect("state"),
        "the state is Encoder 7's"
    );
    assert_eq!(seven.candidates.len(), ten.candidates.len());
    let mut read = 0;
    for (a, b) in seven.candidates.iter().zip(&ten.candidates) {
        assert_eq!(a.feat[TARGET_SIDE_SLOT], 0.0);
        read += usize::from(b.feat[TARGET_SIDE_SLOT] != 0.0);
        let mut same = b.feat.clone();
        same[TARGET_SIDE_SLOT] = 0.0;
        assert_eq!(a.feat, same);
    }
    assert!(read > 0);
    for encoder in [Encoder::V4, Encoder::V8, Encoder::V9] {
        let (pending, view) = encoder.view_for(&table.session, P0);
        for candidate in encoder.encode(&view, &pending, None).1.candidates {
            assert_eq!(candidate.feat[TARGET_SIDE_SLOT], 0.0, "{encoder:?}");
        }
    }
}

#[test]
fn the_slot_reads_the_same_off_the_wire() {
    let mut table = table("Swords to Plowshares", 23);
    cast(&mut table, "Swords to Plowshares", None);
    let (pending, view) = Encoder::V10.view_for(&table.session, P0);
    let wire: baylee_view::PlayerView =
        serde_json::from_slice(&serde_json::to_vec(&view).expect("a view serializes"))
            .expect("a view parses");
    let here = Encoder::V10.encode(&view, &pending, None).1;
    let there = Encoder::V10.encode(&wire, &pending, None).1;
    assert_eq!(here.candidates.len(), there.candidates.len());
    for (a, b) in here.candidates.iter().zip(&there.candidates) {
        assert_eq!(a.feat, b.feat);
    }
    assert!(
        there
            .candidates
            .iter()
            .any(|c| c.feat[TARGET_SIDE_SLOT] != 0.0)
    );
}
