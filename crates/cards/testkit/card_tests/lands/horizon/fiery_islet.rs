//! `cards/lands/horizon/fiery_islet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fiery Islet is the Horizon cycle's blue-red land: "{T}, Pay 1 life: Add
/// {U} or {R}", and "{1}, {T}, Sacrifice Fiery Islet: Draw a card."
///
/// Each half taps the land, so one copy cannot take both in a turn — the
/// board holds one already out for the mana line and one played as a real
/// land drop, which is also what shows a Horizon land arriving untapped. The
/// life total is the reading that separates this from a plain dual land: the
/// pool gains one red *and* the controller drops to 19, and the second half
/// then spends that very mana to cash the other copy in for a card.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn fiery_islet_sells_one_life_for_colored_mana_and_itself_for_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[fiery_islet()])
        .hand(0, &[fiery_islet()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let out = on_battlefield(&engine, p0, fiery_islet()).expect("the first Islet is on the table");
    let dropped = play_land(&mut engine, p0, fiery_islet());
    assert_ne!(out, dropped, "the land drop is the second copy");
    assert!(
        !is_tapped(&engine, dropped),
        "a Horizon land enters untapped, so both of its lines are live this turn"
    );

    // "{T}, Pay 1 life: Add {U} or {R}." A mana ability a card prints is an
    // ordinary `(source, index)` entry in `abilities`, and its price is more
    // than its own tap — which is why it is pressed by hand and by index.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, index)| *index == 0 && *id == out)
        .expect("the mana ability is offered for a land and one point of life");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{U}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated names the color");
    assert_eq!(options.len(), 2, "two colors and no third: {options:?}");
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Red),
        "the two colors the land prints: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one tap, one mana"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"Pay 1 life\": a land that made red for nothing would still read 20"
    );
    assert!(
        is_tapped(&engine, out),
        "the tap was the other half of the price"
    );

    // "{1}, {T}, Sacrifice this land: Draw a card." The {1} is the red the
    // other Islet just made, so the line is affordable the moment it is
    // announced — which is exactly what the offer says.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, index)| *index == 1 && *id == dropped)
        .expect("one red in the pool pays the {1}, so the loot is offered");
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
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        on_battlefield(&engine, p0, fiery_islet()),
        Some(out),
        "only the copy that was sacrificed left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, fiery_islet()).is_some(),
        "\"Sacrifice this land\" — it goes to its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and the card it was cashed in for came off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "a draw that emptied the library without filling the hand would satisfy the count above"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}} came out of the pool the other Islet filled"
    );
}
