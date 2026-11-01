//! `cards/creatures/mv_5/ethereal_champion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::LIGHTNING_BOLT;

const P0: PlayerId = PlayerId::new(0);

fn ethereal_champion() -> CardIndex {
    card_index("f203671b-76f7-4be0-80a9-826fbb450fe7")
}

/// Ethereal Champion (f203671b-76f7-4be0-80a9-826fbb450fe7) — {2}{W}{W}{W} 3/4:
/// "Pay 1 life: Prevent the next 1 damage that would be dealt to this creature
/// this turn."
#[test]
fn ethereal_champion_pays_one_life_to_prevent_next_one_damage_to_itself() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[ethereal_champion(), mountain()])
        .hand(0, &[LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let champ = on_battlefield(&engine, P0, ethereal_champion()).unwrap();

    // Activate once (paying 1 life) to prevent the next 1 damage to Ethereal Champion.
    activate(&mut engine, P0, ethereal_champion(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 19);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, LIGHTNING_BOLT);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![champ],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(champ).unwrap().damage, 2);
}

/// "this turn": the shield is gone on the opponent's turn, so a Lightning
/// Bolt then deals all 3 damage.
#[test]
fn ethereal_champion_shield_expires_at_end_of_turn() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[ethereal_champion(), mountain()])
        .hand(0, &[LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let champ = on_battlefield(&engine, P0, ethereal_champion()).unwrap();
    activate(&mut engine, P0, ethereal_champion(), 0);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        e.state().turn.active == PlayerId::new(1) && e.state().turn.phase == Phase::FirstMain
    });
    engine
        .apply(PlayerId::new(1), PlayerAction::PassPriority)
        .unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, LIGHTNING_BOLT);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![champ],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(champ).unwrap().damage, 3);
}
