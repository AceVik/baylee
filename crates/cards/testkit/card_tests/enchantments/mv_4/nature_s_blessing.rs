//! `cards/enchantments/mv_4/nature_s_blessing.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn nature_s_blessing() -> CardIndex {
    card_index("ae0f65b4-3814-45fe-a869-8585f6869278")
}

/// Nature's Blessing ({2}{G}{W} Enchantment):
/// "{G}{W}, Discard a card: Put a +1/+1 counter on target creature or that creature gains banding, first strike, or trample."
#[test]
fn nature_s_blessing_puts_counter_or_grants_keyword_indefinitely() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                nature_s_blessing(),
                forest(),
                plains(),
                forest(),
                plains(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[forest(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let blessing = on_battlefield(&engine, p0, nature_s_blessing()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let discard1 = in_hand(&engine, p0, forest()).unwrap();
    let discard2 = in_hand(&engine, p0, plains()).unwrap();

    tap_all_mana(&mut engine, p0);

    // First activation: choose mode 0 (+1/+1 counter).
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: blessing,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![discard1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (3, 3));

    // Second activation: choose mode 3 (trample indefinitely).
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: blessing,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![discard2],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(p0, PlayerAction::ChooseMode(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bears).contains(KeywordSet::TRAMPLE));

    // Indefinite effect persists into next turn.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    assert_eq!(pt(&engine, bears), (3, 3));
    assert!(keywords(&engine, bears).contains(KeywordSet::TRAMPLE));
}
