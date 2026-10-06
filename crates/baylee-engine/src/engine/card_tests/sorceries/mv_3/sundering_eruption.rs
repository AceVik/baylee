//! `cards/sorceries/mv_3/sundering_eruption.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Sundering Eruption` // `Volcanic Fissure`: "Destroy target land. Its
/// controller may search their library for a basic land card, put it onto
/// the battlefield tapped, then shuffle. Creatures without flying can't
/// block this turn. // As this land enters, you may pay 3 life. If you
/// don't, it enters tapped. {T}: Add {R}."
///
/// The first two sentences here: the land is destroyed, a creature is not
/// a legal target for it, and the destroyed land's controller — not the
/// caster — is the seat asked to search, who puts a basic onto the
/// battlefield tapped. The third sentence is the test below this one.
#[test]
fn sundering_eruption_destroys_land_and_gives_opponent_tapped_basic() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(476, forest())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .hand(0, &[sundering_eruption()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let target_land = on_battlefield(&engine, p1, forest()).expect("opponent land");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent elf");

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, sundering_eruption());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&target_land),
        "target land is a legal target"
    );
    assert!(
        !options.contains(&elf),
        "creature is not a legal target for land destruction"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_land],
            },
        )
        .unwrap();

    for _ in 0..8 {
        if matches!(engine.pending(), Pending::ChooseCards { .. }) {
            break;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected basic land search prompt, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p1,
        "the basic-land search belongs to the destroyed land's controller"
    );
    assert_eq!((min, max), (0, 1), "may search for up to one basic land");
    let found_land = options[0];
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![found_land],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        !engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Battlefield)
            .contains(&target_land),
        "the targeted land was destroyed — it is off the battlefield, not \
         out of the arena, because a destroyed permanent is still an object"
    );
    assert!(
        is_tapped(&engine, found_land),
        "the searched basic land enters tapped"
    );
    assert_eq!(
        engine.state().object(found_land).map(|o| o.controller),
        Some(p1),
        "the searched basic land is controlled by the opponent"
    );
    assert!(
        in_graveyard(&engine, p0, sundering_eruption()).is_some(),
        "Sundering Eruption moves to graveyard after resolving"
    );
    assert!(
        keywords(&engine, elf).contains(KeywordSet::CANT_BLOCK),
        "and the rider reached the opponent's ground creature on the way \
         past, which is the sentence the test below this one is about"
    );
}

/// `Sundering Eruption`'s third sentence: "Creatures without flying can't
/// block this turn."
///
/// Board-wide and not a target, so it is an `Effect::PumpFilter` with no
/// `controlled_by`: every creature the filter matches, at both seats. The
/// filter is what this is really about, and a flier beside a ground
/// creature is the only board that can show it — one keeps the defence and
/// one does not, and both of them belong to the same player.
///
/// CR 611.2c fixes that set as the spell resolves, which is why both
/// creatures are seated before the cast rather than arriving after it.
#[test]
fn sundering_eruption_grounds_the_defence_and_leaves_a_flier_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(477, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), rootbreaker_wurm()])
        .battlefield(1, &[forest(), llanowar_elves(), baleful_strix()])
        .hand(0, &[sundering_eruption()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let doomed = on_battlefield(&engine, p1, forest()).expect("a land to destroy");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the ground creature");
    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("the flier");
    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("the 6/6 attacks");

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, sundering_eruption());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the land came out of the menu");

    // The controller of the destroyed land declines the search: this test is
    // about the rider, and a basic arriving tapped cannot block anyway.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("\"may search\" — naming nothing is how a player declines");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, elf).contains(KeywordSet::CANT_BLOCK),
        "a creature without flying"
    );
    assert!(
        !keywords(&engine, strix).contains(KeywordSet::CANT_BLOCK),
        "and one with it is not in the set at all — which is the filter, \
         not the seat: both creatures are the opponent's"
    );

    let blocks = attack_and_collect_blocks(&mut engine, wurm, p1);
    assert_eq!(
        blocks.iter().map(|o| o.blocker).collect::<Vec<_>>(),
        vec![strix],
        "only the flier is offered a block, and it is offered one: {blocks:?}"
    );
}
