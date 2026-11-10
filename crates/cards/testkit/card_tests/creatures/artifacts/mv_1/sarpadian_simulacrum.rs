//! `cards/creatures/artifacts/mv_1/sarpadian_simulacrum.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sarpadian_simulacrum() -> CardIndex {
    card_index("b563a578-86ce-41a5-a83e-c70c6a4960e2")
}

/// "Haste": cast this turn, it is still offered as an attacker.
#[test]
fn sarpadian_simulacrum_attacks_the_turn_it_is_cast() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[sarpadian_simulacrum()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, sarpadian_simulacrum());
    pass_until(&mut engine, stack_is_empty);
    let me = on_battlefield(&engine, p0, sarpadian_simulacrum()).expect("resolved");
    assert!(keywords(&engine, me).contains(KeywordSet::HASTE));
    assert!(attackers_offered(&mut engine).contains(&me));
}

/// "{3}{R}, Sacrifice this creature: It deals 4 damage to target creature."
/// The red is required; the target is a creature, and four damage kills the
/// Ogre.
#[test]
fn sarpadian_simulacrum_sacrifices_to_deal_four_to_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = main_with(&[sarpadian_simulacrum()], &[gray_ogre()]);
    let me = on_battlefield(&engine, p0, sarpadian_simulacrum()).expect("seated");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("ogre");
    assert_eq!(pt(&engine, me), (1, 1));

    float_mana(&mut engine, ManaColor::Colorless, 4);
    assert!(!is_offered(&engine, me, 0), "{{R}} is required");
    float_mana(&mut engine, ManaColor::Red, 1);
    activate(&mut engine, p0, sarpadian_simulacrum(), 0);
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, gray_ogre()).is_some(), "4 damage");
    assert!(in_graveyard(&engine, p0, sarpadian_simulacrum()).is_some());
}
