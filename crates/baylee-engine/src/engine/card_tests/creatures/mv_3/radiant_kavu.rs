//! `cards/creatures/mv_3/radiant_kavu.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Radiant Kavu — {R}{G}{W} Creature — Kavu 3/3
/// "{R}{G}{W}: Prevent all combat damage blue creatures and black creatures would deal this turn."
#[test]
fn radiant_kavu_prevents_combat_damage_from_blue_and_black_creatures_only() {
    let kavu = card_index("ab46b957-5206-4390-a2f6-aba2e8d1debe");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kavu, mountain(), forest(), plains()])
        .battlefield(
            1,
            &[index::PHANTOM_MONSTER, index::BOG_WRAITH, grizzly_bears()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let kavu_id = on_battlefield(&engine, p0, kavu).expect("Radiant Kavu on board");
    let blue_id = on_battlefield(&engine, p1, index::PHANTOM_MONSTER).unwrap();
    let black_id = on_battlefield(&engine, p1, index::BOG_WRAITH).unwrap();
    let green_id = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    // Advance to P1's main phase, activate Radiant Kavu, and let P1 attack with all three.
    assert!(walk_to_own_main(&mut engine, p1));
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: kavu_id,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (blue_id, Defender::Player(p0)),
                    (black_id, Defender::Player(p0)),
                    (green_id, Defender::Player(p0)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    // Blue (4/1) and Black (3/3) combat damage was prevented; only Grizzly Bears (2/2) dealt 2.
    assert_eq!(engine.state().players[0].life, 18);
}
