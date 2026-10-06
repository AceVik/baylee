//! `cards/lands/utility/pendelhaven.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pendelhaven prints `{T}: Add {G}` and `{T}: Target 1/1 creature gets
/// +1/+2 until end of turn.` With a 1/1 on the board both lines are offered,
/// and this is the half that reads the **offer**: two abilities on one
/// permanent, each costing only `{T}`, so the first one paid for takes the
/// other away. The pump's own effect is played out one test over; what is
/// asserted here is that taking the mana leaves the creature alone.
#[test]
fn pendelhaven_offers_both_its_lines_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[llanowar_elves()])
        .hand(0, &[pendelhaven()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, pendelhaven());
    assert!(on_battlefield(&engine, p0, pendelhaven()).is_some());
    assert!(!is_tapped(&engine, land));

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf on battlefield");
    assert_eq!(pt(&engine, elf), (1, 1));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    let offered: Vec<(ObjectId, u32)> = legal
        .abilities
        .iter()
        .copied()
        .filter(|(source, _)| *source == land)
        .collect();
    assert_eq!(
        offered,
        vec![(land, 0), (land, 1)],
        "the mana ability and the pump, with a 1/1 standing for the second one to name"
    );

    activate(&mut engine, p0, pendelhaven(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
    assert_eq!(pt(&engine, elf), (1, 1), "the mana half pumps nothing");
}

/// Pendelhaven's second line is a size test in two directions at once:
/// "{T}: Target 1/1 creature gets +1/+2 until end of turn." A 1/1 Llanowar
/// Elves stands beside a 0/3 Sylvan Caryatid, which fails the power bound
/// low and the toughness bound high — a filter reading only
/// `ToughnessAtMost(1)` would have offered it, and one reading only a power
/// bound would have offered it too. The restriction is on the *target*
/// (CR 115.3), so the wrong creature is never in the menu rather than being
/// refused afterwards.
#[test]
fn pendelhaven_pumps_a_one_one_and_is_offered_nothing_else() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9101, forest())
        .battlefield(0, &[pendelhaven(), llanowar_elves(), sylvan_caryatid()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are seated");
    let caryatid = on_battlefield(&engine, p0, sylvan_caryatid()).expect("the Caryatid is seated");
    assert_eq!(pt(&engine, elves), (1, 1), "the printed 1/1");
    assert_eq!(pt(&engine, caryatid), (0, 3), "and a body that is neither");

    activate(&mut engine, p0, pendelhaven(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![elves],
        "only the creature that is 1/1 in both directions is a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the target came out of the menu");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elves), (2, 3), "+1/+2 until end of turn");
    assert_eq!(
        pt(&engine, caryatid),
        (0, 3),
        "and the creature the ability could not name is untouched"
    );
}
