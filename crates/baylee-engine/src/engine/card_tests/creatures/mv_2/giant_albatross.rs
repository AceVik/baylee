//! Giant Albatross — `{1}{U}` 1/1 Creature — Bird (Homelands)
//!
//! Flying
//! When this creature dies, you may pay `{1}{U}`. If you do, for each creature
//! that dealt damage to this creature this turn, destroy that creature unless
//! its controller pays 2 life. A creature destroyed this way can't be
//! regenerated.
//!
//! (`Coverage::Partial`: the death trigger is left off, because no piece asks
//! each damaging creature's controller for 2 life after the mana payment.
//! Destroying without that question would do more than the card prints, so
//! this test pins that nothing is asked and nothing is destroyed. It moves
//! when the trigger can be said.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn giant_albatross() -> CardIndex {
    card_index("8b59cacb-46ff-4400-a7f6-b258c81a9b14")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn giant_albatross_flies_and_its_death_trigger_is_not_played() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[giant_albatross(), island(), island()])
        .battlefield(1, &[bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let albatross = on_battlefield(&engine, p0, giant_albatross()).unwrap();
    let attacker = on_battlefield(&engine, p1, bears()).unwrap();
    assert!(keywords(&engine, albatross).contains(KeywordSet::FLYING));

    // P1 attacks with Grizzly Bears and P0 blocks with the Albatross, which
    // dies to the Bears' 2 combat damage.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(albatross, attacker)],
            },
        )
        .unwrap();

    // No pay-{1}{U} question comes (a `YesNo` would stop this wait), and the
    // creature that killed the Albatross is not destroyed.
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(
        in_graveyard(&engine, p0, giant_albatross()).is_some(),
        "the Albatross died to the Bears"
    );
    assert_eq!(
        engine.state().object(attacker).unwrap().zone,
        Zone::Battlefield,
        "the death trigger is not played, so the Bears are not destroyed"
    );
}
