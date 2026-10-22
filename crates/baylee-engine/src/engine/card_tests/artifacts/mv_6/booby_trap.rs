//! `cards/artifacts/mv_6/booby_trap.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn booby_trap() -> CardIndex {
    card_index("3e46381a-7850-4e50-9213-e1a180299977")
}

/// Booby Trap (3e46381a-7850-4e50-9213-e1a180299977) — {6} Artifact (Partial):
/// "As this artifact enters, choose an opponent and a card name... When the chosen player draws a card..., sacrifice this artifact. If you do, this artifact deals 10 damage to that player."
#[test]
fn booby_trap_chooses_opponent_and_card_name_and_triggers_on_opponent_draw() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .hand(0, &[booby_trap()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, booby_trap());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseCardName {
                card: index::GRIZZLY_BEARS,
                face: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, booby_trap()).is_some());

    reach_their_main_phase(&mut engine, P1);
    assert!(in_graveyard(&engine, P0, booby_trap()).is_some());
    assert_eq!(engine.state().players[1].life, 10);
}
