//! `cards/lands/towns/insomnia_crown_city.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Insomnia, Crown City` is a Town land under `Coverage::Implemented` that enters tapped
/// and taps for `{W}` or `{B}`.
/// When played from hand, it enters tapped. Advancing past the opponent's turn to its controller's
/// next main phase untaps it. Activating its mana ability prompts for a color choice between white
/// and black, successfully adding the chosen color to the mana pool.
#[test]
fn insomnia_crown_city_enters_tapped_and_produces_mana_choice() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[insomnia_crown_city()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let city = play_land(&mut engine, p0, insomnia_crown_city());
    assert!(
        is_tapped(&engine, city),
        "Insomnia, Crown City enters the battlefield tapped"
    );

    // Pass turn to untap the land.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, city),
        "Insomnia, Crown City untaps during the untap step"
    );

    activate(&mut engine, p0, insomnia_crown_city(), 0);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Black),
        "mana choice offers both white and black: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("choosing black mana is legal");

    assert!(
        is_tapped(&engine, city),
        "Insomnia, Crown City is tapped after activating its mana ability"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "one black mana was added to the pool"
    );
}
