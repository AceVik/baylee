//! `cards/creatures/mv_2/dracoplasm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn dracoplasm() -> CardIndex {
    card_index("3155c833-8170-4166-8f47-c3c1fad1575a")
}

/// Dracoplasm (`3155c833-8170-4166-8f47-c3c1fad1575a`) — {U}{R} Creature — Shapeshifter (Partial):
/// "Flying. As this creature enters, sacrifice any number of creatures. ... {R}: This creature gets +1/+0 until end of turn."
#[test]
fn dracoplasm_sacrifices_creatures_on_entry_and_pumps_for_red() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::GRIZZLY_BEARS,
                llanowar_elves(),
            ],
        )
        .hand(0, &[dracoplasm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let elves = on_battlefield(&engine, P0, llanowar_elves()).unwrap();

    cast_from_hand(&mut engine, P0, dracoplasm());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![bears],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    let draco = on_battlefield(&engine, P0, dracoplasm()).unwrap();
    assert_eq!(pt(&engine, draco), (1, 1));
    let _ = elves;

    activate(&mut engine, P0, dracoplasm(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, draco), (2, 1));
}
