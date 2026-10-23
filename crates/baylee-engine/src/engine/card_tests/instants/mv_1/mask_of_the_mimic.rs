//! `cards/instants/mv_1/mask_of_the_mimic.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mask_of_the_mimic() -> CardIndex {
    card_index("f1a16f6d-ea11-485e-8c5b-7f919a731fe0")
}

/// Mask of the Mimic (`f1a16f6d-ea11-485e-8c5b-7f919a731fe0`) — {U} Instant (Partial):
/// "As an additional cost to cast this spell, sacrifice a creature.
/// Search your library for a card with the same name as target nontoken creature, put that card onto the battlefield, then shuffle."
#[test]
fn mask_of_the_mimic_sacrifices_creature_and_searches_library_for_creature_to_battlefield() {
    let mut engine = Duel::new(SEED, index::GRIZZLY_BEARS)
        .battlefield(0, &[island(), index::LLANOWAR_ELVES])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[mask_of_the_mimic()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let target_bear = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    let fodder = on_battlefield(&engine, P0, index::LLANOWAR_ELVES).unwrap();

    cast_from_hand(&mut engine, P0, mask_of_the_mimic());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, P0, target_bear);
    }
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![fodder],
                },
            )
            .unwrap();
    }
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, P0, target_bear);
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, index::LLANOWAR_ELVES).is_none());
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
}
