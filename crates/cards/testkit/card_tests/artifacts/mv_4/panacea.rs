//! `cards/artifacts/mv_4/panacea.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "08ee7879-3c9d-43c6-a44e-f81180f938fd"
fn panacea() -> CardIndex {
    card_index("08ee7879-3c9d-43c6-a44e-f81180f938fd")
}

/// `Panacea` (`Coverage::Generated`):
/// "{X}{X}, {T}: Prevent the next X damage that would be dealt to any target this turn."
///
/// Verifies that activating `Panacea` with `X = 2` (`{4}, {T}`) prevents 2 of the
/// 3 damage from `Lightning Bolt`, leaving 1 damage dealt.
#[test]
fn panacea_prevents_x_damage_for_double_x_mana() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9402, mountain())
        .battlefield(
            0,
            &[panacea(), mountain(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let artifact = on_battlefield(&engine, p0, panacea()).expect("Panacea on battlefield");
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: artifact,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // P1 casts Lightning Bolt (3 damage) at P0; 2 damage is prevented.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        19,
        "Panacea with X=2 prevents 2 of Lightning Bolt's 3 damage"
    );
}
