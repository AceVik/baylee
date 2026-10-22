//! `cards/enchantments/mv_3/teferi_s_realm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn teferi_s_realm() -> CardIndex {
    card_index("9851d934-2e07-49c8-b08b-15f96d0f3f0c")
}

/// `Teferi's Realm` ({1}{U}{U}, World Enchantment, `Coverage::Partial`)
/// resolves onto the battlefield as a world enchantment.
#[test]
fn teferi_s_realm_resolves_as_world_enchantment() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[teferi_s_realm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, teferi_s_realm());
    pass_until(&mut engine, stack_is_empty);

    let realm = on_battlefield(&engine, p0, teferi_s_realm()).unwrap();
    assert!(
        engine
            .state()
            .object(realm)
            .unwrap()
            .characteristics()
            .supertypes
            .contains(SupertypeSet::WORLD)
    );
}
