//! `cards/instants/mv_1/abjure.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, MERFOLK_OF_THE_PEARL_TRIDENT};

fn abjure() -> CardIndex {
    card_index("dd97e457-07eb-4918-b780-ae7ff42a37e7")
}

/// `Abjure` ({U}, Instant) sacrifices a blue permanent as an additional cost
/// to counter target spell.
#[test]
fn abjure_sacrifices_blue_permanent_to_counter_target_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[island(), MERFOLK_OF_THE_PEARL_TRIDENT])
        .hand(0, &[GRIZZLY_BEARS])
        .hand(1, &[abjure()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let merfolk = on_battlefield(&engine, p1, MERFOLK_OF_THE_PEARL_TRIDENT).unwrap();
    cast_from_hand(&mut engine, p0, GRIZZLY_BEARS);
    let spell_id = on_stack(&engine, GRIZZLY_BEARS).unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    cast_from_hand(&mut engine, p1, abjure());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![spell_id],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![merfolk],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, MERFOLK_OF_THE_PEARL_TRIDENT).is_some());
    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_none());
}
