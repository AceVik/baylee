//! `cards/creatures/mv_3/drake_hatchling.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Drake Hatchling is a {2}{U} 1/3 Drake with flying whose only other text is
/// "{U}: This creature gets +1/+0 until end of turn. Activate only once each
/// turn." Five Islands are tapped in one go: three pay for the Drake and the
/// two left floating are what the pump draws on, so the one blue still in the
/// pool after the pump is the control that makes the ability's disappearance a
/// statement about the printed once-a-turn limit rather than about a cost the
/// board could not pay — `legal.abilities` is filtered through `can_afford`,
/// which reads the mana pool and not the untapped lands. The whole scenario
/// stays in this one main phase, so CR 500.5 never empties what is left over.
#[test]
fn drake_hatchling_pumps_once_a_turn_off_a_pool_that_could_still_pay_again() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[drake_hatchling()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Islands, and the Drake is not on the table yet"
    );
    cast_with_floating(&mut engine, p0, drake_hatchling());
    pass_until(&mut engine, stack_is_empty);
    let drake = on_battlefield(&engine, p0, drake_hatchling()).expect("the Drake resolved");
    assert_eq!(pt(&engine, drake), (1, 3), "the body the card prints");
    assert!(
        keywords(&engine, drake).contains(KeywordSet::FLYING),
        "the printed flying line reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "five Islands less the {{2}}{{U}} the Drake cost"
    );

    // Ability 0 is the only line the card prints, and the offer is read off
    // the mana already floating rather than off the untapped lands.
    activate(&mut engine, p0, drake_hatchling(), 0);
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so it uses the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, drake), (2, 3), "+1/+0 until end of turn");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{U}} came out of the pool"
    );

    // The second activation is the claim. One blue is still floating, so
    // `can_afford` has everything it needs: the ability's absence is the
    // printed "activate only once each turn" and nothing else.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == drake),
        "`{{U}}`: +1/+0 has been taken for this turn, so it is not offered \
         again even with the mana to pay it: {:?}",
        legal.abilities
    );
}
