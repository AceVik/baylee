//! `cards/creatures/artifacts/mv_4/golden_guardian.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Golden Guardian` prints `Defender`, `{{2}}: This creature fights another target creature you control. When this creature dies this turn, return it to the battlefield transformed under your control.`, and transforms into `Gold-Forge Garrison`.
///
/// Marked `Coverage::Partial`, its front face carries `KeywordSet::DEFENDER` and a 4/4 body, which prevents it from being declared as an attacker in `Pending::ChooseAttackers` while `young_wolf()` can attack.
/// With `{{2}}` floating mana and another creature controlled, the unmodelled fight/transform ability is omitted from `legal.abilities`.
#[test]
fn golden_guardian_has_defender_and_omits_fight_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[golden_guardian(), young_wolf(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let guardian = on_battlefield(&engine, p0, golden_guardian()).expect("guardian on battlefield");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("wolf on battlefield");

    assert_eq!(pt(&engine, guardian), (4, 4));
    assert!(keywords(&engine, guardian).contains(KeywordSet::DEFENDER));

    tap_mana_except(&mut engine, p0, guardian);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(!is_tapped(&engine, guardian));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == guardian),
        "under `Coverage::Partial` the fight ability is not offered despite {{2}} floating and legal target"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers, got {:?}", engine.pending());
    };
    assert!(
        attackers.contains(&wolf),
        "young wolf without defender is a legal attacker"
    );
    assert!(
        !attackers.contains(&guardian),
        "golden guardian with defender cannot be declared as an attacker"
    );
}
