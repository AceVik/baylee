//! `cards/enchantments/mv_3/megrim.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn megrim() -> CardIndex {
    card_index("633ad9e2-9f55-4a1c-9248-661ad4b0e1dc")
}

fn mind_peel() -> CardIndex {
    card_index("4f424d94-1544-4086-a57d-2add17332094")
}

/// Megrim (`633ad9e2-9f55-4a1c-9248-661ad4b0e1dc`) — {2}{B} Enchantment (Implemented):
/// "Whenever an opponent discards a card, this enchantment deals 2 damage to that player."
#[test]
fn megrim_deals_two_damage_to_opponent_when_that_opponent_discards() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[megrim(), swamp()])
        .hand(0, &[mind_peel()])
        .hand(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, mind_peel());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    {
        engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 18);
}

#[test]
fn your_discard_does_not_trigger_megrim() {
    let mut e = lk::game(
        SEED,
        &[megrim()],
        &[],
        &[mind_peel(), index::GRIZZLY_BEARS],
        &[],
    );
    lk::float(&mut e, P0, &[(ManaColor::Black, 1)]);
    cast_with_floating(&mut e, P0, mind_peel());
    e.apply(P0, PlayerAction::ChoosePlayer(P0)).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let bear = in_hand(&e, P0, index::GRIZZLY_BEARS).unwrap();
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![bear],
        },
    )
    .unwrap();
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, P0), 20);
    assert_eq!(lk::life(&e, P1), 20);
}
