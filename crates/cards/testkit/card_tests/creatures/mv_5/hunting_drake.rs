//! `cards/creatures/mv_5/hunting_drake.rs`, played.
//!
//! Oracle:
//! - "Flying"
//! - "When this creature enters, put target red or green creature on top of its
//!   owner's library."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn hunting_drake() -> CardIndex {
    card_index("d5978943-52fc-4b57-9c1c-d802ade1b014")
}

#[test]
fn hunting_drake_puts_target_red_or_green_creature_on_top_of_library() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                index::LLANOWAR_ELVES,
            ],
        )
        .battlefield(
            1,
            &[
                index::HILL_GIANT,
                index::PHANTOM_MONSTER,
                index::SAVANNAH_LIONS,
            ],
        )
        .hand(0, &[hunting_drake()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    let elves = on_battlefield(&engine, P0, index::LLANOWAR_ELVES).unwrap();
    let monster = on_battlefield(&engine, P1, index::PHANTOM_MONSTER).unwrap();
    let lions = on_battlefield(&engine, P1, index::SAVANNAH_LIONS).unwrap();
    let p1_lib_before = library_size(&engine, P1);

    cast_from_hand(&mut engine, P0, hunting_drake());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("stopped on ChooseTargets");
    };
    assert!(options.contains(&giant), "red creature is legal target");
    assert!(options.contains(&elves), "green creature is legal target");
    assert!(!options.contains(&monster), "blue creature is not legal");
    assert!(!options.contains(&lions), "white creature is not legal");

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let drake = on_battlefield(&engine, P0, hunting_drake()).unwrap();
    assert_eq!(pt(&engine, drake), (2, 2));
    assert!(keywords(&engine, drake).contains(KeywordSet::FLYING));
    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_none());
    assert_eq!(library_size(&engine, P1), p1_lib_before + 1);
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(P1))
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(top).unwrap().card.unwrap().index,
        index::HILL_GIANT
    );
}

#[test]
fn hunting_drake_targets_own_green_creature_and_handles_no_legal_targets() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                index::LLANOWAR_ELVES,
            ],
        )
        .hand(0, &[hunting_drake(), hunting_drake()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let elves = on_battlefield(&engine, P0, index::LLANOWAR_ELVES).unwrap();
    let p0_lib_before = library_size(&engine, P0);
    cast_from_hand(&mut engine, P0, hunting_drake());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, P0, elves);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, index::LLANOWAR_ELVES).is_none());
    assert_eq!(library_size(&engine, P0), p0_lib_before + 1);
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(P0))
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(top).unwrap().card.unwrap().index,
        index::LLANOWAR_ELVES
    );

    // With no red or green creatures left, a second Drake resolves without crashing.
    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));
    cast_from_hand(&mut engine, P0, hunting_drake());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(all_on_battlefield(&engine, P0, hunting_drake()).len(), 2);
}
