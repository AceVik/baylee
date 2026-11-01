//! `cards/enchantments/mv_4/barbed_foliage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{AGILITY, GRIZZLY_BEARS, WIND_DRAKE};

fn barbed_foliage() -> CardIndex {
    card_index("f9275c99-0616-44b3-a19e-98c683e87ad3")
}

#[test]
fn barbed_foliage_strips_flanking_and_deals_one_damage_to_nonflyer_attacking_you() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[barbed_foliage()])
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
    // "Until end of turn": the Aura's flanking is back on p0's turn.
    reach_their_main_phase(&mut engine, p0);
    assert!(keywords(&engine, attacker).contains(KeywordSet::FLANKING));
}

#[test]
fn barbed_foliage_strips_flanking_but_does_not_burn_a_flyer_attacking_you() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[barbed_foliage()])
        .hand(1, &[AGILITY])
        .battlefield(1, &[WIND_DRAKE, mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let drake = on_battlefield(&engine, p1, WIND_DRAKE).unwrap();
    cast_from_hand(&mut engine, p1, AGILITY);
    aim_at(&mut engine, p1, drake);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, drake).contains(KeywordSet::FLANKING));
    attack_with(&mut engine, drake, p0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !keywords(&engine, drake).contains(KeywordSet::FLANKING),
        "every attacker loses flanking, flyers included"
    );
    assert_eq!(
        engine.state().object(drake).unwrap().damage,
        0,
        "the 1 damage is for creatures without flying"
    );
}
