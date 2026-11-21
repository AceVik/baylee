//! `cards/artifacts/mv_3/evolution_vat.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn evolution_vat() -> CardIndex {
    card_index("854e0e2c-06dc-4166-a48a-107e939ca91e")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Evolution Vat (854e0e2c-06dc-4166-a48a-107e939ca91e) — {3} Artifact:
/// "{3}, {T}: Tap target creature and put a +1/+1 counter on it. Until end of
///  turn, that creature gains '{2}{G}{U}: Double the number of +1/+1 counters
///  on this creature.'"
///
/// Taps the target creature, puts a +1/+1 counter on it, grants it the
/// `{2}{G}{U}` doubling ability (which doubles 1 -> 2 and 2 -> 4), and the
/// granted ability expires at end of turn.
#[test]
fn evolution_vat_taps_adds_counter_and_grants_doubling_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                evolution_vat(),
                grizzly_bears(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                island(),
                island(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vat = on_battlefield(&engine, p0, evolution_vat()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    assert!(!is_tapped(&engine, bears));
    assert_eq!(counters_on(&engine, bears, CounterKind::P1P1), 0);

    // Pay {3} for Evolution Vat using only 3 Forests so both Islands stay untapped for {2}{G}{U}.
    let my_forests = forests(&engine, p0, 9);
    tap_mana_where(&mut engine, p0, |id| my_forests[..3].contains(&id));
    activate(&mut engine, p0, evolution_vat(), 0);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, vat));
    assert!(is_tapped(&engine, bears), "target creature was tapped");
    assert_eq!(counters_on(&engine, bears, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, bears), (3, 3));

    // Activate granted {2}{G}{U} twice (1 -> 2 -> 4), tapping exact {3G + 1U} per activation.
    let my_islands = all_on_battlefield(&engine, p0, island());
    for (round, expected) in [(0usize, 2u16), (1usize, 4u16)] {
        let start = 3 + round * 3;
        tap_mana_where(&mut engine, p0, |id| {
            my_forests[start..start + 3].contains(&id) || id == my_islands[round]
        });
        let grant_idx = priority_offer(&engine)
            .abilities
            .iter()
            .find_map(|&(src, idx)| (src == bears).then_some(idx))
            .expect("granted doubling ability is offered on target creature");
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: bears,
                    ability_index: grant_idx,
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(counters_on(&engine, bears, CounterKind::P1P1), expected);
    }
    assert_eq!(pt(&engine, bears), (6, 6), "2/2 with four +1/+1 counters");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    // Next turn: the granted doubling ability wore off at end of turn.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    assert!(
        !priority_offer(&engine)
            .abilities
            .iter()
            .any(|&(src, _)| src == bears),
        "granted doubling ability ended at end of turn"
    );
    assert_eq!(pt(&engine, bears), (6, 6), "the +1/+1 counters remain");
}
