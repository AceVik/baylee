//! `cards/artifacts/mv_4/geth_s_grimoire.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn geth_s_grimoire() -> CardIndex {
    card_index("ef809e99-34a2-4471-8269-f56bf8037686")
}

fn miscalculation() -> CardIndex {
    card_index("9078db26-3a54-4a11-9aae-e45c38da7226")
}

/// Geth's Grimoire (ef809e99-34a2-4471-8269-f56bf8037686) — {4} Artifact — Book:
/// "Whenever an opponent discards a card, you may draw a card."
///
/// Does NOT trigger when you discard (cycling Miscalculation); triggers on each
/// opponent discard, letting you accept (draw 1) or decline (draw 0).
#[test]
fn geth_s_grimoire_may_draw_on_opponent_discard_and_ignores_own_discard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[geth_s_grimoire(), island(), island()])
        .hand(0, &[miscalculation()])
        .battlefield(1, &[island(), island(), island(), island()])
        .hand(1, &[miscalculation(), miscalculation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // 1. p0 discards via Cycling -> Geth's Grimoire must NOT trigger (only 1 cycling draw).
    let my_misc = in_hand(&engine, p0, miscalculation()).unwrap();
    let p0_lib_before = library_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: my_misc,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        p0_lib_before - 1,
        "own discard did not trigger Geth's Grimoire (only the 1 cycling draw)"
    );

    // 2. Opponent's turn: p1 cycles first Miscalculation -> triggers Grimoire; p0 accepts -> draws 1.
    reach_their_main_phase(&mut engine, p1);
    let p0_hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let their_misc_1 = in_hand(&engine, p1, miscalculation()).unwrap();
    let p1_lands = lands_of(&engine, p1);
    tap_mana_where(&mut engine, p1, |id| p1_lands[..2].contains(&id));
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: their_misc_1,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(engine.pending().asked(), Some(p0));
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        p0_hand_before + 1,
        "accepted Geth's Grimoire draw on opponent discard"
    );

    // 3. p1 cycles second Miscalculation -> triggers Grimoire; p0 declines -> draws 0.
    let their_misc_2 = in_hand(&engine, p1, miscalculation()).unwrap();
    tap_mana_where(&mut engine, p1, |id| p1_lands[2..4].contains(&id));
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: their_misc_2,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        p0_hand_before + 1,
        "declining 'you may draw a card' draws nothing"
    );
}
