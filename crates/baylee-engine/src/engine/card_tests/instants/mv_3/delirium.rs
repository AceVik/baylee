//! `cards/instants/mv_3/delirium.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{DELIRIUM, HILL_GIANT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Delirium (a00e5eb6-b916-4cd6-8ff2-c02f4929abc7) — {1}{B}{R} Instant:
/// "Cast this spell only during an opponent's turn. Tap target creature that
/// player controls. Prevent all combat damage that would be dealt to and dealt
/// by the creature this turn. It deals damage equal to its power to the player."
#[test]
fn delirium_taps_opponent_creature_removes_from_combat_and_deals_power_damage_to_controller() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), mountain(), swamp()])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[DELIRIUM])
        .start();
    keep_mulligans(&mut engine);

    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(P0))],
            },
        )
        .unwrap();

    // P1 gets priority in DeclareAttackers first; pass to P0.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, DELIRIUM);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(
        engine
            .state()
            .object(giant)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 17);
}
