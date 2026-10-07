//! `cards/lands/utility/boseiju_who_endures.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Boseiju, Who Endures is a Legendary Land that prints two implemented
/// things: "{T}: Add {G}", and "Channel — {1}{G}, Discard this card: Destroy
/// target artifact, enchantment, or nonbasic land an opponent controls."
///
/// One game reads both, because the land half pays for the channel half: the
/// first copy is played as the land drop and tapped for the green in the
/// cost, while the second copy stays in hand and is activated *from there*,
/// so the discard that leaves it in a graveyard is a cost being paid and not
/// a land drop being made.
///
/// The menu the activation offers is the whole grammar of the sentence. The
/// opponent's Sol Ring and their Badlands are on it; the basic Forest beside
/// them is not, which is the one thing "nonbasic land" is worth; and neither
/// the Sol Ring nor the nonbasic land on this seat's own side is, because "an
/// opponent controls" qualifies all three nouns. (It was read as sitting on
/// the land alone until the search half was built, and this test asserted
/// the seat's own Sol Ring onto the menu.)
///
/// Then the victim's half: "That player may search their library for a land
/// card with a basic land type, put it onto the battlefield, then shuffle."
/// The question goes to the Sol Ring's controller, over their own library,
/// and the Forest they take enters untapped. With no legendary creature on
/// this seat's side the channel costs its whole printed `{1}{G}`; the
/// reduction is the two tests after this one.
#[test]
#[allow(clippy::too_many_lines)] // both printed lines, and the second one's menu
fn boseiju_taps_for_green_and_channels_itself_away_for_an_artifact_or_a_nonbasic_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(29, forest())
        .battlefield(0, &[forest(), quiet_artifact()])
        .hand(0, &[boseiju_who_endures(), boseiju_who_endures()])
        .battlefield(1, &[quiet_artifact(), badlands(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The land half, first. A land with no basic land type is no CR 305.6
    // source: "{T}: Add {G}" is a printed ability, offered as (source, 0) in
    // `abilities` and pressed by index rather than by `tap_all_mana`.
    play_land(&mut engine, p0, boseiju_who_endures());
    activate(&mut engine, p0, boseiju_who_endures(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "CR 605.3b: the green is in the pool the moment the land is tapped, \
         with nothing on the stack"
    );

    // The rest of {1}{G}, in the pool *before* the activation is asked for:
    // an activation this board cannot pay for is one the engine never offers.
    tap_all_mana(&mut engine, p0);
    let pool_before = engine.state().players[0].mana_pool.total();
    assert!(
        pool_before >= 2,
        "the channel's {{1}}{{G}} has to be covered by the pool: {pool_before}"
    );

    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring stands");
    let their_land = on_battlefield(&engine, p1, badlands()).expect("their Badlands stands");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their basic Forest stands");
    let my_rock = on_battlefield(&engine, p0, quiet_artifact()).expect("my own Sol Ring stands");
    let my_boseiju =
        on_battlefield(&engine, p0, boseiju_who_endures()).expect("the played copy stands");

    // Ability 1 is the channel; ability 0 is the mana ability the copy on the
    // battlefield is already using.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.iter().any(|(source, index)| *index == 1
            && engine
                .state()
                .object(*source)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == boseiju_who_endures()))),
        "the card still in hand offers its channel: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, boseiju_who_endures(), 1);

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the channel destroys one target and asks which, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated names the target");
    assert!(
        options.contains(&rock),
        "\"target artifact\" reaches across the table: {options:?}"
    );
    assert!(
        options.contains(&their_land),
        "\"nonbasic land an opponent controls\": {options:?}"
    );
    assert!(
        !options.contains(&my_rock),
        "\"an opponent controls\" qualifies the artifact too, so this seat's \
         own Sol Ring is no target: {options:?}"
    );
    assert!(
        !options.contains(&their_forest),
        "and a basic land is not a nonbasic land — the control for the word \
         \"nonbasic\", on a side of the table the controller clause cannot \
         explain away: {options:?}"
    );
    assert!(
        !options.contains(&my_boseiju),
        "while this land *is* nonbasic and is not an opponent's, so the two \
         clauses are read separately: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "their artifact and their Badlands are the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("the Sol Ring was one of the options");
    let Pending::ChooseCards {
        player,
        options,
        min,
        ..
    } = pass_to_card_choice(&mut engine)
    else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(player, p1, "\"that player may search their library\"");
    assert_eq!(min, 0, "may");
    let theirs = engine.state().zones.list(ZoneLocation::Library(p1)).clone();
    assert!(
        !options.is_empty() && options.iter().all(|o| theirs.contains(o)),
        "the search is of the victim's own library: {options:?}"
    );
    let found = options[0];
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&found)
            && engine
                .state()
                .object(found)
                .is_some_and(|o| o.controller == p1),
        "the Forest they found is on the battlefield under their control"
    );
    assert!(
        !is_tapped(&engine, found),
        "\"put it onto the battlefield\", untapped"
    );

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "\"destroy target artifact\": the Sol Ring is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, badlands()).is_some(),
        "and only the permanent that was named: the Badlands still stands"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "and nothing of this seat's was touched"
    );
    assert!(
        in_graveyard(&engine, p0, boseiju_who_endures()).is_some(),
        "\"Discard this card\" as the cost: the copy activated from hand is in \
         its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, boseiju_who_endures()).is_some(),
        "while the copy played as a land is still on the battlefield — the \
         discard took the card that was activated and not the other printing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        pool_before - 2,
        "{{1}}{{G}} left the pool, so the ability was paid for and not free"
    );
}

