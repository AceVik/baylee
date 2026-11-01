//! Devouring Strossus — {5}{B}{B}{B} — Creature — Phyrexian Horror
//! Oracle: Flying, trample
//! Oracle: At the beginning of your upkeep, sacrifice a creature.
//! Oracle: Sacrifice a creature: Regenerate this creature.

use super::*;
use baylee_core::generated::index;

#[test]
fn devouring_strossus_sacrifices_creature_on_upkeep_and_to_regenerate() {
    let strossus = card_index("8f2ab0a8-9d60-493b-a284-9ed7fa07e5ea");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                grizzly_bears(),
                index::HILL_GIANT,
            ],
        )
        .hand(0, &[strossus, index::HERO_S_DOWNFALL])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, strossus);
    pass_until(&mut engine, stack_is_empty);

    let strossus_id =
        on_battlefield(&engine, p0, strossus).expect("Devouring Strossus on battlefield");
    let bears_id = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant_id = on_battlefield(&engine, p0, index::HILL_GIANT).unwrap();
    assert_eq!(pt(&engine, strossus_id), (9, 9));
    assert!(keywords(&engine, strossus_id).contains(KeywordSet::FLYING.union(KeywordSet::TRAMPLE)));

    // Activate "Sacrifice a creature: Regenerate this creature" sacrificing Grizzly Bears.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: strossus_id,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());

    // Destroy Devouring Strossus with Hero's Downfall: regeneration shield saves it.
    crate::engine::card_tests::legends_kit::float(&mut engine, p0, &[(ManaColor::Black, 3)]);
    cast_with_floating(&mut engine, p0, index::HERO_S_DOWNFALL);
    aim_at(&mut engine, p0, strossus_id);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, strossus).is_some());

    // Advance to seat 0's next upkeep: upkeep trigger asks which creature to sacrifice.
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, index::HILL_GIANT).is_some());
}
