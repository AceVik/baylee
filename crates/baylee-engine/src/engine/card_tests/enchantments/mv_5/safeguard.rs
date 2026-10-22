//! `cards/enchantments/mv_5/safeguard.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn safeguard() -> CardIndex {
    card_index("e310c3ab-a729-404d-944f-9b477258495c")
}

/// `Safeguard` ({3}{W}{W}, Enchantment, `Coverage::Implemented`):
/// `{2}{W}: Prevent all combat damage that would be dealt by target attacking creature this turn.`
#[test]
fn safeguard_prevents_all_combat_damage_dealt_by_target_attacking_creature_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), safeguard()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p0))],
            },
        )
        .unwrap();

    // Activate Safeguard targeting the attacking Hill Giant.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, safeguard(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();

    // Advance to postcombat main phase -> p0 took 0 combat damage.
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[0].life, 20);
}
