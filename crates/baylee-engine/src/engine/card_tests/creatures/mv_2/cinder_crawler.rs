//! `cards/creatures/mv_2/cinder_crawler.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cinder_crawler() -> CardIndex {
    card_index("3850479f-1c1e-4a72-ae47-6da2997202e2")
}

/// Cinder Crawler (`3850479f-1c1e-4a72-ae47-6da2997202e2`) — {1}{R} Creature — Salamander (Partial):
/// "{R}: This creature gets +1/+0 until end of turn. Activate only if this creature is blocked."
#[test]
fn cinder_crawler_activates_only_when_blocked_to_pump_power() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[cinder_crawler(), mountain()])
        .battlefield(1, &[index::WALL_OF_STONE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let crawler = on_battlefield(&engine, P0, cinder_crawler()).unwrap();
    let wall = on_battlefield(&engine, P1, index::WALL_OF_STONE).unwrap();

    tap_all_mana(&mut engine, P0);
    // Cannot activate in precombat main phase while unblocked.
    assert!(kit::offered(&engine, crawler).is_empty());

    kit::declare_attack(&mut engine, &[crawler], P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, crawler)],
            },
        )
        .unwrap();

    crate::engine::card_tests::legends_kit::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    kit::activate_object(&mut engine, P0, crawler, 0);
    kit::resolve_the_stack(&mut engine, |_, _| None);

    assert_eq!(pt(&engine, crawler), (2, 2));
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, crawler), (1, 2));
}
