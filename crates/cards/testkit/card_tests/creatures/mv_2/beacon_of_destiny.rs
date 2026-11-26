//! `cards/creatures/mv_2/beacon_of_destiny.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_core::ids::DamageSourceRef;

fn beacon_of_destiny() -> CardIndex {
    card_index("a1ed2274-7774-4c65-a95b-28ae5e225994")
}

/// Beacon of Destiny redirects the next damage that a chosen source would deal
/// to you this turn to Beacon of Destiny instead.
#[test]
fn beacon_of_destiny_redirects_next_damage_from_chosen_source_to_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[beacon_of_destiny()])
        .battlefield(1, &[mountain()])
        .hand(1, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);

    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, index::LIGHTNING_BOLT);
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    let bolt_id = engine.state().zones.list(ZoneLocation::Stack)[0];
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    activate(&mut engine, p0, beacon_of_destiny(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    let chosen = options
        .iter()
        .copied()
        .find(|s| s.object == bolt_id)
        .unwrap_or_else(|| DamageSourceRef {
            object: bolt_id,
            version: engine.state().object(bolt_id).unwrap().version,
        });
    assert!(options.contains(&chosen));
    engine
        .apply(
            p0,
            PlayerAction::ChooseDamageSource {
                choice,
                source: chosen,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(lk::life(&engine, p0), 20);
    assert!(on_battlefield(&engine, p0, beacon_of_destiny()).is_none());
}

/// Damage from unchosen sources is NOT redirected and hits you normally.
#[test]
fn beacon_of_destiny_leaves_unchosen_source_damage_unredirected() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[beacon_of_destiny()])
        .battlefield(1, &[grizzly_bears(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let beacon = on_battlefield(&engine, p0, beacon_of_destiny()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p0)), (elves, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    activate(&mut engine, p0, beacon_of_destiny(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    let bears_ref = options.iter().copied().find(|s| s.object == bears).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseDamageSource {
                choice,
                source: bears_ref,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(lk::life(&engine, p0), 19);
    assert_eq!(engine.state().object(beacon).unwrap().damage, 2);
}
