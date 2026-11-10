//! `cards/lands/gates/orzhov_guildgate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// A Gate prints two sentences and both are load-bearing: it enters tapped,
/// and its `{T}` offers a *choice* between two colours rather than one fixed
/// output. The scenario plays it as a real land drop, because a permanent
/// seeded onto the battlefield is placed rather than entering and no
/// `EnterModifier` would be read — and it reads the tapped state against the
/// plain Forest beside it, which the same drop left standing. It then walks a
/// whole turn cycle, since a land that arrived tapped has no `{T}` in the turn
/// it arrived (CR 502.3 is the only thing that hands it back), and only then
/// takes the two-colour question apart.
#[test]
fn orzhov_guildgate_enters_tapped_and_then_offers_white_or_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(613, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[orzhov_guildgate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gate = play_land(&mut engine, p0, orzhov_guildgate());
    assert!(
        entered_tapped(&engine, gate),
        "\"This land enters tapped\" — and as an entry, not a placement"
    );
    let woods = on_battlefield(&engine, p0, forest()).expect("the Forest was dealt");
    assert!(
        !entered_tapped(&engine, woods),
        "the Forest is the control: a land printing no entry modifier stands, \
         so the tapped Gate is the card's own text and not a board this \
         harness warped"
    );

    // The consequence of the same printed sentence. The offer is filtered by
    // `can_afford`, and a `{T}` price cannot be paid by a permanent that is
    // already down — so the Gate is not merely unusable this turn, it is not
    // even offered one.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the land drop hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        deeds(&legal, &[gate]).is_empty(),
        "a land that entered tapped has no tap to spend in the turn it \
         arrived: {:?}",
        deeds(&legal, &[gate])
    );

    // One full turn cycle, and the untap step is the only thing that may hand
    // the land its `{T}` back.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, gate),
        "its controller's untap step stood the Gate back up"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        matches!(deeds(&legal, &[gate])[..], [(0, Deed::Ability(0))]),
        "the Gate prints one ability, and a nonbasic land's own {{T}} is a \
         printed mana ability offered by index rather than the CR 305.6 \
         shortcut: {:?}",
        deeds(&legal, &[gate])
    );
    activate(&mut engine, p0, orzhov_guildgate(), 0);

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{B}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Black),
        "the two colours the Gate prints: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and nothing else — not a third colour, and not colourless: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, not a default"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "and never the one that was not"
    );
    assert_eq!(pool.total(), 1, "one tap, one mana");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, gate), "the Gate paid its own {{T}}");
    assert!(
        !is_tapped(&engine, woods),
        "and the Forest beside it never moved, so the black mana has no other \
         source on this board"
    );
}
