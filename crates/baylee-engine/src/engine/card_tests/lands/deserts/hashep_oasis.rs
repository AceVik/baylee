//! `cards/lands/deserts/hashep_oasis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hashep Oasis prints two mana abilities and a sorcery-speed pump:
/// "{1}{G}{G}, {T}, Sacrifice a Desert: Target creature gets +3/+3 until end
/// of turn." The pump is the clause worth playing, because every noun in it is
/// a board reading: the sacrifice menu must hold the Deserts this seat controls
/// — the source is itself one — and neither the Forests beside them nor the
/// Desert across the table, the target must be any creature rather than only
/// its controller's, and the +3/+3 must be a real three mana out of the pool
/// rather than a label. The Elves it is aimed at is the control for the last
/// of those: it ends the phase a 4/4 while the Elves across the table is still
/// the 1/1 it was printed as.
#[test]
#[allow(clippy::too_many_lines)] // one land, played through every clause it prints
fn hashep_oasis_sacrifices_a_desert_for_three_three_on_the_creature_it_targets() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                hashep_oasis(),
                desert(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[desert(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let oasis = on_battlefield(&engine, p0, hashep_oasis()).expect("the Oasis is out");
    let fodder = on_battlefield(&engine, p0, desert()).expect("a Desert of mine is out");
    let theirs_land = on_battlefield(&engine, p1, desert()).expect("a Desert of theirs is out");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");

    // The pump costs {1}{G}{G} besides its tap, so the mana goes into the pool
    // before anything is claimed: the offer is read off the pool, not off what
    // could still be tapped. The Oasis is the one source kept standing — its
    // {T} is half of the cost about to be paid, and `tap_all_mana` would have
    // spent it.
    tap_all_mana_but(&mut engine, p0, Some(hashep_oasis()));
    let before = engine.state().players[0].mana_pool.total();
    assert!(before >= 3, "{{1}}{{G}}{{G}} is payable: {before} floating");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds its own main phase: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(oasis, 2)),
        "a Desert to eat, a creature to aim at and three mana in the pool: the \
         sorcery-speed pump is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, hashep_oasis(), 2);

    // Targets before costs (CR 601.2c, then 601.2h), answered in the order
    // they arrive rather than in the order they are expected.
    let mut aim: Option<Vec<ObjectId>> = None;
    let mut menu: Vec<ObjectId> = Vec::new();
    for _ in 0..8 {
        if aim.is_some() && !menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert_eq!(player, p0, "the activating seat aims its own pump");
                assert!(
                    options.contains(&mine) && options.contains(&theirs),
                    "\"target creature\" is any creature on the table, on either \
                     side of it: {options:?}"
                );
                assert!(
                    !options.contains(&oasis),
                    "a land is no creature: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![mine],
                        },
                    )
                    .expect("my Elves are a legal target");
                aim = Some(options);
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostSacrifice,
                    "the Desert is a cost being paid and not a search"
                );
                assert_eq!((min, max), (1, 1), "one Desert, no more and no fewer");
                menu = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .expect("my Desert is one of the options and pays the cost");
            }
            other => panic!("unexpected while the pump is announced: {other:?}"),
        }
    }
    assert!(aim.is_some(), "the pump asks what it is aimed at");
    assert_eq!(
        menu.len(),
        2,
        "the two Deserts this seat controls, the Oasis among them: {menu:?}"
    );
    assert!(
        menu.contains(&fodder) && menu.contains(&oasis),
        "the source is a Desert and sits on its own menu: {menu:?}"
    );
    assert!(
        !menu.contains(&theirs_land),
        "CR 701.21a: a seat sacrifices only what it controls: {menu:?}"
    );
    assert!(
        !menu.contains(&mine),
        "the Elves are a creature and no Desert: {menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (4, 4),
        "\"target creature gets +3/+3 until end of turn\""
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the pump reaches the creature it targeted and no other"
    );
    assert!(
        on_battlefield(&engine, p0, hashep_oasis()).is_some(),
        "the Oasis ate the other Desert and stayed on the battlefield"
    );
    assert!(is_tapped(&engine, oasis), "its {{T}} was half the cost");
    assert!(
        in_graveyard(&engine, p0, desert()).is_some(),
        "the sacrificed Desert is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, desert()).is_some(),
        "the Desert across the table never moved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        before - 3,
        "{{1}}{{G}}{{G}} came out of the pool, not out of a label"
    );
}
