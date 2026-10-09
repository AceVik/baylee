//! `cards/creatures/mv_1/will_o_the_wisp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Will-o'-the-Wisp — Flying; `{B}`: Regenerate this creature.
#[test]
fn will_o_the_wisp_flies_and_regenerates_for_b() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[will_o_the_wisp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wisp = on_battlefield(&engine, p0, will_o_the_wisp()).expect("seated");
    assert_eq!(pt(&engine, wisp), (0, 1));
    assert!(keywords(&engine, wisp).contains(KeywordSet::FLYING));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, will_o_the_wisp(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(wisp).unwrap().regeneration_shields, 1);
}

/// The shield is used up. Bought for {B}, it saves the 0/1 Wisp from a first
/// Lightning Bolt (tapped, damage healed, shield gone), and a second Bolt in
/// the same turn finds no shield and kills it.
#[test]
fn will_o_the_wisp_regeneration_saves_it_once_and_is_then_spent() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[will_o_the_wisp(), swamp(), mountain(), mountain()])
        .hand(0, &[lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wisp = on_battlefield(&engine, p0, will_o_the_wisp()).expect("seated");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, will_o_the_wisp(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(wisp).unwrap().regeneration_shields, 1);

    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, wisp);
    pass_until(&mut engine, stack_is_empty);
    let survivor = engine.state().object(wisp).expect("regenerated, not dead");
    assert_eq!(survivor.zone, Zone::Battlefield, "the shield saved it");
    assert_eq!(survivor.regeneration_shields, 0, "and is used up");
    assert_eq!(survivor.damage, 0, "regeneration heals all damage");
    assert!(is_tapped(&engine, wisp), "and taps the creature");

    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, wisp);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, will_o_the_wisp()).is_none(),
        "no shield left: the second Bolt kills it"
    );
    assert!(in_graveyard(&engine, p0, will_o_the_wisp()).is_some());
}
