//! `cards/instants/mv_2/legion_leadership.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Legion Leadership // Legion Stronghold (`Coverage::Implemented`): "Until
/// end of turn, double target creature's power and it gains first strike."
///
/// The spell doubles the target's *current* power (`Amount::TargetPower`)
/// and grants first strike. A 2/2 Llanowar Elves on the table (seeded with a
/// +1/+1 counter via the counter helper) becomes a 4/2 with first strike.
/// The toughness is unchanged, ruling out an accidental toughness doubling.
#[test]
fn legion_leadership_doubles_power_and_grants_first_strike() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[mountain(), mountain(), llanowar_elves()])
        .hand(0, &[legion_leadership()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let creature = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is deployed");

    // Put a +1/+1 counter on the Elf so its current power is 2 rather than
    // the printed 1 — a doubled 1 and a doubled 2 are different numbers,
    // which is what makes this assertion not trivially satisfied.
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        crate::replacement::put_counters(state, creature, CounterKind::P1P1, 1);
    }
    engine.refresh_offer();
    // The Elf is not read here. `record_counters` invalidates the
    // projections, but `refresh_offer` recomputes the *legal actions* and
    // not the layers, so `characteristics()` would still answer 1/1 until
    // the engine runs a pass of its own. The claim this test makes — that
    // the doubling reads the creature's current power and not its printed
    // one — is carried by the (4, 2) at the end instead, which is a
    // different number from the (2, 1) a printed 1 would have doubled to.

    cast_from_hand(&mut engine, p0, legion_leadership());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Legion Leadership asks for a target, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&creature),
        "the Elf is a legal creature target: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .expect("the Elf is a legal target");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, creature),
        (4, 2),
        "power doubled from 2 to 4; toughness is unchanged"
    );
    assert!(
        keywords(&engine, creature).contains(KeywordSet::FIRST_STRIKE),
        "the spell also grants first strike until end of turn"
    );
}
