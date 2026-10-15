//! `cards/creatures/artifacts/mv_4/the_vision_and_scarlet_witch.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn the_vision_and_scarlet_witch() -> CardIndex {
    card_index("33219792-f0a7-4849-bfca-70ee2d410b9e")
}

/// The Vision and Scarlet Witch — {2}{R}{R} 3/3 with flying: "Whenever you
/// cast a spell, add {R} and put a +1/+1 counter on The Vision and Scarlet
/// Witch." A Sol Ring paid for with the Mountain's {R} leaves one red mana
/// floating and a counter on her (4/4); both Sol Ring and the trigger
/// resolved.
#[test]
fn the_vision_and_scarlet_witch_makes_red_and_a_counter_per_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[the_vision_and_scarlet_witch(), mountain()])
        .hand(0, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let witch = on_battlefield(&engine, p0, the_vision_and_scarlet_witch()).expect("seated");
    assert_eq!(pt(&engine, witch), (3, 3));
    assert!(keywords(&engine, witch).contains(KeywordSet::FLYING));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    cast_from_hand(&mut engine, p0, sol_ring());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, sol_ring()).is_some(),
        "Sol Ring"
    );
    assert_eq!(counters_on(&engine, witch, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, witch), (4, 4));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "the trigger's {{R}}");
    assert_eq!(
        pool.total(),
        1,
        "and the Mountain's {{R}} went to the spell"
    );
}
