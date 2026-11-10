//! `cards/creatures/mv_4/jungle_barrier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Jungle Barrier — {2}{G}{U} Plant Wall, a 2/6 with Defender and "When this
/// creature enters, draw a card."
///
/// Both printed lines are the engine's answer rather than the card's, so one
/// cast has to be read in two places at once: the body and the keyword are the
/// layer projection on the permanent, and the draw is a card that leaves the
/// library only because the enters-trigger resolved (CR 603.6a) — a creature
/// spell resolving touches no library by itself. Defender is the half a board
/// state cannot assume, so the untapped Elf beside it is the control: an
/// unmodified 1/1 is offered in the same attack declaration the Barrier must be
/// missing from (CR 702.3b).
#[test]
fn jungle_barrier_enters_drawing_a_card_and_can_never_attack() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(77, forest())
        .battlefield(
            0,
            &[forest(), forest(), island(), island(), quiet_creature()],
        )
        .hand(0, &[jungle_barrier()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Two Forests and two Islands pay {2}{G}{U}, and the Elf is named as the
    // printing kept back: it is the attacker the combat step below needs, and a
    // creature tapped for mana may not attack.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Forests and two Islands tapped, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, jungle_barrier());
    pass_until(&mut engine, stack_is_empty);

    let barrier = on_battlefield(&engine, p0, jungle_barrier()).expect("the Barrier resolved");
    assert_eq!(pt(&engine, barrier), (2, 6), "the body the card prints");
    assert!(
        keywords(&engine, barrier).contains(KeywordSet::DEFENDER),
        "\"Defender\" reaches the permanent"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"When this creature enters, draw a card\" — one card off the top of \
         the library, and a creature spell resolving draws nothing by itself"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the cast spent a card and the enters-trigger replaced it, so a \
         library that merely emptied would not satisfy the count above"
    );

    // The control the keyword needs: the offer exists because the Elf may
    // attack, and the Barrier is missing from that very list.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&elf),
        "an untapped 1/1 with no text of its own may attack: {attackers:?}"
    );
    assert!(
        !attackers.contains(&barrier),
        "\"This creature can't attack\" — its 2/6 is a real body and the \
         combat step still may not declare it: {attackers:?}"
    );
}
