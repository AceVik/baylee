//! `cards/creatures/mv_3/stronghold_biologist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn stronghold_biologist_counters_target_creature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                card_index("1e2c4508-54ad-45c2-b35f-678f8c376b27"),
            ],
        )
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, index::GRIZZLY_BEARS);

    pass_until(&mut engine, |e| {
        on_stack(e, index::GRIZZLY_BEARS).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let bears_spell = on_stack(&engine, index::GRIZZLY_BEARS).unwrap();
    let biologist = on_battlefield(
        &engine,
        p0,
        card_index("1e2c4508-54ad-45c2-b35f-678f8c376b27"),
    )
    .unwrap();

    tap_all_mana_but(
        &mut engine,
        p0,
        Some(card_index("1e2c4508-54ad-45c2-b35f-678f8c376b27")),
    );
    activate(
        &mut engine,
        p0,
        card_index("1e2c4508-54ad-45c2-b35f-678f8c376b27"),
        0,
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears_spell],
            },
        )
        .unwrap();

    let fodder = in_hand(&engine, p0, llanowar_elves()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();

    assert!(is_tapped(&engine, biologist));
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
}
