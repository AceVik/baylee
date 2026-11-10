//! `cards/lands/loci/glimmerpost.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Glimmerpost: "When this land enters, you gain 1 life for each Locus on the battlefield." / "{T}: Add {C}."
/// When a second Glimmerpost enters a battlefield that already controls one Locus, its trigger counts both.
/// Upon resolution, the player gains 2 life and the land can be tapped for {C}.
#[test]
fn glimmerpost_enters_gaining_life_per_locus_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(106, forest())
        .battlefield(0, &[glimmerpost()])
        .hand(0, &[glimmerpost()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let post = play_land(&mut engine, p0, glimmerpost());

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        life_before + 2,
        "two Loci on the battlefield gain 2 life"
    );

    // Addressed by object, not by card: two Glimmerposts are on the
    // battlefield and `activate` would take whichever the legal set offers
    // first, which is the one that was already there.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: post,
                ability_index: 0,
            },
        )
        .expect("the mana ability activates");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, post));
}
