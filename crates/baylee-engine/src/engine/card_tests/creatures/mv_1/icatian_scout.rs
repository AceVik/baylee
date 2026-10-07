//! `cards/creatures/mv_1/icatian_scout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Icatian Scout — {W}, a 1/1 Human Soldier Scout whose whole text is one
/// activated ability: "{1}, {T}: Target creature gains first strike until
/// end of turn."
///
/// Both halves of the price are asserted, and asserted *after* the target is
/// answered: CR 601.2c picks the target before CR 601.2h pays, so while the
/// target question stands the Scout is still untapped and the pool is still
/// full. The keyword is read off the layer projection, which is the only
/// reading that can see a granted one — and the two bystanders are what make
/// the grant a *choice*: the Elf beside it on the same battlefield and the
/// Elf across the table keep nothing, and the pump is 0/+0, so a first strike
/// that stayed green while eating a +1/+1 would still fail here.
#[test]
fn icatian_scout_taps_itself_and_a_mana_to_grant_first_strike_to_one_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), icatian_scout(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let scout = on_battlefield(&engine, p0, icatian_scout()).expect("the Scout is out");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "nothing has first strike before anything is activated"
    );

    // The Elves are the creature about to be aimed at, so they are the one
    // source kept untapped; the two Plains are the {1}. Reading the offer
    // only says something once the mana is already in the pool, because
    // `can_afford` reads the pool and not the untapped lands.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    let pool_before = engine.state().players[0].mana_pool.total();
    assert_eq!(pool_before, 2, "two Plains tapped, and the Elves kept back");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        2,
        "the two Plains, and nothing else made mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(scout, 0)),
        "two floating mana pay the {{1}}, so the one line the card prints is \
         offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, icatian_scout(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: \
         {options:?}"
    );
    assert!(
        options.contains(&scout),
        "and the card prints no \"another\", so the Scout is on its own menu: \
         {options:?}"
    );

    // CR 601.2c first, CR 601.2h last: neither half of the price is paid
    // while the target question is still standing.
    assert!(!is_tapped(&engine, scout), "{{T}} is not paid yet");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        pool_before,
        "and no mana has left the pool yet"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    assert!(
        is_tapped(&engine, scout),
        "{{T}} on the Scout is the half of the price a creature pays"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        pool_before - 1,
        "and the {{1}} came out of the pool"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, mine).contains(KeywordSet::FIRST_STRIKE),
        "\"target creature gains first strike until end of turn\""
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "the grant reaches the creature it was aimed at and no other"
    );
    assert!(
        !keywords(&engine, scout).contains(KeywordSet::FIRST_STRIKE),
        "and the Scout gave it away rather than keeping it"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "zero power and zero toughness: the clause is a keyword and not a pump"
    );
}
