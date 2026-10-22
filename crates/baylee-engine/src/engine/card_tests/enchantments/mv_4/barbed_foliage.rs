//! `cards/enchantments/mv_4/barbed_foliage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{AGILITY, BARBED_FOLIAGE, GRIZZLY_BEARS};

#[test]
fn barbed_foliage_strips_flanking_and_deals_one_damage_to_nonflyer_attacking_you() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[BARBED_FOLIAGE])
        .hand(1, &[AGILITY])
        .battlefield(1, &[GRIZZLY_BEARS, mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let attacker = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p1, AGILITY);
    aim_at(&mut engine, p1, attacker);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .object(attacker)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLANKING)
    );

    attack_with(&mut engine, attacker, p0);
    pass_until(&mut engine, stack_is_empty);

    let attacker_obj = engine.state().object(attacker).unwrap();
    assert!(
        !attacker_obj
            .characteristics()
            .keywords
            .contains(KeywordSet::FLANKING),
        "attacker should lose flanking until end of turn"
    );
    assert_eq!(
        attacker_obj.damage, 1,
        "Barbed Foliage should deal 1 damage to the attacking creature without flying"
    );
}
