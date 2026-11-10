//! The house does not retry an action the engine took back
//! (`baylee_ai`'s `stall`, `docs/house-ai.md` §"The frozen reference").
//!
//! `stall/purge-x0.jsonl` is the start of a capped game of the campaign's
//! test pairings (10.11.2026): the house in a seat's chair, answering with
//! no note of its cast, tapped three lands for Kaervek's Purge ("destroy
//! target creature with mana value X") and announced X 0. No creature had
//! mana value 0, so the cast was taken back, and the house cast it again
//! from the same position 2,450 times until the cap ended the game. The
//! record is cut where the house holds priority with the mana up.
//!
//! The note now explains the cast to a seat (`ai_exact_x`), but whatever
//! takes an action back, the house must not give it again from the same
//! position: here it is asked as that seat was, and at every level, the
//! frozen reference included.

use baylee_ai::{AIProfile, HeuristicAgent};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_gamehost::{RegistryLookup, SeatContext, owed_payment, player_view, view};

const HOUSE: PlayerId = PlayerId::new(0);

/// The engine where the record ends: the house to act, the Purge in hand
/// and three mana floating.
fn position() -> Engine<RegistryLookup> {
    let record = include_bytes!("stall/purge-x0.jsonl");
    baylee_gamehost::record::replay(record)
        .expect("the record replays on this build")
        .engine
}

/// `seat`'s view of the question it is asked, as a seat is sent it.
fn seen(engine: &Engine<RegistryLookup>, seat: PlayerId, seq: u64) -> baylee_view::PlayerView {
    let pending = engine.pending();
    player_view(
        engine.state(),
        seat,
        seq,
        Some(pending),
        &SeatContext {
            awaiting: view::awaiting_for(engine, seat),
            decision_player: pending.asked(),
            controlled_players: engine.controlled_players(seat),
            deciding: view::deciding(engine),
            owed: owed_payment(engine),
            inspected_cards: engine.inspected_cards(),
            ..Default::default()
        },
        &[],
    )
}

#[test]
fn a_cast_the_engine_took_back_is_not_cast_again_from_the_same_position() {
    for level in [
        AIProfile::SHARP_2026_11,
        AIProfile::STEADY,
        AIProfile::SHARP,
        AIProfile::EXPERT,
    ] {
        let mut engine = position();
        let start = engine.snapshot_hash();
        let house = HeuristicAgent::new(level);
        let other = HeuristicAgent::new(level);
        let mut casts = 0;
        let mut taken_back = 0;
        let mut passed = false;
        for seq in 0..200 {
            if matches!(engine.pending(), Pending::GameOver(_)) {
                break;
            }
            let actor = engine.decision_actor().expect("somebody is asked");
            let pending = engine.pending().clone();
            let view = seen(&engine, actor, seq);
            // The seat as it was: the question with no note of the cast.
            let action = if actor == HOUSE {
                house.act(&view, &pending)
            } else {
                other.act_with_context(&view, &pending, &engine.decision_context())
            };
            if actor == HOUSE && matches!(pending, Pending::Priority { .. }) {
                match action {
                    PlayerAction::CastSpell { .. } => casts += 1,
                    PlayerAction::PassPriority => {
                        passed = true;
                        break;
                    }
                    _ => {}
                }
            }
            engine
                .apply(actor, action.clone())
                .unwrap_or_else(|err| panic!("{action:?} for {pending:?} was refused: {err}"));
            taken_back += u32::from(engine.snapshot_hash() == start);
        }
        assert!(
            passed,
            "{level:?}: the house never passed ({casts} casts, {taken_back} taken back)"
        );
        assert!(
            casts <= 1 && taken_back <= 1,
            "{level:?}: the house cast {casts} times and was taken back {taken_back} times"
        );
    }
}
