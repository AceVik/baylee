//! `cards/enchantments/mv_1/burgeoning.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn burgeoning() -> CardIndex {
    card_index("88cf4c1a-5863-45fc-b76d-7ad33b175b2b")
}

/// Burgeoning (`88cf4c1a-5863-45fc-b76d-7ad33b175b2b`) — {G} Enchantment (Implemented):
/// "Whenever an opponent plays a land, you may put a land card from your hand onto the battlefield."
#[test]
fn burgeoning_puts_land_from_hand_when_opponent_plays_land() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[burgeoning(), plains()])
        .hand(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, burgeoning());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, P1);
    let p1_island = in_hand(&engine, P1, island()).unwrap();
    let p0_plains = in_hand(&engine, P0, plains()).unwrap();
    engine
        .apply(P1, PlayerAction::PlayLand { card: p1_island })
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_plains],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, plains()).is_some());
}
