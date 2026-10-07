//! `cards/lands/utility/sejiri_steppe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn sejiri_steppe_enters_tapped_and_taps_for_white() {
    let p0 = PlayerId::new(0);

    // The control, first: a basic Forest, played the same way this test plays
    // the Steppe.
    let mut control = Duel::new(SEED, forest()).hand(0, &[forest()]).start();
    keep_mulligans(&mut control);
    reach_main_phase(&mut control, p0);
    let plain = play_land(&mut control, p0, forest());
    assert!(
        !entered_tapped(&control, plain),
        "a land with no entry clause arrives untapped, so playing a land is \
         not itself what taps one"
    );

    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[sejiri_steppe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let steppe = play_land(&mut engine, p0, sejiri_steppe());
    assert!(
        on_battlefield(&engine, p0, sejiri_steppe()).is_some(),
        "the land drop put it onto the battlefield"
    );
    assert!(
        entered_tapped(&engine, steppe),
        "\"This land enters tapped\" — a real `PlayLand`, so the entry \
         modifier runs as a replacement effect instead of being skipped"
    );

    // Its next turn. A permanent that arrived tapped is untapped in its
    // controller's untap step and not before, so the mana ability it prints
    // is unreachable on the turn it was played — a pool read here would be
    // reading the entry clause a second time and calling it the mana line.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, steppe), "the untap step stood it up");

    // Mana into the pool before the claim: what a mana ability is worth is
    // read off the pool, not off the permanent that could still be tapped.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "{{T}}: Add {{W}}, and no other source stood on this board to make it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one land, one mana, and no second route off it"
    );
    assert!(
        entered_tapped(&engine, steppe),
        "which is the {{T}} the Steppe paid by tapping"
    );
}

/// "When this land enters, target creature you control gains protection
/// from the color of your choice until end of turn." The color is asked as
/// the trigger resolves, of its controller, among the five; red is named,
/// and the creature is protected from a red source and not from a white
/// one, until the turn ends.
#[test]
fn sejiri_steppe_protects_a_creature_from_the_color_its_controller_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[llanowar_elves()])
        .battlefield(1, &[thundering_giant()])
        .hand(0, &[sejiri_steppe()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let guard = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    play_land(&mut engine, p0, sejiri_steppe());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let menu = aim_at(&mut engine, p0, guard);
    assert_eq!(menu, vec![guard], "a creature you control, and only that");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, p0, "the color of *your* choice");
    assert_eq!(options.len(), 5, "any of the five colors");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red is on the list");
    pass_until(&mut engine, stack_is_empty);

    let red = on_battlefield(&engine, p1, thundering_giant()).unwrap();
    let white = in_hand(&engine, p1, swords_to_plowshares()).unwrap();
    assert!(
        crate::eval::protected_from(engine.state(), guard, red),
        "protection from red"
    );
    assert!(
        !crate::eval::protected_from(engine.state(), guard, white),
        "and from nothing else"
    );

    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert!(
        !crate::eval::protected_from(engine.state(), guard, red),
        "until end of turn"
    );
}
