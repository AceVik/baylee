//! `cards/artifacts/mv_5/myr_matrix.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Myr Matrix prints three lines — indestructible, "Myr creatures get +1/+1",
/// and "{5}: Create a 1/1 colorless Myr artifact creature token" — and one game
/// reads all three. Ten Forests are exactly the {5} the artifact costs plus the
/// {5} the ability charges, so the pool reads ten, then five, then nothing. The
/// static is read on two creatures at once, a Myr Retriever that gains a point
/// and a Llanowar Elves that does not, which is what tells "Myr creatures" from
/// "creatures you control"; the token that arrives proves it is a Myr the same
/// way, printed 1/1 and projected 2/2. The keyword is played rather than read:
/// an opponent's Vindicate names the Matrix and the Matrix is still where it was.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end
fn myr_matrix_builds_a_myr_and_pumps_only_the_myr() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board: Vec<CardIndex> = vec![forest(); 10];
    board.extend([llanowar_elves(), myr_retriever()]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        // A destroy effect across the table, so the printed indestructible is
        // paid for rather than read off the card file.
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(0, &[myr_matrix()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let retriever = on_battlefield(&engine, p0, myr_retriever()).expect("the Myr Retriever is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert_eq!(
        pt(&engine, retriever),
        (1, 1),
        "a printed 1/1 while the Matrix is still in hand"
    );
    assert_eq!(pt(&engine, elves), (1, 1), "and one that is no Myr at all");

    // Ten Forests and nothing else: the Elf is named as the printing kept
    // back, because it is a creature this test reads afterwards and its own
    // `{T}: Add {G}` would otherwise be in the pool the cast is measured
    // against.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        10,
        "ten tapped Forests and ten green, and neither creature contributed"
    );
    cast_with_floating(&mut engine, p0, myr_matrix());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let matrix = on_battlefield(&engine, p0, myr_matrix()).expect("the Matrix resolved");
    assert!(
        keywords(&engine, matrix).contains(KeywordSet::INDESTRUCTIBLE),
        "the printed indestructible reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "the {{5}} is spent and exactly the {{5}} the ability charges is left"
    );
    assert_eq!(
        pt(&engine, retriever),
        (2, 2),
        "\"Myr creatures get +1/+1\": the Myr on this board is one point bigger"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "and the creature that is no Myr is left exactly as it was printed"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing has been activated yet, so nothing has been made"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, and that
    // reads the pool rather than the untapped lands — so the claim about the
    // offer is made with the mana already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(matrix, 1)),
        "ability 0 is the static that pumps and is never offered; ability 1 is \
         the {{5}} that makes a Myr, and its price is in the pool: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, myr_matrix(), 1);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{5}} it charges came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "making a token is no mana ability, so the ability is on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Myr arrives on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Myr");
    let myr = tokens[0];
    let kinds = types(&engine, myr);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::CREATURE),
        "\"artifact creature token\": {kinds:?}"
    );
    let printed = engine
        .state()
        .object(myr)
        .expect("the Myr is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Myr", "the name the card gives it");
    assert_eq!(
        (printed.power, printed.toughness),
        (Some(1), Some(1)),
        "the body the card prints"
    );
    for color in [
        baylee_core::color::Color::White,
        baylee_core::color::Color::Blue,
        baylee_core::color::Color::Black,
        baylee_core::color::Color::Red,
        baylee_core::color::Color::Green,
    ] {
        assert!(
            !printed.colors.contains(color),
            "\"colorless\" is the first word of the token's type line"
        );
    }
    assert_eq!(
        pt(&engine, myr),
        (2, 2),
        "and the Matrix's own static pumps the Myr it just made: a point of \
         power and toughness that only the subtype can account for"
    );

    // The other printed line, played rather than read: a destroy effect names
    // the Matrix and the Matrix stays where it is (CR 702.12b).
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p1, "the seat that cast it names the target");
    assert!(
        options.contains(&matrix),
        "\"target permanent\" names any permanent, the indestructible one \
         included: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![matrix],
            },
        )
        .expect("the Matrix was one of the options it enumerated");
    pass_until(&mut engine, |e| in_graveyard(e, p1, vindicate()).is_some());

    assert!(
        on_battlefield(&engine, p0, myr_matrix()).is_some(),
        "\"effects that say destroy don't destroy this artifact\" — the \
         Vindicate resolved against it and it is still on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, myr_matrix()).is_none(),
        "and it was not moved anywhere else either"
    );
    assert!(
        in_graveyard(&engine, p1, vindicate()).is_some(),
        "the Vindicate itself resolved rather than being countered or fizzling"
    );
    assert_eq!(
        pt(&engine, retriever),
        (2, 2),
        "the Matrix survived, so its static is still running on the Myr beside it"
    );
}
