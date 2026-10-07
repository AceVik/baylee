//! `cards/lands/utility/secluded_starforge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Secluded Starforge` prints `{{T}}: Add {{C}}.`, `{{2}}, {{T}}, Tap X untapped artifacts you control: Target creature gets +X/+0 until end of turn. Activate only as a sorcery.`, and `{{5}}, {{T}}: Create a 2/2 colorless Robot artifact creature token.`
///
/// Under `Coverage::Partial`, the pump is left off because its cost taps a counted number of artifacts, which no cost part says. So the card has two abilities, the mana at 0 and the Robot at 1, and nothing at 2.
/// With five Forests floating `{{5}}`, the Robot ability taps the land and makes one 2/2 colorless artifact creature.
#[test]
fn secluded_starforge_taps_for_colorless_and_makes_a_robot_for_five() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                secluded_starforge(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let starforge = on_battlefield(&engine, p0, secluded_starforge())
        .expect("secluded starforge on battlefield");

    tap_mana_except(&mut engine, p0, starforge);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(starforge, 0)),
        "the mana ability"
    );
    assert!(legal.abilities.contains(&(starforge, 1)), "the Robot");
    assert!(
        !legal.abilities.contains(&(starforge, 2)),
        "and no third ability: the pump is not written"
    );

    activate(&mut engine, p0, secluded_starforge(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, starforge));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "{{5}} paid");
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Robot");
    assert_eq!(pt(&engine, tokens[0]), (2, 2));
    let t = types(&engine, tokens[0]);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));
}

/// Secluded Starforge's `{{T}}: Add {{C}}`, on its own.
#[test]
fn secluded_starforge_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[secluded_starforge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let starforge = on_battlefield(&engine, p0, secluded_starforge())
        .expect("secluded starforge on battlefield");
    activate(&mut engine, p0, secluded_starforge(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, starforge));
}
