//! `cards/creatures/mv_2/wind_dancer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Wind Dancer — {1}{U} 1/1 Faerie, printed with flying — says "{T}: Target
/// creature gains flying until end of turn." The grant is a keyword and no
/// body, so the only reading worth playing is the one that tells the creature
/// the ability *named* from every other creature on the table: a second Elf
/// under the same seat and an Elf across it must both stay grounded, and the
/// target's `0/0` has to leave its printed 1/1 exactly as it was. The whole
/// price is the Dancer's own `{T}` — the pool is empty before the activation
/// and still empty after it — which is what says the flying cost no mana.
#[test]
fn wind_dancer_grants_flying_to_the_creature_it_names_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[wind_dancer(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two Elves, one of which stays on the ground"
    );
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    let dancer = on_battlefield(&engine, p0, wind_dancer()).expect("the Dancer is out");
    assert!(
        keywords(&engine, dancer).contains(KeywordSet::FLYING),
        "the Dancer carries the flying its own card prints"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "nothing has been granted yet"
    );
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the grant");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(dancer, 0)),
        "the one line the Dancer prints is paid by its own {{T}} and no mana, \
         so it is offered out of a bare board: {:?}",
        legal.abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing had to be floated for it — the pool is empty"
    );

    activate(&mut engine, p0, wind_dancer(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures under this seat are on the menu: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        options.contains(&dancer),
        "and the Dancer is a creature too, so it may name itself: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf the question offered was chosen");
    assert!(
        is_tapped(&engine, dancer),
        "CR 601.2h: the {{T}} is the last step of the activation, paid after \
         the target was named"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, host).contains(KeywordSet::FLYING),
        "the named creature gained flying until end of turn"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FLYING),
        "the Elf nobody named is untouched"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "and nothing crossed the table"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "the grant is the keyword and no body: +0/+0 leaves the printed 1/1"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the whole price was the tap symbol and nothing else was spent"
    );
}
