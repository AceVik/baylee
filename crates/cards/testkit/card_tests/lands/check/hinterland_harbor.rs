//! `cards/lands/check/hinterland_harbor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hinterland Harbor prints two sentences, and neither can be read off a seeded
/// board: `SeatSpec::starting_battlefield` places a permanent without running
/// its entry replacement effects, so a Harbor put there would stand untapped
/// whatever its check said. Three boards therefore *play* the land out of hand —
/// over a Forest, where the check holds, the land arrives untapped and taps for
/// the blue no other permanent there can make; over an Island, the second limb
/// of the printed `or`; and over the *opponent's* Forest alone, where the check
/// fails, the land enters tapped and its tap ability is not offered at all for
/// the rest of that turn.
#[test]
fn hinterland_harbor_checks_for_a_forest_or_island_of_your_own_and_taps_for_green_or_blue() {
    let p0 = PlayerId::new(0);

    // A Forest you control is the condition, and the land is played rather
    // than seeded because only a real entry runs the check.
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[forest()])
        .hand(0, &[hinterland_harbor()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let harbor = play_land(&mut engine, p0, hinterland_harbor());
    assert!(
        !entered_tapped(&engine, harbor),
        "'enters tapped unless you control a Forest or an Island' — a Forest \
         you control is exactly that"
    );

    // {T}: Add {G} or {U}. Both printed colours are enumerated and the answer
    // is the one taken — blue, which nothing else on this board produces.
    activate(&mut engine, p0, hinterland_harbor(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{U}}\" is a choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the land's controller names the colour");
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
        "the two colours the card prints and no other: {options:?}"
    );
    assert_eq!(options.len(), 2, "'or' is two answers, not five");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap: the untapped Forest beside it paid nothing"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(
        is_tapped(&engine, harbor),
        "and the Harbor is the source that tapped, which is why the blue is its own"
    );

    // The other limb of the printed `or`: an Island you control.
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[island()])
        .hand(0, &[hinterland_harbor()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let harbor = play_land(&mut engine, p0, hinterland_harbor());
    assert!(
        !entered_tapped(&engine, harbor),
        "an Island you control satisfies the condition too, and the check is a \
         disjunction rather than a Forest test"
    );

    // And the control: the same Forest, under the *other* seat. The card says
    // "you control", so a land across the table must not open the gate — and
    // it is the only land in the game there, so nothing else can be doing it.
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(1, &[forest()])
        .hand(0, &[hinterland_harbor()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let harbor = play_land(&mut engine, p0, hinterland_harbor());
    assert!(
        entered_tapped(&engine, harbor),
        "the opponent's Forest is not a Forest you control, so the land enters \
         tapped"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the land drop leaves the seat holding priority, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == harbor),
        "a tapped land pays no tap, so the ability is not offered at all — \
         which is what entering tapped costs it for the rest of the turn"
    );
}
