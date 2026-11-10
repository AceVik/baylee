//! `cards/lands/utility/crumbling_vestige.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Crumbling Vestige` is a utility land under `Coverage::Implemented` that enters tapped,
/// adds one mana of any color when it enters, and has "{T}: Add {C}."
/// When played from hand, it enters tapped and its enters-the-battlefield trigger asks for a color choice.
/// Choosing green adds one green mana to the pool. On the next turn after untapping, its mana ability
/// taps to produce one colorless mana.
#[test]
fn crumbling_vestige_enters_tapped_adds_color_and_taps_for_colorless() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[crumbling_vestige()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let vestige = play_land(&mut engine, p0, crumbling_vestige());
    assert!(
        is_tapped(&engine, vestige),
        "Crumbling Vestige enters tapped"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseColor prompt for ETB trigger, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&ManaColor::Green),
        "green is offered among any-color options: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("choosing green mana is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "ETB trigger added one green mana to the pool"
    );

    // Advance to next turn so Crumbling Vestige untaps and mana empties.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, vestige),
        "Crumbling Vestige untaps during controller's untap step"
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 1, "Crumbling Vestige taps for mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "Crumbling Vestige produces one colorless mana"
    );
}
