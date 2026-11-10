//! `cards/lands/pain/scabland.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Scabland prints three lines that are one card: "This land enters tapped",
/// "{T}: Add {C}", and "{T}: Add {R} or {W}. This land deals 1 damage to you."
///
/// The board is bare on purpose — the land, an empty pool, nothing else — so
/// that both numbers the test reads can only have come from this printing.
/// What has to be separated is the ordering: the land arrives tapped, so
/// neither `{T}` line is offered until a turn cycle stands it back up
/// (CR 502.3), and the line pressed afterwards is the second one, which is
/// why the colored mana and the lost life arrive together while `Add {C}`
/// sits beside it untouched.
#[test]
fn scabland_enters_tapped_and_trades_one_life_for_a_named_color_of_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .life(0, 20)
        .life(1, 20)
        .hand(0, &[scabland()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Played and not seeded: `starting_battlefield` places a permanent with
    // no entry at all, so only a real land drop shows the enter modifier ran.
    let land = play_land(&mut engine, p0, scabland());
    assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a played land leaves the seat holding priority: {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "and a tapped land cannot pay a `{{T}}`: {:?}",
        legal.abilities
    );

    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "a turn cycle stands the land back up"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(source, _)| *source == land)
        .map(|(_, index)| *index)
        .collect();
    assert!(
        offered.contains(&0) && offered.contains(&1),
        "both printed lines are `{{T}}` and the land is untapped now: {offered:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats, so the pool below is the land's own"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing has cost life yet"
    );

    // Ability 1 is "{{T}}: Add {{R}} or {{W}}. This land deals 1 damage to you."
    activate(&mut engine, p0, scabland(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{W}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the color");
    assert_eq!(
        options.len(),
        2,
        "the two colors the card prints: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::White),
        "red or white, and nothing else: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colors it offered");
    pass_until(&mut engine, stack_is_empty);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "and none of the other one"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"This land deals 1 damage to you\" — the price of the colored line"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the damage is the controller's own, not the opponent's"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
