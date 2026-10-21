//! Giant Albatross — `{1}{U}` 1/1 Creature — Bird (Homelands)
//!
//! Flying
//! When this creature dies, you may pay `{1}{U}`. If you do, for each creature
//! that dealt damage to this creature this turn, destroy that creature unless
//! its controller pays 2 life. A creature destroyed this way can't be
//! regenerated.
//!
//! (`Coverage::Partial`: per-creature controller 2-life payment to prevent
//! destruction is not in `Effect`; when Giant Albatross dies, you may pay
//! `{1}{U}` to destroy without regeneration each creature that dealt damage to
//! it this turn.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn giant_albatross() -> CardIndex {
    card_index("8b59cacb-46ff-4400-a7f6-b258c81a9b14")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn giant_albatross_pays_one_blue_when_it_dies_to_destroy_only_creatures_that_damaged_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[giant_albatross(), island(), island()])
        .battlefield(1, &[bears(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let albatross = on_battlefield(&engine, p0, giant_albatross()).unwrap();
    let attacker = on_battlefield(&engine, p1, bears()).unwrap();
    let bystander = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    assert!(keywords(&engine, albatross).contains(KeywordSet::FLYING));

    // Pass to p1's combat: p1 attacks with Grizzly Bears, p0 blocks with Giant Albatross.
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

    // Giant Albatross dies to Grizzly Bears' 2 combat damage -> asks p0 to pay {1}{U}.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(attacker).unwrap().zone,
        Zone::Graveyard,
        "creature that dealt damage to Giant Albatross this turn was destroyed"
    );
    assert_eq!(
        engine.state().object(bystander).unwrap().zone,
        Zone::Battlefield,
        "creature that did not deal damage to Giant Albatross this turn was spared"
    );
}
