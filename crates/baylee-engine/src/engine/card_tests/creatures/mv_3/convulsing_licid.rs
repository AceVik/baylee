//! `cards/creatures/mv_3/convulsing_licid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn convulsing_licid() -> CardIndex {
    card_index("893c88cb-72ba-4f54-809c-bfac4d6efb74")
}

/// Convulsing Licid (`893c88cb-72ba-4f54-809c-bfac4d6efb74`) — {2}{R} 2/2 Creature — Licid (Partial):
/// "{R}, {T}: Target creature can't block this turn."
#[test]
fn convulsing_licid_prevents_target_creature_from_blocking() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), convulsing_licid(), index::GRIZZLY_BEARS])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, convulsing_licid(), 0);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[1].life, 18);
}
