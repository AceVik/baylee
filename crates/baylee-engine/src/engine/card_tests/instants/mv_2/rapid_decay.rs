//! `cards/instants/mv_2/rapid_decay.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn rapid_decay() -> CardIndex {
    card_index("2c769dfc-79be-4fcb-840c-b72cafdd0a23")
}

/// Rapid Decay (2c769dfc-79be-4fcb-840c-b72cafdd0a23) — {1}{B} Instant (Partial):
/// "Exile up to three target cards from a single graveyard."
/// "Cycling {2}"
///
/// Under Partial coverage, exiles up to one target card from any graveyard
/// (refusing a second target) and supports Cycling {2}.
#[test]
fn rapid_decay_exiles_up_to_one_graveyard_card_and_cycles_for_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[rapid_decay(), rapid_decay()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 3);
    let mine = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .clone();
    let theirs = engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p1))
        .clone();

    // 1. Cast first Rapid Decay: offers graveyard cards with (min, max) = (0, 1),
    // refuses 2 targets, and exiles the chosen card.
    cast_from_hand(&mut engine, p0, rapid_decay());
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets for Rapid Decay");
    };
    assert_eq!((min, max), (0, 1));
    assert!(options.contains(&mine[0]) && theirs.iter().all(|c| options.contains(c)));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: theirs[..2].to_vec(),
                },
            )
            .is_err(),
        "a second target is refused"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(theirs[0]).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().object(theirs[1]).unwrap().zone,
        Zone::Graveyard
    );

    // 2. Cycle the second Rapid Decay from hand for {2}.
    let second = in_hand(&engine, p0, rapid_decay()).unwrap();
    let hand_before_cycle = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: second,
                ability_index: 0,
            },
        )
        .unwrap();
    assert_eq!(engine.state().object(second).unwrap().zone, Zone::Graveyard);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before_cycle,
        "cycling discards Rapid Decay and draws 1 replacement card"
    );
}
