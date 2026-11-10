//! `cards/lands/utility/accursed_duneyard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Accursed Duneyard prints `{{T}}: Add {{C}}.` and `{{2}}, {{T}}: Regenerate
/// target Shade, Skeleton, Specter, Spirit, Vampire, Wraith, or Zombie.`
///
/// The longest of the four regenerating lands' filters — seven subtypes —
/// and the only one with a mana price on top of the tap, so the board holds
/// the `{{2}}` that price wants and the Elf that the seven subtypes refuse.
///
/// Unlike its three neighbours this pin was a real one: the Zombie was here
/// and the `{{2}}` was floating, so it failed the day the rule was written,
/// which is what a pin is for.
#[test]
fn accursed_duneyard_pays_two_to_shield_a_zombie_and_not_an_elf() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                accursed_duneyard(),
                forest(),
                forest(),
                festering_goblin(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let duneyard =
        on_battlefield(&engine, p0, accursed_duneyard()).expect("accursed duneyard on battlefield");
    let zombie = on_battlefield(&engine, p0, festering_goblin()).expect("the Zombie is seated");

    // Float everything but the land under test, which has to stay untapped
    // for the offer this reads. The Elves are a mana source too and tap
    // along with the Forests; a tapped creature is a target all the same.
    tap_mana_except(&mut engine, p0, duneyard);
    let floating = engine.state().players[0].mana_pool.total();
    assert!(floating >= 2, "the two Forests pay the {{2}}: {floating}");
    assert!(!is_tapped(&engine, duneyard));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(duneyard, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        legal.abilities.contains(&(duneyard, 1)),
        "and ability 1, with the {{2}} floating and a Zombie to point it at"
    );

    activate(&mut engine, p0, accursed_duneyard(), 1);
    let menu = aim_at(&mut engine, p0, zombie);
    assert_eq!(
        menu,
        vec![zombie],
        "seven subtypes and the Elf is none of them"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(zombie)
            .expect("the Zombie is still there")
            .regeneration_shields,
        1
    );
    assert!(is_tapped(&engine, duneyard));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        floating - 2,
        "and the generic {{2}} came out of the pool"
    );
}
