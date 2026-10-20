//! `cards/creatures/mv_2/arctic_foxes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn arctic_foxes() -> CardIndex {
    card_index("898f0824-cf4c-4e34-8407-9a32fdae6dcb")
}

/// Arctic Foxes (898f0824-cf4c-4e34-8407-9a32fdae6dcb) — {1}{W} 1/1 Creature — Fox
/// (`Coverage::Partial`: defender snow-land conditional unblockability not supported).
#[test]
fn arctic_foxes_casts_for_one_and_white_as_a_one_one_and_deals_combat_damage() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[arctic_foxes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, arctic_foxes());
    pass_until(&mut engine, stack_is_empty);

    let foxes = on_battlefield(&engine, P0, arctic_foxes()).expect("Arctic Foxes seated");
    assert_eq!(pt(&engine, foxes), (1, 1));

    reach_their_main_phase(&mut engine, P1);
    assert!(walk_to_own_main(&mut engine, P0));
    let _blocks = attack_and_collect_blocks(&mut engine, foxes, P1);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 19);
}
