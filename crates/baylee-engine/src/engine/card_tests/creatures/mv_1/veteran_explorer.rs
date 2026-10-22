//! `cards/creatures/mv_1/veteran_explorer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::LIGHTNING_BOLT;

fn veteran_explorer() -> CardIndex {
    card_index("fa9677d2-1535-4c57-baa8-c0408e15da47")
}

/// `Veteran Explorer` ({G}, 1/1 Human Soldier Scout, Partial) searches its
/// controller's library for up to two basic land cards and puts them onto the
/// battlefield untapped when it dies.
#[test]
fn veteran_explorer_dies_and_searches_up_to_two_basic_lands_to_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[veteran_explorer(), baylee_core::generated::index::MOUNTAIN],
        )
        .hand(0, &[LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let explorer = on_battlefield(&engine, p0, veteran_explorer()).expect("Veteran Explorer");
    let forests_before = mine(&engine, p0, forest(), Zone::Battlefield).len();

    cast_from_hand(&mut engine, p0, LIGHTNING_BOLT);
    aim_at(&mut engine, p0, explorer);
    while !stack_is_empty(&engine) {
        pass_until(&mut engine, |e| {
            stack_is_empty(e) || matches!(e.pending(), Pending::ChooseCards { .. })
        });
        if let Pending::ChooseCards {
            player, options, ..
        } = engine.pending().clone()
        {
            let chosen = if player == p0 {
                vec![options[0]]
            } else {
                vec![]
            };
            engine
                .apply(player, PlayerAction::ChooseObjects { objects: chosen })
                .unwrap();
        }
    }

    assert!(in_graveyard(&engine, p0, veteran_explorer()).is_some());
    assert_eq!(
        mine(&engine, p0, forest(), Zone::Battlefield).len(),
        forests_before + 2
    );
}
