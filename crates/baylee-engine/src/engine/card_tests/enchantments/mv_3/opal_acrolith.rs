#[allow(clippy::wildcard_imports)]
use super::*;

fn opal_acrolith() -> CardIndex {
    card_index("d08b8dd6-3a44-4e86-8ce8-2d0a014a9aa8")
}

#[test]
fn opal_acrolith_partial_becomes_2_4_soldier_on_opponent_creature_cast() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[opal_acrolith()])
        .hand(1, &[grizzly_bears()])
        .battlefield(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1));

    cast_from_hand(&mut engine, p1, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);

    let perm = on_battlefield(&engine, p0, opal_acrolith()).expect("seated");
    assert_eq!(pt(&engine, perm), (2, 4));
}

/// The trigger excludes your creature spells, opposing noncreature spells,
/// and further creature spells once the Opal has become a creature.
#[test]
fn opal_acrolith_respects_opponent_creature_and_enchantment_conditions() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[opal_acrolith()],
        &[],
        &[grizzly_bears()],
        &[giant_growth(), grizzly_bears(), grizzly_bears()],
    );
    let opal = lk::obj(&e, p0, opal_acrolith());
    lk::float(&mut e, p0, &[(ManaColor::Green, 2)]);
    lk::cast_at(&mut e, p0, grizzly_bears(), &[], &[]);
    assert!(
        e.state()
            .object(opal)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::Green, 5)]);
    let bear = lk::obj(&e, p0, grizzly_bears());
    lk::cast_at(&mut e, p1, giant_growth(), &[bear], &[]);
    assert!(
        e.state()
            .object(opal)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
    lk::cast_at(&mut e, p1, grizzly_bears(), &[], &[]);
    let chars = e.state().object(opal).unwrap().characteristics();
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(!chars.types.contains(TypeSet::ENCHANTMENT));
    assert_eq!(pt(&e, opal), (2, 4));
    cast_with_floating(&mut e, p1, grizzly_bears());
    assert_eq!(
        e.state().zones.list(crate::zone::ZoneLocation::Stack).len(),
        1,
        "intervening if prevents another trigger"
    );
    lk::settle(&mut e);
}
