//! `cards/creatures/mv_7/windbrisk_raptor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn windbrisk_raptor() -> CardIndex {
    card_index("1fd729f6-27ea-4b07-81f0-885f2c0c00eb")
}

/// Windbrisk Raptor — a 5/7 flier for {5}{W}{W}.
#[test]
fn windbrisk_raptor_costs_seven_and_is_a_five_seven_flier() {
    let mut engine = table7(&lots(plains(), 7), &[windbrisk_raptor()], &[]);
    let raptor = cast_and_resolve(&mut engine, windbrisk_raptor());
    assert_body(&engine, raptor, (5, 7), KeywordSet::FLYING);
}

/// "Attacking creatures you control have lifelink": not before the attack,
/// and once the Raptor and an Ogre attack, 5 + 2 damage is 7 life.
#[test]
fn windbrisk_raptor_gives_attackers_lifelink() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&[windbrisk_raptor(), gray_ogre()], &[], &[]);
    let raptor = on_battlefield(&engine, p0, windbrisk_raptor()).expect("out");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("out");
    assert!(!keywords(&engine, ogre).contains(KeywordSet::LIFELINK));
    declare_attack(&mut engine, &[raptor, ogre], &[]);
    assert!(keywords(&engine, ogre).contains(KeywordSet::LIFELINK));
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(life_of(&engine, 1), 13, "7 damage");
    assert_eq!(life_of(&engine, 0), 27, "and 7 life");
}
