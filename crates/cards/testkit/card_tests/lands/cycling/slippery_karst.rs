//! `cards/lands/cycling/slippery_karst.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Slippery Karst prints three lines and one board plays all three: it enters
/// tapped, it taps for `{G}`, and it cycles for `{2}`. The enter modifier is
/// only readable off a real `PlayLand` — `starting_battlefield` places a
/// permanent with `Cause::Setup` and no replacement effect ever looks at it —
/// so the copy that is played is the one asserted to arrive tapped, while a
/// second, seeded copy is the `{T}: Add {G}` that is read. Green is the
/// isolation: the Sol Rings beside it make colourless, so the single green in
/// the pool can only have come off the Karst. The copy left in hand is then
/// the cycling cost's other half, discarded and drawn back for out of the same
/// pool.
#[test]
fn slippery_karst_enters_tapped_makes_green_and_cycles_itself_from_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[slippery_karst(), quiet_artifact(), quiet_artifact()])
        .hand(0, &[slippery_karst(), slippery_karst()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // "This land enters tapped." — played, so the enter modifier really runs.
    let played = play_land(&mut engine, p0, slippery_karst());
    assert!(
        is_tapped(&engine, played),
        "a land played at sorcery speed arrives through its own enter modifiers"
    );

    // "{{T}}: Add {{G}}." — two Sol Rings and the seeded Karst are every source
    // on the board: Sol Ring prints {{T}}: Add {{C}}{{C}}, so five mana out of
    // three taps, and exactly one of the five is green.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "two Sol Rings and the Karst, and nothing else taps"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the Karst is the only green source on this board"
    );

    // "Cycling {{2}}" — an ability activated from hand, of the card it discards.
    let cycled = in_hand(&engine, p0, slippery_karst()).expect("the copy left in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == cycled)
        .expect("cycling is offered from hand once the {{2}} is in the pool");
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("{{2}} is floating and the card is its own other half");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, slippery_karst()).is_some(),
        "discarding the card is half the cost, and it goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, slippery_karst()).is_some(),
        "only the copy in hand was discarded: the permanents never moved"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"draw a card\" — one off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card discarded and one drawn, so the hand is the size it was"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "{{2}} of the five went to the cycle"
    );
}
