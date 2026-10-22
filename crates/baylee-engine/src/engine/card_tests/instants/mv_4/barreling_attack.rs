//! `cards/instants/mv_4/barreling_attack.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BARRELING_ATTACK, GRIZZLY_BEARS, HILL_GIANT};

#[test]
fn barreling_attack_grants_trample_and_pumps_per_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[BARRELING_ATTACK])
        .battlefield(
            0,
            &[HILL_GIANT, mountain(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[GRIZZLY_BEARS, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let attacker = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let blockers: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let obj = engine.state().object(id).unwrap();
            obj.controller == p1 && obj.card.is_some_and(|c| c.index == GRIZZLY_BEARS)
        })
        .collect();
    assert_eq!(blockers.len(), 2);

    cast_from_hand(&mut engine, p0, BARRELING_ATTACK);
    aim_at(&mut engine, p0, attacker);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(attacker)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::TRAMPLE)
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blockers[0], attacker), (blockers[1], attacker)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, attacker), (5, 5));
}
