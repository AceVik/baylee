//! `cards/instants/mv_3/force_void.rs`, played.
//!
//! Oracle:
//! - "Counter target spell unless its controller pays {1}."
//! - "Draw a card at the beginning of the next turn's upkeep."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn force_void() -> CardIndex {
    card_index("469bdca3-d373-4420-9266-921b6605f327")
}

#[test]
fn force_void_counters_spell_unless_paid_and_draws_at_next_turns_upkeep() {
    let mut engine = lk::game(SEED, &[], &[], &[index::GRIZZLY_BEARS], &[force_void()]);

    lk::float(&mut engine, P0, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut engine, P0, index::GRIZZLY_BEARS);
    let spell = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .first()
        .unwrap();

    lk::pass_once(&mut engine);
    lk::float(
        &mut engine,
        P1,
        &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
    );
    cast_with_floating(&mut engine, P1, force_void());
    lk::aim(&mut engine, &[spell], &[]);
    for _ in 0..20 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            other => panic!("unexpected pending {other:?}"),
        }
    }

    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some());

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P1)).len();
    lk::to_step(&mut engine, P1, Step::Upkeep);
    lk::settle(&mut engine);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        hand_before + 1
    );
}