/// "This ability costs {1} less to activate for each legendary creature you
/// control." One Forest and one legend of this seat's: the `{1}{G}` channel
/// costs `{G}`, so it is offered off a single green in the pool, activates,
/// spends exactly that green, and destroys the opponent's artifact. Before
/// the reduction was read the channel cost its printed `{1}{G}` and was never
/// offered on this board.
#[test]
fn boseiju_channel_costs_one_less_for_each_legendary_creature_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(29, forest())
        .battlefield(0, &[forest(), thrun_the_last_troll()])
        .hand(0, &[boseiju_who_endures()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1, "one green");
    assert!(
        boseiju_channel_offered(&engine, p0),
        "{{1}}{{G}} less {{1}} for Thrun is {{G}}, which the pool holds"
    );
    let rock = on_battlefield(&engine, p1, quiet_artifact()).unwrap();
    activate(&mut engine, p0, boseiju_who_endures(), 1);
    aim_at(&mut engine, p0, rock);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the green paid the whole reduced cost"
    );
    let Pending::ChooseCards { player, .. } = pass_to_card_choice(&mut engine) else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(player, p1, "that player may search");
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, quiet_artifact()).is_some());
    assert!(in_graveyard(&engine, p0, boseiju_who_endures()).is_some());
}

/// The reduction counts only this seat's legends and takes only generic
/// mana (CR 118.7a): an opponent's legendary creature leaves the channel at
/// `{1}{G}`, so one green does not reach it; and two legends of this seat's
/// take off the `{1}` but never the `{G}`, so with an empty pool it is still
/// not offered.
#[test]
fn boseiju_channel_counts_only_your_legends_and_never_its_green() {
    let p0 = PlayerId::new(0);
    let mut theirs = Duel::new(29, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[boseiju_who_endures()])
        .battlefield(1, &[quiet_artifact(), thrun_the_last_troll()])
        .start();
    keep_mulligans(&mut theirs);
    assert!(walk_to_own_main(&mut theirs, p0));
    tap_all_mana(&mut theirs, p0);
    assert!(
        !boseiju_channel_offered(&theirs, p0),
        "\"you control\": the opponent's Thrun takes nothing off"
    );

    let mut mine = Duel::new(29, forest())
        .battlefield(0, &[thrun_the_last_troll(), vendilion_clique()])
        .hand(0, &[boseiju_who_endures()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut mine);
    assert!(walk_to_own_main(&mut mine, p0));
    assert_eq!(mine.state().players[0].mana_pool.total(), 0);
    assert!(
        !boseiju_channel_offered(&mine, p0),
        "two legends take the {{1}} off and leave the {{G}}"
    );
}
