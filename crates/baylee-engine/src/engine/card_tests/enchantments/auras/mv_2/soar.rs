//! `cards/enchantments/auras/mv_2/soar.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn soar() -> CardIndex {
    card_index("acee6f4f-8b66-495d-bb0e-71c339e7dc49")
}

/// Soar (`{1}{U}`, `Coverage::Partial`): flash Aura that enchants a creature
/// and gives it +0/+1 and flying.
#[test]
fn soar_enchants_creature_and_grants_0_1_and_flying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), grizzly_bears()])
        .hand(0, &[soar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, soar());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (2, 3));
    assert!(keywords(&engine, bear).contains(KeywordSet::FLYING));
}
