//! `cards/lands/fetch/rocky_tar_pit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rocky Tar Pit prints two sentences: it enters tapped, and
/// `{T}, Sacrifice this land: Search your library for a Swamp or Mountain
/// card, put it onto the battlefield, then shuffle.`
///
/// The cost is the tap symbol and the sacrifice and no mana at all, so a
/// `legal.abilities` reading that omits the fetch the turn the land arrives is
/// about the tap alone — and the same reading a turn later, once the tapped
/// Forest beside it proves an untap step really ran, has it back. The fetch is
/// then played for real: the Swamps in hand and the one seeded into the
/// graveyard satisfy the printed filter just as well as the library's cards do
/// and are absent from the menu, the sacrificed land is in the graveyard, and
/// the card that was found arrives on the battlefield and not in hand.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn rocky_tar_pit_arrives_tapped_then_cracks_for_a_swamp_onto_the_battlefield() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // A library of Swamps: every card in it matches the filter the land
    // searches for, so the menu read below is the library and nothing but it.
    let mut engine = Duel::new(4021, swamp())
        .battlefield(0, &[forest()])
        .hand(0, &[rocky_tar_pit()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let ground = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    // A Swamp in the graveyard is a card the filter matches, so leaving it off
    // the menu is what "search your *library*" means.
    seed_graveyard(&mut engine, p0, 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "one Swamp in the graveyard, matching the filter the fetch prints"
    );

    // The turn's mana, spent now so that the Forest is a witness: it is back
    // up only if CR 502.3 ran over the turn in between.
    tap_all_mana_but(&mut engine, p0, Some(rocky_tar_pit()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "the Forest gave its one green"
    );

    let pit = play_land(&mut engine, p0, rocky_tar_pit());
    assert!(entered_tapped(&engine, pit), "the land enters tapped");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the land drop leaves the seat holding priority: {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.contains(&(pit, 0)),
        "a land that entered tapped has no {{T}} to pay its own cost with: {:?}",
        legal.abilities
    );

    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "and back around to p0");
    assert!(
        !is_tapped(&engine, ground) && !is_tapped(&engine, pit),
        "the untap step ran: the Forest tapped last turn and the land that \
         entered tapped are both standing again"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(pit, 0)),
        "and the fetch is offered now that its {{T}} is payable: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, rocky_tar_pit(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that cracked the land searches");
    assert_eq!(prompt, ChoicePrompt::SearchLibrary, "a library search");
    assert_eq!(
        options.len(),
        library_before,
        "every card in the library is a Swamp, so the whole library is on the \
         menu — while the Swamps in hand and the one in the graveyard, which \
         match the filter just as well, are not: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("the card the search offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, rocky_tar_pit()).is_none(),
        "\"Sacrifice this land\" is the other half of the cost"
    );
    assert!(
        in_graveyard(&engine, p0, rocky_tar_pit()).is_some(),
        "and the sacrificed land is in its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card left the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "and it did not go to hand: \"put it onto the battlefield\""
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, swamp()).len(),
        1,
        "it is on the battlefield as the Swamp it was"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2,
        "and the graveyard holds only the seeded Swamp and the sacrificed land"
    );
}
