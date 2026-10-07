//! `cards/lands/tapland/woodland_stream.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Woodland Stream is a dual land under `Coverage::Implemented` that enters tapped and produces {G} or {U}.
/// Playing the land from hand puts it onto the battlefield in a tapped state.
/// After cycling through an opponent's turn to its controller's next main phase, the land untaps.
/// Activating its mana ability prompts for a color choice between Green and Blue and deposits the chosen mana into the pool.
#[test]
fn woodland_stream_enters_tapped_and_produces_green_or_blue() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[woodland_stream()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, woodland_stream());
    assert!(
        entered_tapped(&engine, land),
        "Woodland Stream enters tapped"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, land),
        "Woodland Stream untaps on the next turn"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .expect("activating mana ability is legal");

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 2, "offers exactly two colors");
    assert!(options.contains(&ManaColor::Green), "offers Green");
    assert!(options.contains(&ManaColor::Blue), "offers Blue");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("choosing Blue is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "one blue mana added to pool"
    );
    assert!(is_tapped(&engine, land), "land is tapped after activation");
}
