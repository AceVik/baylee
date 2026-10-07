//! `cards/lands/check/woodland_cemetery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Woodland Cemetery is a check land: it enters tapped unless a Swamp or a
/// Forest is already under its controller, and then taps for {B} or {G}.
/// Two boards, because the printed sentence is a condition and its own
/// negation — a Mountain alone must leave it tapped where a Forest must not —
/// and the second board's activation is the only reading that tells
/// "{B} or {G}" from either colour printed on its own.
#[test]
fn woodland_cemetery_checks_for_a_swamp_or_forest_and_taps_for_black_or_green() {
    let p0 = PlayerId::new(0);

    // Board one: one Mountain, which is a land but neither half of the
    // printed filter, so the check fails and the land enters tapped.
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .hand(0, &[woodland_cemetery()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let tapped = play_land(&mut engine, p0, woodland_cemetery());
    assert!(
        entered_tapped(&engine, tapped),
        "no Swamp and no Forest: the check is false and the land enters tapped"
    );

    // Board two: a Forest, which is one half of the filter. The same card,
    // played the same way, is untapped — and its own {T} is payable in this
    // very phase.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[woodland_cemetery()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = play_land(&mut engine, p0, woodland_cemetery());
    assert!(
        !entered_tapped(&engine, land),
        "a Forest is the printed check, so the land enters untapped"
    );

    activate(&mut engine, p0, woodland_cemetery(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Green),
        "\"Add {{B}} or {{G}}\": both, and nothing else: {options:?}"
    );
    assert_eq!(options.len(), 2, "black and green are the whole menu");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black is one of the two colours printed on the card");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "a Forest and a Woodland Cemetery stand, and exactly one was tapped"
    );
    assert!(is_tapped(&engine, land), "{{T}} is the whole price");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability resolves as it is activated, so no stack is left"
    );
}
