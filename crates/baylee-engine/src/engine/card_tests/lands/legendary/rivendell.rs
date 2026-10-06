//! `cards/lands/legendary/rivendell.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rivendell: "{1}{U}, {T}: Scry 2. Activate only if you control a legendary
/// creature." Without one it is not offered; with Jin it asks for two cards.
#[test]
fn rivendell_scries_two_only_while_a_legendary_creature_is_controlled() {
    let p0 = PlayerId::new(0);
    let offered = |board: &[CardIndex]| -> (Engine<RegistryLookup>, ObjectId, bool) {
        let mut engine = Duel::new(2305, island()).battlefield(0, board).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let r = on_battlefield(&engine, p0, rivendell()).expect("rivendell");
        tap_mana_where(&mut engine, p0, |id| id != r);
        let yes = priority_offer(&engine).abilities.contains(&(r, 1));
        (engine, r, yes)
    };
    let (_, _, without) = offered(&[rivendell(), island(), island(), quiet_creature()]);
    assert!(!without, "no legendary creature, no scry");
    let (mut engine, _, with) = offered(&[rivendell(), island(), island(), jin_gitaxias()]);
    assert!(with, "a legendary creature enables it");

    activate(&mut engine, p0, rivendell(), 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        player,
        cards,
        piles,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the arrangement")
    };
    assert_eq!(player, p0);
    assert_eq!(cards.len(), 2, "scry 2 looks at two cards");
    assert_eq!(piles, scry_piles(2));

    // Answering the question is what finishes the ability (CR 701.18a), so
    // the test walks the choice through instead of stopping at the prompt:
    // one card is bottomed, the other stays on top, and the ability reaches
    // the firing log that had never seen it resolve.
    let (top, second) = (cards[0], cards[1]);
    engine
        .apply(p0, look_answer(&cards, &[second]))
        .expect("one card bottomed, the other kept");
    pass_until(&mut engine, stack_is_empty);
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library.first().copied(),
        Some(second),
        "the bottomed card is the library's last"
    );
    assert_eq!(
        library.last().copied(),
        Some(top),
        "the kept card is the new top"
    );
}
