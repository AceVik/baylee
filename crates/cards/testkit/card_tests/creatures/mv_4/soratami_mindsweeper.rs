//! `cards/creatures/mv_4/soratami_mindsweeper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Soratami Mindsweeper — {3}{U} for a 1/4 Moonfolk Wizard with flying — prints
/// "{2}, Return a land you control to its owner's hand: Target player mills two
/// cards." The price is three claims the card file cannot show at once: the {2}
/// out of a pool only the Islands filled, the land going to its *owner's* hand,
/// and the mill landing on the seat that was named while the other library
/// keeps every card it had. An Island of the opponent's is the same printing,
/// so a filter that had lost `YOUR_LAND` would be offered it and every count
/// below would still pass.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn soratami_mindsweeper_returns_its_own_land_to_mill_the_player_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 6])
        .battlefield(1, &[island()])
        .hand(0, &[soratami_mindsweeper()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let their_land = on_battlefield(&engine, p1, island()).expect("their Island is out");

    // Six Islands pay the {3}{U} and leave exactly the {2} the ability charges
    // floating beside it — CR 500.5 keeps the pool across the cast and this
    // whole scenario lives inside one main phase. It is also the only thing
    // `can_afford` reads, which is why the offer is claimed now.
    cast_from_hand(&mut engine, p0, soratami_mindsweeper());
    pass_until(&mut engine, stack_is_empty);
    let sweeper =
        on_battlefield(&engine, p0, soratami_mindsweeper()).expect("the Mindsweeper resolved");
    assert_eq!(pt(&engine, sweeper), (1, 4), "the body the card prints");
    assert!(
        keywords(&engine, sweeper).contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "six Islands less the {{3}}{{U}} the cast cost, and the {{2}} the \
         ability charges is exactly what is left"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(sweeper, 0)),
        "with {{2}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );

    let my_library = library_size(&engine, p0);
    let their_library = library_size(&engine, p1);
    let their_yard = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();

    activate(&mut engine, p0, soratami_mindsweeper(), 0);

    // One activation asks two questions — who is milled (CR 601.2c) and which
    // land is given up (CR 601.2h) — answered in whichever order they arrive.
    let mut named = false;
    let mut returned: Option<ObjectId> = None;
    for _ in 0..12 {
        if named && returned.is_some() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                player_options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat is the one that aims it");
                assert_eq!((min, max), (1, 1), "one player, and the ability asks once");
                assert!(
                    player_options.contains(&p0) && player_options.contains(&p1),
                    "\"target player\" reaches both seats of the table: {player_options:?}"
                );
                assert_eq!(
                    engine.state().players[0].mana_pool.total(),
                    2,
                    "CR 601.2c before CR 601.2h: the {{2}} is still floating while \
                     the question of who is milled stands"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .expect("the opponent seat was one of the targets it enumerated");
                named = true;
            }
            Pending::ChoosePlayer { player, options } => {
                assert_eq!(player, p0, "the activating seat is the one that aims it");
                assert!(
                    options.contains(&p0) && options.contains(&p1),
                    "\"target player\" reaches both seats of the table: {options:?}"
                );
                engine
                    .apply(p0, PlayerAction::ChoosePlayer(p1))
                    .expect("the opponent seat was one of the targets it enumerated");
                named = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat pays its own cost");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostReturn,
                    "a cost and not a search, which is all a client has to tell the two apart"
                );
                assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
                assert_eq!(
                    options.len(),
                    6,
                    "the six Islands this seat controls, and nothing else: {options:?}"
                );
                assert!(
                    !options.contains(&their_land),
                    "\"a land you control\": the Island across the table is the same \
                     printing and is not this seat's to give up: {options:?}"
                );
                assert!(
                    !options.contains(&sweeper),
                    "the Mindsweeper is a creature and no land: {options:?}"
                );
                let land = options[0];
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![land],
                        },
                    )
                    .expect("the land the question offered pays the cost");
                returned = Some(land);
            }
            Pending::Priority { player, .. } => {
                engine
                    .apply(player, PlayerAction::PassPriority)
                    .expect("passing priority is always legal");
            }
            other => panic!("unexpected while the Mindsweeper's ability resolves: {other:?}"),
        }
    }
    assert!(
        named,
        "\"target player mills two cards\" is a target choice"
    );
    let returned = returned.expect("the return-a-land cost asks which land is given up");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}} it charges came out of the pool"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&returned),
        "\"return a land you control to its owner's hand\": the very card the \
         question offered is in its owner's hand"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&returned),
        "and it left the battlefield rather than being copied into a hand"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&their_land),
        "the land the question declined never moved"
    );
    assert_eq!(
        library_size(&engine, p1),
        their_library - 2,
        "\"target player mills two cards\" — two off the top of the named player's library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        their_yard + 2,
        "and the two are in that player's graveyard, not merely gone from the library"
    );
    assert_eq!(
        library_size(&engine, p0),
        my_library,
        "\"target player\" is one seat: the seat that activated mills nothing"
    );
    assert!(
        on_battlefield(&engine, p0, soratami_mindsweeper()).is_some(),
        "the price was a land and no sacrifice, so the creature is still standing"
    );
}
