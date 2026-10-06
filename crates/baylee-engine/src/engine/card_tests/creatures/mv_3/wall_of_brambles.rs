//! `cards/creatures/mv_3/wall_of_brambles.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wall of Brambles — Defender; `{G}`: Regenerate this creature.
#[test]
fn wall_of_brambles_regenerates_for_g_and_cannot_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wall_of_brambles(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wall = on_battlefield(&engine, p0, wall_of_brambles()).expect("seated");
    assert_eq!(pt(&engine, wall), (2, 3));
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, wall_of_brambles(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(wall).unwrap().regeneration_shields, 1);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(!attackers.contains(&wall), "Defender: it cannot attack");
}
