//! `cards/lands/manlands/svogthos_the_restless_tomb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Svogthos, the Restless Tomb prints `{{T}}: Add {{C}}` and `{3}{B}{G}: Until
/// end of turn, this land becomes a black and green Plant Zombie creature with
/// "This creature's power and toughness are each equal to the number of
/// creature cards in your graveyard." It's still a land.`
///
/// Under `Coverage::Implemented`, all printed characteristics are fully
/// realized. This test seeds three creature cards into the graveyard, pays
/// `{3}{B}{G}` to animate Svogthos, and verifies that it becomes a 3/3 black
/// and green Plant Zombie creature while continuing to be a land.
#[test]
fn svogthos_animates_with_pt_equal_to_graveyard_creature_count() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, quiet_creature())
        .battlefield(
            0,
            &[
                svogthos_the_restless_tomb(),
                swamp(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 3);
    assert!(in_graveyard(&engine, p0, quiet_creature()).is_some());

    tap_all_mana_but(&mut engine, p0, Some(svogthos_the_restless_tomb()));
    activate(&mut engine, p0, svogthos_the_restless_tomb(), 1);
    pass_until(&mut engine, stack_is_empty);

    let tomb =
        on_battlefield(&engine, p0, svogthos_the_restless_tomb()).expect("svogthos on battlefield");
    let chars = engine
        .state()
        .object(tomb)
        .expect("tomb exists")
        .characteristics();
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(chars.types.contains(TypeSet::LAND), "it's still a land");
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::PLANT)
    );
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::ZOMBIE)
    );
    assert!(chars.colors.contains(baylee_core::color::Color::Black));
    assert!(chars.colors.contains(baylee_core::color::Color::Green));
    assert_eq!(pt(&engine, tomb), (3, 3));
}
