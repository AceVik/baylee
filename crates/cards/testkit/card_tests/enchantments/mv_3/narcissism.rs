//! `cards/enchantments/mv_3/narcissism.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Narcissism — {2}{G} Enchantment: "{G}, Discard a card: Target creature gets
/// +2/+2 until end of turn" and "{G}, Sacrifice this enchantment: Target
/// creature gets +2/+2 until end of turn."
///
/// The two printed lines differ in one word of their price, so one board pays
/// both inside a single main phase: the discard line leaves the enchantment
/// standing with a card gone out of the hand, and the sacrifice line then takes
/// the enchantment itself to its owner's graveyard — which is the only thing
/// that tells the second line from the first, since the pump they hand out is
/// identical and stacks. The creature aimed at is read beside an Elf across the
/// table, so `Filter::CREATURE` is shown to reach either side of the board
/// rather than being assumed, and a turn boundary afterwards shows that the
/// printed "until end of turn" is a duration and not a body.
#[test]
#[allow(clippy::too_many_lines)] // one card, both printed prices, and the pump read three times
fn narcissism_pumps_for_a_discarded_card_and_then_for_the_enchantment_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[narcissism(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "a printed 1/1 before either pump is aimed at it"
    );

    // Six Forests into the pool and the Elf kept back: {2}{G} brings the
    // enchantment to the table and leaves three green, which is the {G} of each
    // printed line. Keeping the Elf out of the tapping is what leaves the
    // creature the pumps are about to be aimed at in the state the assertions
    // below find it in.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests tapped and nothing off the Elf"
    );
    cast_with_floating(&mut engine, p0, narcissism());
    pass_until(&mut engine, stack_is_empty);

    let card = on_battlefield(&engine, p0, narcissism()).expect("the enchantment resolved");
    assert!(
        types(&engine, card).contains(TypeSet::ENCHANTMENT),
        "it is the enchantment the card prints"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{2}}{{G}} is spent and three green are left for the two lines"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(card, 0)) && legal.abilities.contains(&(card, 1)),
        "both printed lines are offered while {{G}} is in the pool and a card \
         is in hand: {:?}",
        legal.abilities
    );

    // Ability 0: "{G}, Discard a card: Target creature gets +2/+2 …". The
    // target is named first (CR 601.2c) and the price is the last step of the
    // activation (CR 601.2h), so while the question stands the pool is
    // untouched and the hand is still full.
    let fodder = in_hand(&engine, p0, giant_growth()).expect("the card to discard is in hand");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, narcissism(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one target, and the ability asks once");
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&card),
        "the enchantment is no creature, so it is not a legal target for its own \
         ability: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the cost is the last step of the activation, so the {{G}} is still floating"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "and nothing has left the hand yet, for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf was one of the options it enumerated");

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the discard is a cost and is asked before it is paid: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostDiscard,
        "a cost and not an effect, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert!(
        options.len() == hand_before && options.contains(&fodder),
        "`Discard a card` names no filter, so the whole hand is the menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("a card the cost's own menu offered pays it");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{G}} came out of the pool"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "and the card left the hand"
    );
    assert!(
        in_graveyard(&engine, p0, giant_growth()).is_some(),
        "a discarded card goes to its owner's graveyard"
    );
    assert!(
        in_hand(&engine, p0, giant_growth()).is_none(),
        "and it is not still in the hand it was discarded from"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is waiting on the stack"
    );
    assert!(
        on_battlefield(&engine, p0, narcissism()).is_some(),
        "the discard line costs a card and leaves the enchantment where it is"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, host),
        (3, 3),
        "+2/+2 on the creature the ability named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for the same card standing across the table"
    );

    // Ability 1: "{G}, Sacrifice this enchantment: …". The same target and the
    // same pump, and the only thing that differs is which price is taken.
    activate(&mut engine, p0, narcissism(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims the second line too");
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "the second line carries the same target filter as the first: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf was one of the options it enumerated");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{G}} came out of the pool"
    );
    assert!(
        on_battlefield(&engine, p0, narcissism()).is_none(),
        "\"Sacrifice this enchantment\" takes the card the ability is printed on"
    );
    assert!(
        in_graveyard(&engine, p0, narcissism()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "and the pump is still no mana ability"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, host),
        (5, 5),
        "the second +2/+2 stacks on the first, which is what says both printed \
         lines resolved rather than one resolving twice"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and never across the table"
    );

    // "until end of turn": one turn boundary later the Elf is a printed 1/1
    // again, so both grants were durations and not bodies the board keeps.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "both grants lasted the turn they were made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature is still standing, so the pump left rather than the creature"
    );
}
