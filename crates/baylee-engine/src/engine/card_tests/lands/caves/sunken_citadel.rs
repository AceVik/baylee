//! `cards/lands/caves/sunken_citadel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sunken Citadel prints `This land enters tapped. As it enters, choose a color`, `{{T}}: Add one mana of the chosen color`,
/// and `{{T}}: Add two mana of the chosen color. Spend this mana only to activate abilities of land sources.`
/// The card is marked `Coverage::Partial` and the two-mana ability is left off: its spend restriction cannot be stated, and without it the land made two mana for anything.
/// When played, Sunken Citadel prompts for a color choice as it enters, arrives tapped, untaps on the next turn,
/// offers only its one-mana ability, and that makes one mana of the chosen color.
#[test]
fn sunken_citadel_chooses_color_and_taps_for_one_of_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[sunken_citadel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let citadel = play_land(&mut engine, p0, sunken_citadel());
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(entered_tapped(&engine, citadel));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, citadel));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(citadel, 1)),
        "the unrestricted two-mana ability is gone"
    );

    activate(&mut engine, p0, sunken_citadel(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert!(is_tapped(&engine, citadel));
}
