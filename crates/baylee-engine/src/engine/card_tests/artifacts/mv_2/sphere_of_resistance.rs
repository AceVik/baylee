#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn sphere_of_resistance() -> CardIndex {
    card_index("09c96077-3804-4f12-a613-5bebc5e0413f")
}

fn spellbook() -> CardIndex {
    card_index("83133f51-bfbd-4db5-9be0-f660e3b66435")
}

#[test]
fn sphere_of_resistance_increases_spell_cost_by_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[sphere_of_resistance(), spellbook()])
        .battlefield(0, &[forest(), forest(), forest()])
        .battlefield(1, &[forest()])
        .hand(1, &[index::SPELLBOOK])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast Sphere of Resistance ({2}) and Spellbook ({0} + {1} tax = {1}) using all 3 Forests.
    cast_from_hand(&mut engine, p0, sphere_of_resistance());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, sphere_of_resistance()).is_some());

    cast_from_hand(&mut engine, p0, spellbook());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, spellbook()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, index::SPELLBOOK);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, index::SPELLBOOK).is_some());
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "opponent also pays the tax"
    );
}
