//! `cards/creatures/mv_4/discordant_spirit.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{DISCORDANT_SPIRIT, LIGHTNING_BOLT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Discordant Spirit (ce785306-eaf5-42b3-9840-884cd690ae0a) — {2}{B}{R} 2/2:
/// "Whenever damage is dealt to you, put that many +1/+1 counters on this
/// creature. At the beginning of the end step, remove all +1/+1 counters from
/// this creature."
#[test]
fn discordant_spirit_gains_counters_from_damage_to_controller_and_removes_them_at_end_step() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[DISCORDANT_SPIRIT, mountain()])
        .hand(0, &[LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);

    let spirit = on_battlefield(&engine, P0, DISCORDANT_SPIRIT).unwrap();
    assert_eq!(pt(&engine, spirit), (2, 2));

    // Deal 3 damage to P0 during P1's turn so Discordant Spirit gets 3 +1/+1 counters at P1's end step.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, LIGHTNING_BOLT);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number == 3
            && e.state().turn.phase == Phase::FirstMain
    });

    assert_eq!(pt(&engine, spirit), (5, 5));

    // At the beginning of P0's end step, all +1/+1 counters are removed.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.number == 4
            && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(pt(&engine, spirit), (2, 2));
}
