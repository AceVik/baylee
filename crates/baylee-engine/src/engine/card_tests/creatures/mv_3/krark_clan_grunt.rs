//! `cards/creatures/mv_3/krark_clan_grunt.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Krark-Clan Grunt is a `{2}{R}` 2/2 Goblin Warrior whose whole text is one
/// line: "Sacrifice an artifact: This creature gets +1/+0 and gains first
/// strike until end of turn."
///
/// The price names no artifact in particular, so the engine has to ask which
/// one — and that menu is half the card: both Sol Rings under this seat are on
/// it, while the Sol Ring across the table is not (CR 701.21a, a seat
/// sacrifices only what it controls) and the Grunt itself is a creature, not
/// an artifact. Two Rings stand on the board so the second activation's menu
/// can show that a cost letting one artifact answer twice would have eaten both
/// from a single press, and the pump is read off the layer projection — 2/2 to
/// 3/2 to 4/2, first strike with the first one — which no count of activations
/// could tell from a body that never changed.
#[allow(clippy::too_many_lines)] // two activations, and every clause of the card read off them
#[test]
fn krark_clan_grunt_eats_its_own_artifacts_to_pump_itself_one_at_a_time() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                quiet_artifact(),
                quiet_artifact(),
                krark_clan_grunt(),
            ],
        )
        // An artifact on the other side of the table: "sacrifice an artifact"
        // is not an invitation to eat somebody else's.
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let grunt = on_battlefield(&engine, p0, krark_clan_grunt()).expect("the Grunt is out");
    let rings = all_on_battlefield(&engine, p0, quiet_artifact());
    assert_eq!(rings.len(), 2, "two artifacts, one per activation");
    let (first, second) = (rings[0], rings[1]);
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    assert_eq!(pt(&engine, grunt), (2, 2), "a printed 2/2");
    assert!(
        !keywords(&engine, grunt).contains(KeywordSet::FIRST_STRIKE),
        "and nothing has pumped it yet"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the price is an artifact and no mana, so nothing is floating"
    );

    // The cost is a sacrifice, so `can_afford` turns on the board and not on
    // the pool: the line is offered with an empty pool and an untapped Grunt.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(grunt, 0)),
        "an artifact to eat is the whole price, so the line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, krark_clan_grunt(), 0);
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which artifact, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
    assert_eq!(
        options.len(),
        2,
        "the two artifacts this seat controls: {options:?}"
    );
    assert!(
        options.contains(&first) && options.contains(&second),
        "both Sol Rings are on the menu: {options:?}"
    );
    assert!(
        !options.contains(&grunt),
        "the Grunt is a Goblin Warrior and no artifact: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "CR 701.21a: an opponent's artifact is not yours to sacrifice: {options:?}"
    );

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![theirs],
                },
            )
            .is_err(),
        "an answer the question did not enumerate is refused"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and the refusal costs the other seat nothing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![first],
            },
        )
        .expect("the artifact the question offered pays the cost");
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "CR 601.2h: the sacrifice is the last step and is already paid"
    );
    assert!(
        !stack_is_empty(&engine),
        "pumping a creature is no mana ability, so the ability is on the stack"
    );
    assert_eq!(
        pt(&engine, grunt),
        (2, 2),
        "and nothing has resolved yet, so the pump is still on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, grunt), (3, 2), "+1/+0 from the first Ring");
    assert!(
        keywords(&engine, grunt).contains(KeywordSet::FIRST_STRIKE),
        "and the printed first strike arrives with it"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the second Ring is still standing to be eaten"
    );

    // The second activation, and the same menu one shorter. A cost that let
    // one artifact answer twice would have emptied the board from the first
    // press and offered nothing here.
    activate(&mut engine, p0, krark_clan_grunt(), 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "the second activation asks again, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![second],
        "one Ring left, and the one already eaten is not on the menu"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![second],
            },
        )
        .expect("the artifact the question offered pays the cost");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, grunt),
        (4, 2),
        "two activations stack: +1/+0 twice"
    );
    assert!(
        keywords(&engine, grunt).contains(KeywordSet::FIRST_STRIKE),
        "and the keyword it already had is still there"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "both artifacts were spent rather than one answering twice"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2,
        "and both are in their owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the opponent's board never moved"
    );
}
