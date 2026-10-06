//! `cards/lands/cycling/drifting_meadow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Drifting Meadow prints three sentences — enters tapped, `{T}: Add {W}`, and
/// cycling `{2}` — and one board plays all three. It is *played* as the land
/// drop rather than seeded with `starting_battlefield`, which places a
/// permanent without an entry replacement and would have arrived untapped; the
/// turn it arrives its `{T}` is offered nowhere, and in its controller's next
/// main phase it is standing again and adds the one white this board's white
/// mana consists of. Cycling lives on the card in *hand*, so it is pressed on a
/// second copy: the card is discarded as the cost while the ability is still on
/// the stack, and a card arrives when it resolves — which is what tells cycling
/// from a card that merely changed zones.
#[test]
#[allow(clippy::too_many_lines)] // one card in both zones, and the entry clause read on either side of a turn
fn drifting_meadow_enters_tapped_cycles_for_two_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[drifting_meadow(), drifting_meadow()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Mana into the pool before any offer is read: `legal` is filtered on
    // `can_afford`, so cycling with {2} left to pay would be missing for a
    // reason this test is not about.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests, and the Meadow is still in hand where it makes nothing"
    );

    let land = play_land(&mut engine, p0, drifting_meadow());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — the entry replacement, not a placement"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the land drop leaves the seat holding priority, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land)
            && !legal.mana_abilities.contains(&land),
        "and a land that arrived tapped pays no {{T}}: nothing on it is offered"
    );

    // Cycling {2} off the copy still in hand. Ability 1 is the cycling; ability
    // 0 is the mana ability, which lives on the battlefield and nowhere else.
    let cycler = in_hand(&engine, p0, drifting_meadow()).expect("the second Meadow is in hand");
    let library_before = library_size(&engine, p0);
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .expect("p0 has a library");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(cycler, 1)),
        "cycling is an ability of the card in *hand*: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, drifting_meadow(), 1);
    assert!(
        in_graveyard(&engine, p0, drifting_meadow()).is_some(),
        "\"Discard this card\" is the cost, so it is paid as the ability is \
         announced and the card is already in the graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "cycling is an activated ability and no mana ability: the draw is still \
         a stack object"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the {{2}} came out of the pool"
    );

    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the cycling ability resolves back to a quiet priority"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\" — one card off the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and it is the card that was on top, which is what tells a draw from a \
         card that merely left the library"
    );
    assert!(
        in_hand(&engine, p0, drifting_meadow()).is_none(),
        "the cycled card stayed in the graveyard: nothing put it back in hand"
    );

    // A whole turn cycle, then the same land read again — the other side of
    // the eligibility question, on an offer the untap step has rebuilt.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood it back up, which the reading above could not see"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "so the mana ability it prints is offered again: {:?}",
        legal.abilities
    );
    let white_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::White);
    activate(&mut engine, p0, drifting_meadow(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        white_before + 1,
        "{{T}}: Add {{W}}, and white is the only thing it can add"
    );
    assert!(
        is_tapped(&engine, land),
        "the tap is the whole of the price"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
}
