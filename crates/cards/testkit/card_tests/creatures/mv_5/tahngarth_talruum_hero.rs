//! `cards/creatures/mv_5/tahngarth_talruum_hero.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn tahngarth_talruum_hero() -> CardIndex {
    card_index("c5b4131e-13aa-4a22-b8f5-e5f9d366cb4f")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Tahngarth, Talruum Hero (c5b4131e-13aa-4a22-b8f5-e5f9d366cb4f) — `{3}{R}{R}` 4/4
/// Legendary Creature — Minotaur Warrior:
/// - Vigilance
/// - `{1}{R}`, `{T}`: Tahngarth deals damage equal to its power to target creature.
///   That creature deals damage equal to its power to Tahngarth.
#[test]
fn tahngarth_attacks_untapped_with_vigilance_and_trades_power_damage_with_target() {
    let lands = vec![mountain(); 7];
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &lands)
        .battlefield(1, &[hill_giant()])
        .hand(0, &[tahngarth_talruum_hero()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, tahngarth_talruum_hero());
    pass_until(&mut engine, stack_is_empty);

    let tahngarth =
        on_battlefield(&engine, P0, tahngarth_talruum_hero()).expect("Tahngarth entered");
    let giant = on_battlefield(&engine, P1, hill_giant()).unwrap();
    assert_eq!(pt(&engine, tahngarth), (4, 4));
    assert!(keywords(&engine, tahngarth).contains(KeywordSet::VIGILANCE));

    // Next turn cycle: attack unblocked (stays untapped via vigilance), then activate `{1}{R}, {T}`.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(tahngarth, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[1].life, 16);
    assert!(
        !engine
            .state()
            .object(tahngarth)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, tahngarth_talruum_hero(), 0);
    aim_at(&mut engine, P0, giant);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P1, hill_giant()).is_some(),
        "4 damage kills 3/3 Hill Giant"
    );
    assert_eq!(
        engine.state().object(tahngarth).unwrap().damage,
        3,
        "Hill Giant deals 3 damage back before SBAs destroy it"
    );
    assert!(on_battlefield(&engine, P0, tahngarth_talruum_hero()).is_some());
}
