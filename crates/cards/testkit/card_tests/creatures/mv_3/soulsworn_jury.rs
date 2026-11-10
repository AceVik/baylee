//! `cards/creatures/mv_3/soulsworn_jury.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Soulsworn Jury — {2}{W} — a 1/4 Spirit with defender whose second line is
/// "{1}{U}, Sacrifice this creature: Counter target creature spell."
///
/// A creature spell exists only between the announcement and its resolution,
/// so the board has to hold one mid-flight: p1 casts a Llanowar Elves and
/// passes, which is the only window in which the Jury's ability has a legal
/// target at all. Two Islands pay the {1}{U} and the printed sacrifice is read
/// where it lands — the Jury is still standing while the target question is
/// open (CR 601.2c before 601.2h) and in its owner's graveyard once the answer
/// is given — while the Elves go from the stack to a graveyard without ever
/// becoming a creature.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn soulsworn_jury_sacrifices_itself_to_counter_a_creature_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[soulsworn_jury(), island(), island()])
        .battlefield(1, &[forest()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    let jury = on_battlefield(&engine, p0, soulsworn_jury()).expect("the Jury is deployed");
    assert_eq!(pt(&engine, jury), (1, 4), "the body the card prints");
    assert!(
        keywords(&engine, jury).contains(KeywordSet::DEFENDER),
        "and the defender that keeps it out of combat"
    );

    // p1's own main phase: a creature spell needs a sorcery-speed window, and
    // the seat that is not casting it is the one that has to answer.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, llanowar_elves());
    assert!(
        on_stack(&engine, llanowar_elves()).is_some(),
        "the Elves are a spell on the stack, not yet a creature: {:?}",
        engine.pending()
    );
    // The caster holds priority after casting, so the walk passes until the
    // response window belongs to p0 and the spell is still in flight.
    pass_until(&mut engine, |e| {
        on_stack(e, llanowar_elves()).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    // Mana before the claim: `can_afford` reads the pool and not the untapped
    // Islands.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        2,
        "two Islands, two blue"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(jury, 0)),
        "{{1}}{{U}} and the sacrifice are payable, so the one line the card \
         prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, soulsworn_jury(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"counter target creature spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    let elves = on_stack(&engine, llanowar_elves()).expect("the spell is still on the stack");
    assert!(
        options.contains(&elves),
        "the creature spell mid-flight is the whole of the offer: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, soulsworn_jury()).is_some(),
        "targets are chosen before costs are paid (CR 601.2c, then CR 601.2h)"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the {{1}}{{U}} is still in the pool for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the spell the question named was one of the options");

    assert!(
        on_battlefield(&engine, p0, soulsworn_jury()).is_none(),
        "\"Sacrifice this creature\" is paid as the ability is announced"
    );
    assert!(
        in_graveyard(&engine, p0, soulsworn_jury()).is_some(),
        "and the Jury is in its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{U}} came out of the pool"
    );
    assert!(
        on_stack(&engine, llanowar_elves()).is_some(),
        "the counter has not resolved yet: the spell is still on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "a countered creature spell never becomes a creature"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "it goes to the graveyard of the seat that cast it"
    );
}
