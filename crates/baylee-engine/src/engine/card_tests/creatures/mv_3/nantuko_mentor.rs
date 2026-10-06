//! `cards/creatures/mv_3/nantuko_mentor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Nantuko Mentor` prints `{{2}}{{G}}, {{T}}: Target creature gets +X/+X until end of turn, where X is that creature's power.`
///
/// Marked `Coverage::Implemented`, paying `{{2}}{{G}}` and tapping `Nantuko Mentor` activates `Effect::PumpTarget` with `Amount::TargetPower`.
/// Targeting `aurochs()` (2/3) grants +2/+2 until end of turn, elevating its body to 4/5.
#[test]
fn nantuko_mentor_doubles_target_creature_power_and_toughness() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[nantuko_mentor(), aurochs(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mentor = on_battlefield(&engine, p0, nantuko_mentor()).expect("mentor deployed");
    let aurochs_obj = on_battlefield(&engine, p0, aurochs()).expect("aurochs deployed");
    assert_eq!(pt(&engine, mentor), (1, 1));
    assert_eq!(pt(&engine, aurochs_obj), (2, 3));

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        3
    );
    assert!(!is_tapped(&engine, mentor));

    activate(&mut engine, p0, nantuko_mentor(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&aurochs_obj));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![aurochs_obj],
                players: vec![],
            },
        )
        .unwrap();

    assert!(is_tapped(&engine, mentor), "mentor tapped for its cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "mana spent on activation"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, aurochs_obj),
        (4, 5),
        "aurochs received +2/+2 matching its power of 2"
    );
}
