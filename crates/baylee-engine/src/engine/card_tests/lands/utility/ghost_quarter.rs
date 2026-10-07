//! `cards/lands/utility/ghost_quarter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ghost Quarter prints two lines: "{T}: Add {C}" and "{T}, Sacrifice this
/// land: Destroy target land. Its controller may search their library for a
/// basic land card, put it onto the battlefield, then shuffle."
///
/// One play reads all of it. The target is chosen before the cost is paid
/// (CR 601.2c, then 601.2h), so with the target question open the Quarter is
/// still standing and untapped — and the filter prints no controller, so the
/// opponent's Badlands is offered beside my own Forest while only the named
/// one dies. The search afterwards goes to the *destroyed land's* controller
/// and not to the seat that fired the ability, which is the whole difference
/// between `PlayerRel::ControllerOfTarget` and a search for me.
///
/// The basic land's arrival is asserted as a position, not as a tap state:
/// that it enters untapped is the clause the card def records as the
/// `Coverage::Partial` gap and is deliberately not pinned here.
#[test]
#[allow(clippy::too_many_lines)] // one land, played through every clause it prints
fn ghost_quarter_sacrifices_itself_to_destroy_a_land_and_its_controller_gets_the_search() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[ghost_quarter()])
        .battlefield(1, &[badlands()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // A real land drop rather than a seeded permanent, so the Quarter is the
    // permanent the rules give it — and it may tap the turn it arrives,
    // because summoning sickness is a creature's problem (CR 302.6).
    let quarter = play_land(&mut engine, p0, ghost_quarter());
    let my_forest = on_battlefield(&engine, p0, forest()).expect("my Forest is out");
    let their_land = on_battlefield(&engine, p1, badlands()).expect("their Badlands is out");

    // Ability 0 is the printed "{T}: Add {C}"; ability 1 is the line above.
    activate(&mut engine, p0, ghost_quarter(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("\"target land\" is a target, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&their_land),
        "\"target land\" reaches across the table: {options:?}"
    );
    assert!(
        options.contains(&my_forest),
        "and the filter prints no controller, so my own Forest is on the menu too: {options:?}"
    );
    // CR 601.2c before CR 601.2h: neither half of the price has been paid
    // while the target is still being chosen.
    assert!(
        on_battlefield(&engine, p0, ghost_quarter()).is_some(),
        "the sacrifice is a cost and not a condition of aiming"
    );
    assert!(
        !is_tapped(&engine, quarter),
        "and the tap has not happened either"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_land],
            },
        )
        .expect("the Badlands was one of the lands it offered");

    // The ability resolves: the land dies, then its controller is asked the
    // question the card prints. The may-clause arrives first if it arrives
    // at all, so both are answered on the way to the search.
    let mut search = None;
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::ChooseCards {
                player,
                options,
                prompt,
                ..
            } => {
                search = Some((player, options, prompt));
                break;
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Quarter's ability resolved: {other:?}"),
        }
    }
    let (asked, options, prompt) =
        search.expect("the destroyed land's controller is offered the search");
    assert_eq!(
        asked, p1,
        "the *destroyed land's* controller searches, not the seat that destroyed it"
    );
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "a library search, which is what the card says"
    );
    assert!(
        !options.is_empty(),
        "the filler deck is nothing but basic lands"
    );

    let chosen = options[0];
    let found = engine
        .state()
        .object(chosen)
        .and_then(|o| o.card)
        .map(|c| c.index)
        .expect("a card in a library");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("the search offered it");
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. })
    });

    assert!(
        on_battlefield(&engine, p0, ghost_quarter()).is_none(),
        "the Quarter gave itself up to pay"
    );
    assert!(
        in_graveyard(&engine, p0, ghost_quarter()).is_some(),
        "and a sacrificed permanent lands in its owner's graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, badlands()).is_some(),
        "the land the ability named was destroyed"
    );
    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "the Forest nobody aimed at is still standing"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_none(),
        "and nothing else died beside it"
    );
    assert!(
        on_battlefield(&engine, p1, found).is_some(),
        "the basic land the search found is on the battlefield, under the \
         seat that searched"
    );
}
