//! `cards/lands/manlands/stirring_wildwood.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Stirring Wildwood prints three lines and all three are playable: it enters
/// tapped, it taps for {G} or {W}, and {1}{G}{W} turns it into a 3/4 Elemental
/// with reach that is still a land. The land drop is a real `PlayLand` and not
/// a battlefield seed, because a permanent the harness *places* is placed and
/// not entered — no `EnterModifier` looks at it, so the printed comes-in-tapped
/// line would be invisible and the test would pass on a Mountain. The turn
/// cycle then does the work the entry line demands: the tapped land is no mana
/// route at all on the turn it arrives (CR 502.3 has not run), and the next
/// turn is where its mana ability and its animation can both be paid for.
#[test]
fn stirring_wildwood_enters_tapped_then_taps_for_white_and_animates() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), plains(), plains()])
        .hand(0, &[stirring_wildwood()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let wildwood = play_land(&mut engine, p0, stirring_wildwood());
    assert!(
        entered_tapped(&engine, wildwood),
        "\"This land enters tapped\""
    );
    // Which is exactly what its {T} cannot pay: four basics are the whole of
    // this turn's mana, and the land that just arrived is no route among them.
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(
        taken, 4,
        "two Forests and two Plains, and the Wildwood that entered tapped is \
         not a mana source until it untaps"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "and that is all the mana this turn has"
    );

    // A whole turn cycle, so that the untap step gives the land back.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, wildwood),
        "the untap step ran — without this the mana ability below would be \
         read off a land that never came back"
    );

    // Ability 0 is the printed "{T}: Add {G} or {W}", and the colour is a
    // question rather than a default (CR 105.4: colorless is no color at all).
    activate(&mut engine, p0, stirring_wildwood(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{G}} or {{W}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert_eq!(
        options.len(),
        2,
        "the two colours it prints and no more: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::White),
        "both {{G}} and {{W}} are on offer: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colours it offered");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "the colour that was named, off the land's own tap"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one mana, and nothing else is floating"
    );
    assert!(is_tapped(&engine, wildwood), "the land paid its own {{T}}");

    // The rest of the {1}{G}{W}, with the Wildwood kept out of the tapping: it
    // is the permanent the animation is about, not a mana source for it.
    tap_all_mana_but(&mut engine, p0, Some(stirring_wildwood()));
    let pool_before = engine.state().players[0].mana_pool.total();
    assert_eq!(
        pool_before, 5,
        "the white it made plus the two Forests and two Plains"
    );

    // Ability 1 is the animation, and it is no mana ability: the cost is paid
    // (CR 601.2h) and the effect waits on the stack.
    activate(&mut engine, p0, stirring_wildwood(), 1);
    assert!(!stack_is_empty(&engine), "animating uses the stack");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        pool_before - 3,
        "and {{1}}{{G}}{{W}} came out of the pool for it"
    );
    pass_until(&mut engine, stack_is_empty);

    let kinds = types(&engine, wildwood);
    assert!(
        kinds.contains(TypeSet::CREATURE) && kinds.contains(TypeSet::LAND),
        "\"becomes a 3/4 green and white Elemental creature with reach. It's \
         still a land\": {kinds:?}"
    );
    assert_eq!(
        pt(&engine, wildwood),
        (3, 4),
        "the printed body, on a permanent that has none of its own"
    );
    assert!(
        keywords(&engine, wildwood).contains(KeywordSet::REACH),
        "and the reach the animation grants"
    );
}
