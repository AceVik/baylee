//! `cards/lands/towns/treno_dark_city.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Treno, Dark City` is a Town land under `Coverage::Implemented` that enters tapped and taps for `{U}` or `{B}`.
/// Playing the land puts it onto the battlefield tapped, where its mana ability cannot be activated this turn.
/// On the following turn after untapping, activating the mana ability prompts for a color choice and adds the chosen mana to the pool.
#[test]
fn treno_dark_city_enters_tapped_and_produces_blue_or_black() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[treno_dark_city()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, treno_dark_city());
    assert!(
        entered_tapped(&engine, land),
        "Treno, Dark City enters tapped"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(land, 0)),
        "tapped land cannot activate its mana ability on entry turn"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        !is_tapped(&engine, land),
        "Treno, Dark City untaps on next turn"
    );

    activate(&mut engine, p0, treno_dark_city(), 0);

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "activating player chooses color");
    assert_eq!(options.len(), 2, "offers two colors");
    assert!(options.contains(&ManaColor::Blue), "offers Blue");
    assert!(options.contains(&ManaColor::Black), "offers Black");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("choosing Black is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "one black mana added to pool"
    );
    assert!(
        is_tapped(&engine, land),
        "Treno, Dark City is tapped after activation"
    );
}
