//! `cards/enchantments/auras/mv_1/white_ward.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The play test for the reader's protection rule, and for the one
/// exception it reads: White Ward is a white Aura that grants protection
/// from white, and "this effect doesn't remove this Aura". Without the
/// exception CR 702.16c would put the Ward into the graveyard the moment it
/// arrived. With it, the Ward stays, and a white spell still cannot target
/// the creature (CR 702.16b).
#[test]
fn white_ward_protects_from_white_and_stays_attached() {
    let p0 = PlayerId::new(0);
    let ward = card_index("860468b3-625a-4663-a5ae-336ae10fc7d0");
    // The opponent's creature is there so that Swords has something to
    // target; without it the spell is not castable at all.
    let mut engine = Duel::new(11, plains())
        .battlefield(0, &[plains(), plains(), quiet_creature()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[ward, swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creature = on_battlefield(&engine, p0, quiet_creature()).unwrap();
    let plains_ids = all_on_battlefield(&engine, p0, plains());
    tap_mana_where(&mut engine, p0, |id| id == plains_ids[0]);
    cast_with_floating(&mut engine, p0, ward);
    let _ = aim_at(&mut engine, p0, creature);
    pass_until(&mut engine, stack_is_empty);
    let attached = on_battlefield(&engine, p0, ward).expect("the Ward stays on the battlefield");
    assert_eq!(
        engine.state().object(attached).and_then(|o| o.attached_to),
        Some(creature),
        "attached to the creature it protects"
    );

    cast_from_hand(&mut engine, p0, swords_to_plowshares());
    let offered = match engine.pending() {
        Pending::ChooseTargets { options, .. } => options.clone(),
        other => panic!("Swords asks for a target: {other:?}"),
    };
    assert!(
        !offered.contains(&creature),
        "a white spell cannot target it: {offered:?}"
    );
}
