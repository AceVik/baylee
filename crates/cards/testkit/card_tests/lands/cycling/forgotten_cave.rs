//! `cards/lands/cycling/forgotten_cave.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Forgotten Cave prints three lines: it enters tapped, it taps for {R}, and
/// it has cycling {R} ({R}, Discard this card: Draw a card). The scenario has
/// to see all three, because the first is exactly what keeps the second out of
/// reach on the turn it arrives — a land that entered untapped would let the
/// mana ability be claimed immediately, and a `{T}` ability on a tapped land is
/// never even offered. So the arrival is read first (tapped, no mana ability on
/// offer), the untap step of the controller's next turn gives the `{T}` back,
/// and the cycling line is played out of hand, where its `ActivationZone::Hand`
/// puts it — costing {R} off the very land that just untapped, discarding
/// itself and drawing the top card of the library.
#[test]
fn forgotten_cave_enters_tapped_taps_for_red_and_cycles_itself_for_one_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // Two copies, because the test plays both printed halves and a card can
    // only be in one zone: the first is the land drop, the second stays in
    // hand for the cycling line below.
    let mut engine = Duel::new(881, forest())
        .hand(0, &[forgotten_cave(), forgotten_cave()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The land drop, on an otherwise empty board: it is the only permanent
    // this seat has, so everything read off it afterwards has no other source.
    let land = play_land(&mut engine, p0, forgotten_cave());
    assert!(entered_tapped(&engine, land), "\"This land enters tapped\"");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "expected priority after the land drop, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.mana_abilities.contains(&land) && !legal.abilities.iter().any(|(s, _)| *s == land),
        "a tapped land has no {{T}} to pay, so neither list carries it: {:?}",
        legal.mana_abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing was produced on the way in"
    );

    // Through the opponent's turn, so the controller's untap step really runs
    // (CR 502.3) and the land stands back up — `reach_main_phase` alone would
    // answer "already there" from the phase we are standing in.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the Cave's controller takes another turn"
    );
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran and the Cave is standing"
    );

    // {T}: Add {R}, read off the pool rather than off the offer alone.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one red off the Cave"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "and it is red, which is the color the card prints"
    );

    // Cycling {R}: the card is in hand, so the ability's `ActivationZone::Hand`
    // is where it is offered — and the {R} it charges is already floating.
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let cycled = in_hand(&engine, p0, forgotten_cave()).expect("the card is still in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let slot = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == cycled)
        .expect("cycling is offered from hand while its {R} is affordable");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: slot.0,
                ability_index: slot.1,
            },
        )
        .expect("the {R} in the pool pays the cycled cost");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, forgotten_cave()).is_some(),
        "\"Discard this card\" — cycling puts the card in the graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the discarded card is replaced by the drawn one"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and the draw took exactly one card off the top"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}} was spent, so cycling is a real payment and not a free draw"
    );
}
