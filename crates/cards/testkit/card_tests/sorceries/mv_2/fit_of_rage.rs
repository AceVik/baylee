//! `cards/sorceries/mv_2/fit_of_rage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fit of Rage is one line — a `{1}{R}` sorcery: "target creature gets +3/+3
/// and gains first strike until end of turn" — and each of its three parts is
/// readable only off a board that carries a control for it. Two Elves stand
/// under the caster and a third across the table, so the target question has
/// to name all three (the card says "target creature", not "you control")
/// while the pump and the keyword may land on exactly the one that was
/// answered. Walking on into the following turn's main phase then reads the
/// "until end of turn" off that same creature, which is what separates a pump
/// from a permanent.
#[test]
fn fit_of_rage_pumps_and_arms_the_one_creature_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), llanowar_elves(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[fit_of_rage()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves of mine, one of which stays bare");
    let (target, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        pt(&engine, target),
        (1, 1),
        "a printed 1/1 before the spell"
    );
    assert!(
        !keywords(&engine, target).contains(KeywordSet::FIRST_STRIKE),
        "and no keyword at all yet"
    );

    // The two Mountains go into the pool first: castability is read off the
    // pool and not off the untapped lands.
    cast_from_hand(&mut engine, p0, fit_of_rage());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast the spell aims it");
    assert!(
        options.contains(&target) && options.contains(&bystander) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the creature the question offered is the one it was aimed at");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, target),
        (4, 4),
        "+3/+3 on the creature Fit of Rage named"
    );
    assert!(
        keywords(&engine, target).contains(KeywordSet::FIRST_STRIKE),
        "and the printed keyword arrives with it"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody named is still the 1/1 it was printed as, so the pump \
         is a target and not \"creatures you control\""
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FIRST_STRIKE),
        "and the keyword reaches no other body either"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the effect reaches the target and never across the table"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "nor does the keyword"
    );

    // "until end of turn" is a clause, not decoration: the same object read
    // after the turn has rolled over is the printed 1/1 again.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, target),
        (1, 1),
        "the pump expires with the turn it was cast in"
    );
    assert!(
        !keywords(&engine, target).contains(KeywordSet::FIRST_STRIKE),
        "and the granted keyword goes with it"
    );
}
