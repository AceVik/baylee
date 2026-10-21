//! Heart Wolf — `{3}{R}` 2/2 Creature — Wolf (Homelands)
//!
//! First strike
//! `{T}`: Target Dwarf creature gets +2/+0 and gains first strike until end of
//! turn. When that creature leaves the battlefield this turn, sacrifice this
//! creature. Activate only during combat.

#[allow(clippy::wildcard_imports)]
use super::*;

fn heart_wolf() -> CardIndex {
    card_index("2f50c45f-058d-41fb-9965-08c2e0edc8ac")
}

fn dwarven_trader() -> CardIndex {
    card_index("b40c5f3e-7080-4828-9215-79727c7a911f")
}

#[test]
fn heart_wolf_pumps_target_dwarf_during_combat_and_sacrifices_itself_when_that_dwarf_leaves() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[heart_wolf(), dwarven_trader(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wolf = on_battlefield(&engine, p0, heart_wolf()).unwrap();
    let dwarf = on_battlefield(&engine, p0, dwarven_trader()).unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();

    // Cannot activate during main phase (only during combat).
    assert!(
        !priority_offer(&engine).abilities.contains(&(wolf, 0)),
        "Heart Wolf ability cannot be activated outside combat"
    );

    // Advance into Combat phase: now Heart Wolf can activate!
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::Combat);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wolf,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![dwarf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, dwarf), (3, 1));
    assert!(keywords(&engine, dwarf).contains(KeywordSet::FIRST_STRIKE));

    // Destroy the Dwarf with Lightning Bolt this turn: Heart Wolf sacrifices itself!
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![dwarf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(dwarf).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state().object(wolf).unwrap().zone,
        Zone::Graveyard,
        "Heart Wolf sacrificed itself when the targeted Dwarf left the battlefield this turn"
    );
}
