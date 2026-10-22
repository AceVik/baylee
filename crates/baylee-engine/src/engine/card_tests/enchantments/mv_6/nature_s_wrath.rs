//! `cards/enchantments/mv_6/nature_s_wrath.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn nature_s_wrath() -> CardIndex {
    card_index("5a647f28-6e26-4262-8593-d582e0a691f7")
}

/// Nature's Wrath ({4}{G}{G} Enchantment):
/// - "At the beginning of your upkeep, sacrifice this enchantment unless you pay {G}."
/// - "Whenever a player puts an Island or blue permanent onto the battlefield, that player sacrifices an Island or blue permanent of their choice."
/// - "Whenever a player puts a Swamp or black permanent onto the battlefield, that player sacrifices a Swamp or black permanent of their choice."
#[test]
fn nature_s_wrath_triggers_on_island_and_swamp_entries_and_upkeep_tax() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[nature_s_wrath(), forest()])
        .battlefield(1, &[island(), swamp()])
        .hand(0, &[island()])
        .hand(1, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    reach_main_phase(&mut engine, p0);

    let p0_island = in_hand(&engine, p0, island()).unwrap();
    engine
        .apply(p0, PlayerAction::PlayLand { card: p0_island })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![p0_island],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, island()).is_some());

    // Advance to P1's main phase; P1 plays a Swamp and must sacrifice one of their two Swamps.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    let p1_swamp_hand = in_hand(&engine, p1, swamp()).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::PlayLand {
                card: p1_swamp_hand,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![p1_swamp_hand],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, swamp()).is_some());

    // Advance to P0's upkeep and decline to pay {G}: Nature's Wrath is sacrificed.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, nature_s_wrath()).is_some());
}
