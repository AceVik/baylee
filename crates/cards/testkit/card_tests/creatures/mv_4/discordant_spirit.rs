//! `cards/creatures/mv_4/discordant_spirit.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::LIGHTNING_BOLT;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn discordant_spirit() -> CardIndex {
    card_index("ce785306-eaf5-42b3-9840-884cd690ae0a")
}

/// Discordant Spirit (ce785306-eaf5-42b3-9840-884cd690ae0a) — {2}{B}{R} 2/2:
/// "At the beginning of each end step, if it's an opponent's turn, put a +1/+1
/// counter on this creature for each 1 damage dealt to you this turn. At the
/// beginning of your end step, remove all +1/+1 counters from this creature."
#[test]
fn discordant_spirit_gains_counters_from_damage_to_controller_and_removes_them_at_end_step() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[discordant_spirit(), mountain()])
        .hand(0, &[LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);

    let spirit = on_battlefield(&engine, P0, discordant_spirit()).unwrap();
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

/// Damage to its controller on the controller's own turn: no counters at the
/// opponent's end step, because "damage dealt to you this turn" is read on
/// that turn.
#[test]
fn discordant_spirit_gets_no_counters_for_damage_taken_on_its_controllers_turn() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[discordant_spirit(), mountain()])
        .hand(0, &[LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let spirit = on_battlefield(&engine, P0, discordant_spirit()).unwrap();
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
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 17);

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number == 3
            && e.state().turn.phase == Phase::FirstMain
    });
    assert_eq!(pt(&engine, spirit), (2, 2));
}

/// "damage dealt to you": damage dealt to the opponent during the opponent's
/// own turn gives no counters.
#[test]
fn discordant_spirit_gets_no_counters_for_damage_dealt_to_the_opponent() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[discordant_spirit(), mountain()])
        .hand(0, &[LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);

    let spirit = on_battlefield(&engine, P0, discordant_spirit()).unwrap();
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
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.number == 3
            && e.state().turn.phase == Phase::FirstMain
    });

    assert_eq!(engine.state().players[1].life, 17);
    assert_eq!(pt(&engine, spirit), (2, 2));
}
