//! `cards/sorceries/mv_2/nylea_s_intervention.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nylea's Intervention, first mode: "Search your library for up to X land
/// cards, reveal them, put them into your hand, then shuffle." X = 2 is a
/// search for at most two, which may settle for fewer; the two found are
/// shown to the table and go to the hand.
#[test]
fn nyleas_intervention_fetches_up_to_x_lands_into_the_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[nyleas_intervention()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_nyleas(&mut engine, p0, 0, 2);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = pass_to_card_choice(&mut engine)
    else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    assert_eq!((min, max), (0, 2), "\"up to X\" with X = 2");
    let found = options[..2].to_vec();
    let journal_from = engine.state().journal.len();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: found.clone(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    for card in &found {
        assert!(
            engine
                .state()
                .zones
                .list(ZoneLocation::Hand(p0))
                .contains(card),
            "a found land is in the hand"
        );
    }
    assert!(
        engine.state().journal.entries()[journal_from..]
            .iter()
            .any(|e| matches!(
                &e.event,
                GameEvent::Revealed { cards, .. } if cards == &found
            )),
        "\"reveal them\""
    );
}

/// With X = 0 the search finds nothing and asks nobody.
#[test]
fn nyleas_intervention_for_x_zero_asks_nothing() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[nyleas_intervention()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_nyleas(&mut engine, p0, 0, 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(!matches!(engine.pending(), Pending::ChooseCards { .. }));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand - 1,
        "nothing came to the hand"
    );
}

/// Second mode: "Nylea's Intervention deals twice X damage to each creature
/// with flying." X = 2 is four damage: the Air Elemental (4/4, flying) dies
/// and the Llanowar Elves, which does not fly, is not dealt any.
#[test]
fn nyleas_intervention_deals_twice_x_to_each_flier() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .battlefield(1, &[air_elemental(), llanowar_elves()])
        .hand(0, &[nyleas_intervention()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    cast_nyleas(&mut engine, p0, 1, 2);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, air_elemental()).is_some(),
        "four damage kills the 4/4 flier"
    );
    assert_eq!(
        engine.state().object(elves).map(|o| o.damage),
        Some(0),
        "a creature without flying is dealt nothing"
    );
}
