//! `cards/creatures/mv_6/jareth_leonine_titan.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Jareth, Leonine Titan (`019aa6a6-7be8-4c3d-8f3e-4ccce2b9c3d3`) — {3}{W}{W}{W} Legendary Creature — Cat Giant 4/7:
/// "Whenever Jareth blocks, it gets +7/+7 until end of turn.
/// {W}: Jareth gains protection from the color of your choice until end of turn."
#[test]
fn jareth_leonine_titan_pumps_when_blocking_and_gains_protection_from_chosen_color() {
    let jareth_card = card_index("019aa6a6-7be8-4c3d-8f3e-4ccce2b9c3d3");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[jareth_card, plains()])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let jareth = on_battlefield(&engine, p0, jareth_card).unwrap();
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    assert_eq!(pt(&engine, jareth), (4, 7));

    // Activate {W}: Jareth gains protection from red until end of turn.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, jareth_card, 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Advance to p1's turn and attack with Hill Giant; Jareth blocks and gets +7/+7 -> 11/14.
    reach_their_main_phase(&mut engine, p1);
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

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(jareth, giant)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, jareth), (11, 14));

    reach_their_main_phase(&mut engine, p1);
    assert!(in_graveyard(&engine, p1, index::HILL_GIANT).is_some());
}

#[test]
fn jareth_leonine_titan_protection_from_chosen_color_excludes_matching_spells() {
    let jareth_card = card_index("019aa6a6-7be8-4c3d-8f3e-4ccce2b9c3d3");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[jareth_card, plains()])
        .battlefield(1, &[index::HILL_GIANT, index::MOUNTAIN])
        .hand(1, &[index::SHOCK])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let jareth = on_battlefield(&engine, p0, jareth_card).unwrap();
    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, jareth_card, 1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, index::SHOCK);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected Shock targets")
    };
    assert!(!options.contains(&jareth));
    assert!(options.contains(&giant));
}
