//! `cards/sorceries/mv_4/lay_waste.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lay Waste prints two halves and this plays both off one board: "{3}{R}
/// Sorcery — Destroy target land", and "Cycling {2} ({2}, Discard this card:
/// Draw a card.)" on a card still in hand. The filter is the half a card file
/// cannot show — "target land" names no side of the table, so the offer has to
/// carry the opponent's Forest *and* this seat's own Mountain while the Elf
/// beside them is no land at all. The second copy is cycled out of what the
/// cast left in the pool, and the discard-then-draw is read as one move in each
/// of the zones it touches: the card into the graveyard, a card off the top of
/// the library into the hand.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn lay_waste_destroys_any_land_and_cycles_the_other_copy_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[forest(), llanowar_elves()])
        .hand(0, &[lay_waste(), lay_waste()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let my_mountain = on_battlefield(&engine, p0, mountain()).expect("my Mountain is out");

    // Six Mountains into the pool: the cast's {3}{R} is four of them and the
    // cycle's {2} is what is left beside it. `can_afford` reads the pool rather
    // than the untapped lands, so the cast is claimed after the tapping.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Mountains tapped, and nothing else on this board makes mana for p0"
    );
    cast_with_floating(&mut engine, p0, lay_waste());

    let options = pass_until_targets(&mut engine, p0);
    // Seven lands stand on the table — p0's six Mountains, tapped for the pool
    // but no less lands for it, and p1's one Forest — and the menu is all of
    // them and nothing else.
    let mut lands = lands_of(&engine, p0);
    lands.extend(lands_of(&engine, p1));
    assert_eq!(lands.len(), 7, "six Mountains and a Forest");
    assert_eq!(
        options.len(),
        lands.len(),
        "the seven lands on the table and no creature: {options:?}"
    );
    for land in &lands {
        assert!(
            options.contains(land),
            "every land is on the menu, tapped or not and on either side: {options:?}"
        );
    }
    assert!(
        options.contains(&their_land),
        "the land across the table is a legal target: {options:?}"
    );
    assert!(
        options.contains(&my_mountain),
        "\"target land\" names no side of the table, so this seat's own \
         Mountain is on the menu too: {options:?}"
    );
    assert!(
        !options.contains(&their_elf),
        "an Elf is a creature and no land: {options:?}"
    );
    // CR 601.2c takes the target before CR 601.2h pays, so while this question
    // stands the mana is still in the pool and the land is still standing.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "the cost is the last step of the cast, so nothing is spent yet"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and the land the spell will destroy is still on the battlefield"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_land],
            },
        )
        .expect("their Forest was one of the options the question enumerated");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{3}}{{R}} is paid and two mana are left for the cycle"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "\"Destroy target land\": the named Forest is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "and it left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, mountain()).is_some(),
        "the land the spell did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and neither did the creature the filter declined"
    );

    // The other half of the card: the second copy is still in hand, and the two
    // mana the cast left behind are exactly the cycle's {2}.
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let cards_before = mine(&engine, p0, lay_waste(), Zone::Graveyard).len();

    activate(&mut engine, p0, lay_waste(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} came out of the pool the cast filled"
    );
    assert!(
        !stack_is_empty(&engine),
        "cycling is no mana ability, so it uses the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        mine(&engine, p0, lay_waste(), Zone::Graveyard).len(),
        cards_before + 1,
        "the discarded copy joined the one the cast spent"
    );
    assert!(
        in_hand(&engine, p0, lay_waste()).is_none(),
        "and no copy is left in hand, so the cycle really did discard one"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the discard and the draw cancel out, so a library that merely emptied \
         would not satisfy the count above"
    );
}
