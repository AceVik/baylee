//! `cards/instants/mv_3/realms_uncharted.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Realms Uncharted: "Search your library for up to four land cards with
/// different names and reveal them. An opponent chooses two of those cards.
/// Put the chosen cards into your graveyard and the rest into your hand."
///
/// Five names in the library (the Forests are one name, however many), four
/// found, and the opponent — not the caster — picks the two for the
/// graveyard.
#[test]
fn realms_uncharted_lets_the_opponent_bin_two_of_four_lands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(
            0,
            &[realms_uncharted(), island(), plains(), swamp(), mountain()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    hide_in_library(&mut engine, p0, &[island(), plains(), swamp(), mountain()]);
    let journal_from = engine.state().journal.len();
    let found = realms_search(&mut engine, p0, 4);

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the opponent's choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p1, "an opponent chooses");
    assert_eq!(options, found, "among the four found");
    assert_eq!((min, max), (2, 2), "exactly two");
    assert_eq!(prompt, ChoicePrompt::PutIntoGraveyard);
    assert!(
        engine.state().journal.entries()[journal_from..]
            .iter()
            .any(|e| matches!(&e.event, GameEvent::Revealed { cards, .. } if cards == &found)),
        "\"and reveal them\" — before the opponent chooses"
    );
    let binned = found[..2].to_vec();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: binned.clone(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    for card in &found {
        let zone = engine
            .state()
            .object(*card)
            .and_then(|o| o.zone_owner.map(|_| o.zone));
        let expected = if binned.contains(card) {
            crate::zone::Zone::Graveyard
        } else {
            crate::zone::Zone::Hand
        };
        assert_eq!(
            zone,
            Some(expected),
            "chosen to the graveyard, the rest to hand"
        );
    }
}

/// Two found are two chosen: nothing to decide, both go to the graveyard.
#[test]
fn realms_uncharted_bins_everything_when_two_or_fewer_are_found() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[realms_uncharted(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    hide_in_library(&mut engine, p0, &[island()]);
    let found = realms_search(&mut engine, p0, 2);
    pass_until(&mut engine, stack_is_empty);
    assert!(!matches!(engine.pending(), Pending::ChooseCards { .. }));
    for card in &found {
        assert_eq!(
            engine.state().object(*card).map(|o| o.zone),
            Some(crate::zone::Zone::Graveyard)
        );
    }
}

/// At a table of three the caster names which opponent chooses, and that
/// opponent is the one asked.
#[test]
fn realms_uncharted_at_a_table_lets_the_caster_name_the_opponent() {
    let (p0, p2) = (PlayerId::new(0), PlayerId::new(2));
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[realms_uncharted(), island(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    hide_in_library(&mut engine, p0, &[island(), plains()]);
    let found = realms_search(&mut engine, p0, 3);
    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!(
            "expected the caster to name an opponent, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert_eq!(options.len(), 2, "either opponent");
    engine.apply(p0, PlayerAction::ChoosePlayer(p2)).unwrap();
    let Pending::ChooseCards { player, .. } = engine.pending().clone() else {
        panic!(
            "expected the named opponent's choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p2, "the named opponent chooses");
    engine
        .apply(
            p2,
            PlayerAction::ChooseObjects {
                objects: found[..2].to_vec(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(found[2]).map(|o| o.zone),
        Some(crate::zone::Zone::Hand)
    );
}
