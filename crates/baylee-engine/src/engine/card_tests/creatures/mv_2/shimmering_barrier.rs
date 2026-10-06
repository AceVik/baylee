//! `cards/creatures/mv_2/shimmering_barrier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shimmering Barrier is a {1}{W} 1/3 Wall printing three lines: Defender,
/// first strike, and cycling {2}. This scenario plays all three in one main
/// phase — the first copy is cast onto the table, where its body and both
/// keywords are read off the projected characteristics and Defender is
/// checked where it actually bites, in the attack declaration against an
/// untapped Elf that the same offer does name; the second copy is cycled
/// *out of the hand*, which is the zone the ability lives in, for {2} out of
/// a pool the four Plains actually filled. The Elf is kept back from the
/// mana so that "may not attack" is a claim about Defender and not about a
/// creature that tapped for its own cast.
#[test]
fn shimmering_barrier_stands_with_defender_and_cycles_itself_away_for_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), quiet_creature()],
        )
        .hand(0, &[shimmering_barrier(), shimmering_barrier()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Four Plains and the Elf left standing: it is the control in the attack
    // declaration below, and a creature tapped for mana may not attack.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Plains, and the Elf contributed nothing"
    );

    // {1}{W} for the first copy.
    cast_with_floating(&mut engine, p0, shimmering_barrier());
    pass_until(&mut engine, stack_is_empty);
    let wall = on_battlefield(&engine, p0, shimmering_barrier()).expect("the Wall resolved");
    assert_eq!(pt(&engine, wall), (1, 3), "the body the card prints");
    let projected = keywords(&engine, wall);
    assert!(
        projected.contains(KeywordSet::DEFENDER),
        "Defender reaches the permanent"
    );
    assert!(
        projected.contains(KeywordSet::FIRST_STRIKE),
        "and so does first strike"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{1}}{{W}} came out of the pool and the {{2}} for the cycling is left"
    );

    // Cycling {2}: the ability lives in the hand, so it is the *other* copy
    // that is activated, and its price is the mana plus the card itself.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&engine, p0);
    activate(&mut engine, p0, shimmering_barrier(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, shimmering_barrier()).is_some(),
        "\"Discard this card\" is part of the cost, so the card is in its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\" — one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card discarded and one drawn: a cycled card is replaced, not lost"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{2}} came out of the pool"
    );

    // Defender where it bites, against the Elf that the same offer names.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    let elves = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is out");
    assert!(
        attackers.contains(&elves),
        "an untapped 1/1 with no text of its own may attack: {attackers:?}"
    );
    assert!(
        !attackers.contains(&wall),
        "\"This creature can't attack\" — the Wall is on the table with a \
         body and no summoning sickness and is still not offered: {attackers:?}"
    );
}
