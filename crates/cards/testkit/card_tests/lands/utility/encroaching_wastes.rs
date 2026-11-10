//! `cards/lands/utility/encroaching_wastes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "43144f06-079b-4515-a03a-01ea3e90d586"

/// Encroaching Wastes prints two lines and no others: "{T}: Add {C}" and
/// "{4}, {T}, Sacrifice this land: Destroy target nonbasic land."
///
/// Both are played in one main phase, and "nonbasic" is read against three
/// bystanders: the opponent's basic Forest and this seat's own basic Forests
/// must stay off the menu while the opponent's nonbasic dual — a `Land —
/// Plains Island` and therefore no basic land — is on it, so neither a bare
/// `Filter::LAND` nor a "you control" reading would pass. The {4} is claimed
/// only with the six tapped Forests' mana already floating, because
/// `can_afford` reads the *pool* and not the untapped lands, and the copy that
/// pays with its own {T} and sacrifice is named in the activation rather than
/// left to whichever the engine finds first. Two Wastes are seated rather than
/// played: the card prints no enter modifier, so a placement leaves the same
/// untapped permanent a land drop would have.
#[allow(clippy::too_many_lines)] // one card, both printed lines, each read where its price lands
#[test]
fn encroaching_wastes_taps_for_colorless_and_sells_itself_to_kill_a_nonbasic_land() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
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
                encroaching_wastes(),
                encroaching_wastes(),
            ],
        )
        // A nonbasic land and a basic one across the table: the first is what
        // "target nonbasic land" reaches, the second is what tells that word
        // from "target land".
        .battlefield(1, &[irrigated_farmland(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wastes = all_on_battlefield(&engine, p0, encroaching_wastes());
    assert_eq!(wastes.len(), 2, "two copies, one per printed line");
    let (keep, spent) = (wastes[0], wastes[1]);
    let my_forest = on_battlefield(&engine, p0, forest()).expect("a Forest of mine is out");
    let their_nonbasic =
        on_battlefield(&engine, p1, irrigated_farmland()).expect("their nonbasic dual is out");
    let their_basic = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats, so a {{4}} is unpayable from the pool"
    );

    // `LegalActions::abilities` is filtered through `can_afford`, and that
    // reads the pool: with nothing floating the destroy line is absent from
    // the offer, while the mana line — whose whole price is its own tap — is
    // there on both copies.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(keep, 0)) && legal.abilities.contains(&(spent, 0)),
        "{{T}}: Add {{C}} is offered on either copy: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(keep, 1)) && !legal.abilities.contains(&(spent, 1)),
        "{{4}} is not four, so the destroy line is not offered yet: {:?}",
        legal.abilities
    );

    // Six Forests, and both Wastes named as the printing kept back: one of
    // them is the permanent about to pay its own {{T}}, and `tap_all_mana`
    // presses a printed `{{T}}: Add …` like any other (#159).
    tap_all_mana_but(&mut engine, p0, Some(encroaching_wastes()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Forests tapped, and nothing else on this board makes mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(spent, 1)) && legal.abilities.contains(&(keep, 1)),
        "with {{4}} floating the whole price is payable on either copy: {:?}",
        legal.abilities
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: spent,
                ability_index: 1,
            },
        )
        .expect("the copy named here is the one that pays");

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target nonbasic land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the land");
    assert_eq!((min, max), (1, 1), "one land, exactly one");
    assert!(
        options.contains(&their_nonbasic),
        "\"target nonbasic land\" reaches across the table: {options:?}"
    );
    assert!(
        !options.contains(&their_basic),
        "their Forest is a land and not a nonbasic one: {options:?}"
    );
    assert!(
        !options.contains(&my_forest),
        "and a basic land of mine is no more a legal target than theirs: {options:?}"
    );
    assert_eq!(
        engine.state().object(spent).map(|o| o.zone),
        Some(Zone::Battlefield),
        "CR 601.2c before CR 601.2h: the land that will be sacrificed is still \
         on the battlefield while the question stands"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "and the {{4}} is still in the pool for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_nonbasic],
            },
        )
        .expect("the dual land was one of the options it enumerated");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{4}} came out of the six the Forests made"
    );
    assert!(
        in_graveyard(&engine, p0, encroaching_wastes()).is_some(),
        "\"Sacrifice this land\" is paid on announcement, so the copy that paid \
         is already in its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying a land is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, irrigated_farmland()).is_none(),
        "the land the ability named left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, irrigated_farmland()).is_some(),
        "and it is in its owner's graveyard — the seat that owned it, not the \
         seat that aimed the ability"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "the basic land the filter declined never moved"
    );
    assert_eq!(
        on_battlefield(&engine, p0, encroaching_wastes()),
        Some(keep),
        "the copy that paid with itself is the one that left, and the other is \
         still standing"
    );

    // The other printed line, on the copy that survived. Its whole price is
    // its own tap, so it is offered over an empty manabase and the colourless
    // lands in the pool at once (CR 605.3b: no stack).
    assert!(
        !is_tapped(&engine, keep),
        "the surviving copy is untapped, so its {{T}} is still there to pay"
    );
    let floating = engine.state().players[0].mana_pool.total();
    activate(&mut engine, p0, encroaching_wastes(), 0);
    assert!(is_tapped(&engine, keep), "{{T}} was the whole price");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.total(),
        floating + 1,
        "\"{{T}}: Add {{C}}\" — one mana, off one tap"
    );
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "and it is colourless, which the green the Forests left behind is not"
    );
    assert_eq!(
        u64::from(pool.available(ManaColor::Green)),
        floating,
        "the green the {{4}} did not eat is untouched"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
