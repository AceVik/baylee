//! Grim Giganotosaurus, played while it is also an artifact.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

#[test]
fn grim_giganotosaurus_counts_large_opposing_creatures_and_survives_its_sweep() {
    let dinosaur = card_index("1c197b3c-f540-4766-aeaa-ae09d22214cb");
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![dinosaur, liquimetal_coating(), forest()];
    board.extend([swamp(); 10]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, &[index::SERRA_ANGEL, index::HILL_GIANT, sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, me);

    let source = on_battlefield(&engine, me, dinosaur).unwrap();
    activate(&mut engine, me, liquimetal_coating(), 0);
    aim_at(&mut engine, me, source);
    pass_until(&mut engine, stack_is_empty);
    let types = engine
        .state()
        .object(source)
        .unwrap()
        .characteristics()
        .types;
    assert!(types.contains(TypeSet::ARTIFACT));
    assert!(types.contains(TypeSet::CREATURE));

    let last_swamp = on_battlefield(&engine, me, swamp()).unwrap();
    tap_mana_except(&mut engine, me, last_swamp);
    assert!(!priority_offer(&engine).abilities.contains(&(source, 0)));
    engine
        .apply(me, PlayerAction::ActivateManaAbility { source: last_swamp })
        .unwrap();
    assert!(priority_offer(&engine).abilities.contains(&(source, 0)));
    activate(&mut engine, me, dinosaur, 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(on_battlefield(&engine, me, dinosaur), Some(source));
    assert_eq!(counters_on(&engine, source, CounterKind::P1P1), 10);
    assert!(in_graveyard(&engine, me, liquimetal_coating()).is_some());
    assert!(in_graveyard(&engine, them, sol_ring()).is_some());
    assert!(in_graveyard(&engine, them, index::SERRA_ANGEL).is_some());
    assert!(in_graveyard(&engine, them, index::HILL_GIANT).is_some());
    assert!(on_battlefield(&engine, me, forest()).is_some());
}
