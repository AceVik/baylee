//! `cards/lands/cycling/tranquil_thicket.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tranquil Thicket prints three things: it enters tapped, it taps for {G},
/// and it cycles for {G} out of hand. The land is played rather than seeded
/// onto the battlefield, because only a real land drop runs the replacement
/// effect that taps it (CR 614.1c) — a board built by `starting_battlefield`
/// arrives untapped whatever the card says, and the first assertion here
/// would measure nothing. The two copies in hand are the whole test: one
/// arrives tapped and offers nothing at all — in either list a mana source
/// can hide in — until it untaps, and the other is the cycling card, paid for
/// out of the Forest beside it and read afterwards in the graveyard with a
/// card drawn off the library.
#[test]
fn tranquil_thicket_enters_tapped_taps_for_green_and_cycles_itself_into_the_graveyard() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(7331, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[tranquil_thicket(), tranquil_thicket()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The land drop, and not a placement: "enters tapped" is a replacement
    // effect, so it only runs on a real entry.
    let played = play_land(&mut engine, p0, tranquil_thicket());
    assert!(
        entered_tapped(&engine, played),
        "\"This land enters tapped\" — the printed line, on a land that was played"
    );

    // Tapped means no mana and no ability, in both of the lists a mana source
    // can hide in: this land is no basic land type, so its `{{T}}: Add {{G}}`
    // is an ordinary `(source, index)` entry and never the CR 305.6 shortcut.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&played)
            && !legal.abilities.iter().any(|(src, _)| *src == played),
        "a tapped land is offered by neither list: {:?}",
        legal.abilities
    );

    // Cycling {G} out of hand. The Forest pays, and the whole price is the
    // one green it makes.
    tap_all_mana_but(&mut engine, p0, Some(tranquil_thicket()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the one Forest, which is the whole price of a cycle"
    );
    let library_before = library_size(&engine, p0);
    let graveyard_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();

    // Ability 1 is the cycling line; ability 0 is the mana ability the card
    // prints, unreachable on the copy the land drop tapped.
    activate(&mut engine, p0, tranquil_thicket(), 1);
    assert!(
        in_graveyard(&engine, p0, tranquil_thicket()).is_some(),
        "the card discards itself as the cost, before the ability resolves (CR 601.2h)"
    );
    assert!(
        !stack_is_empty(&engine),
        "cycling is no mana ability: it uses the stack"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        graveyard_before + 1,
        "one card reached the graveyard, and it is the Thicket"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and the draw came off the top of the library"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{G}} the Forest made was the cycle's cost and nothing is left over"
    );
    assert!(
        on_battlefield(&engine, p0, tranquil_thicket()).is_some(),
        "the copy that was played is still on the battlefield: one card cycled"
    );

    // The untap step is the other half of "enters tapped", and the reason the
    // mana line has to be read a turn later: until then the permanent has a
    // `{{T}}` ability that is not even offered.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, played),
        "the untap step stood it back up"
    );

    activate(&mut engine, p0, tranquil_thicket(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "{{T}}: Add {{G}}, off the land itself — the Forest beside it is untouched"
    );
    assert!(is_tapped(&engine, played), "which tapped the land");
}
