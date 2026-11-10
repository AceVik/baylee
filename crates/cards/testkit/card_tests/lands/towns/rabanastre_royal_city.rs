//! `cards/lands/towns/rabanastre_royal_city.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Rabanastre, Royal City` is a Town land under `Coverage::Implemented` that enters tapped and taps for `{R}` or `{W}`.
/// Playing the land from hand puts it onto the battlefield tapped, where its mana ability cannot be activated this turn.
/// On the subsequent turn after untapping, activating its mana ability prompts for a color choice and deposits the chosen mana into the pool.
#[test]
fn rabanastre_royal_city_enters_tapped_and_produces_red_or_white() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[rabanastre_royal_city()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, rabanastre_royal_city());
    assert!(
        entered_tapped(&engine, land),
        "Rabanastre, Royal City enters tapped"
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
        "Rabanastre, Royal City untaps on next turn"
    );

    activate(&mut engine, p0, rabanastre_royal_city(), 0);

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "activating player chooses color");
    assert_eq!(options.len(), 2, "offers exactly two colors");
    assert!(options.contains(&ManaColor::Red), "offers Red");
    assert!(options.contains(&ManaColor::White), "offers White");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("choosing White is legal");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "one white mana added to pool"
    );
    assert!(
        is_tapped(&engine, land),
        "Rabanastre, Royal City is tapped after activation"
    );
}
