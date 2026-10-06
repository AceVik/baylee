//! `cards/instants/mv_3/rescind.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rescind prints two lines and one board plays both: "{1}{U}{U} — Return
/// target permanent to its owner's hand" and its cycling, "{2}, Discard this
/// card: Draw a card." The bounce is read across the table, where "its
/// owner's hand" is the word under test — the Sol Ring that leaves goes to the
/// hand of the seat that owns it and not to the caster's, while the second
/// permanent nobody named never moves — and the cycling is read on the copy
/// still in hand, whose `{2}` comes out of the same pool the spell left behind.
/// Five Islands are exactly both printed costs, so the pool reads five, then
/// two, then nothing, and each half's price is a payment rather than a label.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn rescind_bounces_a_permanent_to_its_owners_hand_and_cycles_a_copy_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[rescind(), rescind()])
        // A permanent to bounce and a second one to leave alone, both on the
        // far side of the table: "target permanent" crosses it, "its owner's
        // hand" is the seat that owns it, and nothing claims the board.
        .battlefield(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before a single Island is tapped"
    );

    // `LegalActions` is filtered through `can_afford`, and that reads the pool
    // rather than the five untapped Islands. The spell has targets standing on
    // the table, so the only thing between the seat and the cast is mana.
    let card = in_hand(&engine, p0, rescind()).expect("the Rescind is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{1}}{{U}}{{U}}: {:?}",
        legal.castable
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Islands, five blue, and no other source on this board"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "with the mana floating the same card is castable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, rescind());
    let options = aim_at(&mut engine, p0, rock);
    assert!(
        options.contains(&rock) && options.contains(&elf),
        "\"target permanent\" is any permanent, on either side of the table: {options:?}"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the permanent the Rescind named left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, quiet_artifact()).is_some(),
        "\"to its owner's hand\": the Sol Ring goes back to the seat that owns it"
    );
    assert!(
        in_hand(&engine, p0, quiet_artifact()).is_none(),
        "and nowhere near the hand of the seat that cast the spell"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the permanent nobody named never moved"
    );
    assert!(
        in_graveyard(&engine, p0, rescind()).is_some(),
        "an instant that resolved is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{1}}{{U}}{{U}} is spent and exactly the {{2}} the cycling charges is left"
    );

    // The other printed line: "{2}, Discard this card: Draw a card." The second
    // copy is still in hand, and its activation is taken out of the offer
    // rather than guessed at — the card in hand is the source the engine names,
    // because that is the zone the ability lives in.
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == rescind()))
        })
        .expect("the Rescind left in hand offers its own cycling");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the two mana already floating pay for the cycling");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the cycling's {{2}} came out of the pool the Islands filled"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "and the discard pays for it, so the hand is the size it was"
    );
    assert_eq!(
        mine(&engine, p0, rescind(), Zone::Graveyard).len(),
        2,
        "one Rescind resolved and one was discarded to its own cycling"
    );
}
