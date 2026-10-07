//! `cards/lands/utility/amonkhet_raceway.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Amonkhet Raceway prints `Start your engines!`, `{{T}}: Add {{C}}.`, and
/// `Max speed — {{T}}: Target creature gains haste until end of turn.`
///
/// Under `Coverage::Partial`, the speed mechanic and the Max speed condition have no
/// DSL vocabulary and are omitted. Ability index 0 provides `{{T}}: Add {{C}}`, while
/// ability index 1 is not offered in `legal.abilities` despite controlling an untapped creature
/// (`llanowar_elves()`). Activating ability 0 adds one colorless mana to `pool.available(ManaColor::Colorless)`.
#[test]
fn amonkhet_raceway_taps_for_colorless_and_omits_speed_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[amonkhet_raceway(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let raceway =
        on_battlefield(&engine, p0, amonkhet_raceway()).expect("amonkhet raceway on battlefield");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(raceway, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        !legal.abilities.contains(&(raceway, 1)),
        "under `Coverage::Partial`, Max speed haste ability is omitted despite creature on battlefield"
    );

    activate(&mut engine, p0, amonkhet_raceway(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, raceway));
}
