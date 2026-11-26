//! `cards/creatures/mv_1/faerie_squadron.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn faerie_squadron() -> CardIndex {
    card_index("109f7a7d-dc08-4c8f-8065-93c620a9194e")
}

/// Faerie Squadron ({U} 1/1 Creature — Faerie, Kicker {3}{U}):
/// - Unkicked enters as a 1/1 without flying or +1/+1 counters.
/// - Kicked enters with two +1/+1 counters (3/3) and with flying.
#[test]
fn faerie_squadron_enters_1_1_ground_unkicked_and_3_3_flying_when_kicked() {
    let mut unkicked = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[faerie_squadron()])
        .start();
    keep_mulligans(&mut unkicked);
    reach_main_phase(&mut unkicked, P0);

    tap_all_mana(&mut unkicked, P0);
    cast_with_floating(&mut unkicked, P0, faerie_squadron());
    unkicked.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut unkicked, stack_is_empty);

    let f0 = on_battlefield(&unkicked, P0, faerie_squadron()).unwrap();
    assert_eq!(pt(&unkicked, f0), (1, 1));
    assert!(!keywords(&unkicked, f0).contains(KeywordSet::FLYING));
    assert_eq!(counters_on(&unkicked, f0, CounterKind::P1P1), 0);

    let mut kicked = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[faerie_squadron()])
        .start();
    keep_mulligans(&mut kicked);
    reach_main_phase(&mut kicked, P0);

    tap_all_mana(&mut kicked, P0);
    cast_with_floating(&mut kicked, P0, faerie_squadron());
    kicked.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut kicked, stack_is_empty);

    let f1 = on_battlefield(&kicked, P0, faerie_squadron()).unwrap();
    assert_eq!(pt(&kicked, f1), (3, 3));
    assert!(keywords(&kicked, f1).contains(KeywordSet::FLYING));
    assert_eq!(counters_on(&kicked, f1, CounterKind::P1P1), 2);
}
