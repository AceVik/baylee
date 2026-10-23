//! `cards/creatures/mv_5/shyft.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn shyft() -> CardIndex {
    card_index("40c3007a-be6c-4400-9139-12b9ca098ac4")
}

/// Shyft (40c3007a-be6c-4400-9139-12b9ca098ac4) — {4}{U} 4/2 Creature — Shapeshifter:
/// (`Coverage::Partial` — upkeep color-change ability not supported.)
#[test]
fn shyft_casts_for_four_and_blue_and_attacks_for_four() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[shyft()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, shyft());
    pass_until(&mut engine, stack_is_empty);

    let id = on_battlefield(&engine, P0, shyft()).unwrap();
    assert_eq!(pt(&engine, id), (4, 2));

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0)
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(id, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 16);
}
