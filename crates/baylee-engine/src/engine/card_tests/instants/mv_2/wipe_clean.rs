//! `cards/instants/mv_2/wipe_clean.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wipe Clean prints two lines and both want a card in hand, so the test plays
/// them with two copies in one main phase: {1}{W} exiles an enchantment, and
/// {3} cycles the other copy away for a card. The board carries both
/// enchantments plus a creature and an artifact, so the target question is a
/// filter rather than a count — "target enchantment" reaches either side of the
/// table and neither of the other two permanents. Five Plains pay for both
/// lines out of one pool, which makes the emptied pool a price, and the cycled
/// copy landing in the graveyard beside the resolved one is the discard
/// cycling charges.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn wipe_clean_exiles_an_enchantment_and_cycles_its_other_copy_away_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                exploration(),
            ],
        )
        .hand(0, &[wipe_clean(), wipe_clean()])
        .battlefield(
            1,
            &[their_enchantment(), llanowar_elves(), quiet_artifact()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_enchantment =
        on_battlefield(&engine, p0, exploration()).expect("my own enchantment is out");
    let theirs =
        on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    // How many copies of this printing are in p0's graveyard, which is where
    // both halves of the card are read: a resolved instant lands there and a
    // discarded card is put there.
    let in_yard = |engine: &Engine<RegistryLookup>| {
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .iter()
            .filter(|id| {
                engine
                    .state()
                    .object(**id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == wipe_clean()))
            })
            .count()
    };
    assert_eq!(
        in_yard(&engine),
        0,
        "nothing has been cast or discarded yet"
    );

    // Mana first: five Plains are the whole board and five white is what the
    // two printed lines cost together.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Plains tapped and five white floating — the Exploration beside \
         them makes no mana"
    );
    cast_with_floating(&mut engine, p0, wipe_clean());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"exile target enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!(
        (min, max),
        (1, 1),
        "one enchantment, and the spell asks once"
    );
    assert!(
        options.contains(&my_enchantment) && options.contains(&theirs),
        "\"target enchantment\" is any enchantment, on either side of the \
         table: {options:?}"
    );
    assert!(
        !options.contains(&elf) && !options.contains(&rock),
        "a creature and an artifact are no enchantments, so the filter is read \
         and not skipped: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the enchantment across the table was one of the options");
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the exile resolves and the board comes back to a quiet priority"
    );

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the enchantment the spell named left the battlefield"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&theirs),
        "\"exile\" is exile and not the graveyard: the card is in its owner's \
         exile zone"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&my_enchantment),
        "the enchantment on this side of the table was never named"
    );
    assert_eq!(
        in_yard(&engine),
        1,
        "and the spell itself is a resolved instant in its owner's graveyard"
    );

    // The second printed line. Its price is {3} and the card, and the three
    // white the cast left behind are exactly that {3}.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "five white less the {{1}}{{W}} the spell cost"
    );
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let cycler = in_hand(&engine, p0, wipe_clean()).expect("the second copy is still in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.iter().any(|(src, _)| *src == cycler),
        "cycling is taken from hand, so the card itself is the source of the \
         ability being offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, wipe_clean(), 0);
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the cycled ability resolves and the seat is back at a quiet priority"
    );

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}} came out of the pool the cast left it in"
    );
    assert_eq!(
        in_yard(&engine),
        2,
        "discarding the card is cycling's own price, so the cycled copy joined \
         the resolved one in the graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the cycled card left the hand and the drawn one replaced it, so the \
         count is unchanged — and only a real draw holds it there"
    );
}
