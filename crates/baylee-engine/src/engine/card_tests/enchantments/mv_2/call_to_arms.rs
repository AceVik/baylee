//! `cards/enchantments/mv_2/call_to_arms.rs`, played.
//!
//! Oracle:
//! - "As this enchantment enters, choose a color and an opponent."
//! - "White creatures get +1/+1 as long as the chosen color is the most common color among nontoken permanents the chosen player controls but isn't tied for most common."
//! - "When the chosen color isn't the most common color among nontoken permanents the chosen player controls or is tied for most common, sacrifice this enchantment."
//!
//! The card is Partial: the +1/+1 is unconditional and the sacrifice is
//! missing. The board here makes the condition true — the chosen opponent's
//! only nontoken permanent is a white Savannah Lions and the chosen color is
//! white — so what is asserted is what the printed card does too. A board
//! where the condition fails is not played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "72eaaa73-3263-46e9-aa86-5862d3aed20a"
fn call_to_arms() -> CardIndex {
    card_index("72eaaa73-3263-46e9-aa86-5862d3aed20a")
}

#[test]
fn call_to_arms_chooses_a_color_and_an_opponent_and_pumps_white_creatures() {
    let mut engine = lk::game(
        SEED,
        &[index::SAVANNAH_LIONS, index::GRIZZLY_BEARS],
        &[index::SAVANNAH_LIONS],
        &[call_to_arms()],
        &[],
    );
    let my_lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let their_lions = lk::obj(&engine, P1, index::SAVANNAH_LIONS);

    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);
    cast_with_floating(&mut engine, P0, call_to_arms());
    let (mut colour, mut opponent) = (false, false);
    for _ in 0..12 {
        match engine.pending().clone() {
            Pending::ChooseColor { player, .. } => {
                assert_eq!(player, P0, "the caster chooses the color");
                colour = true;
                engine
                    .apply(P0, PlayerAction::ChooseColor(ManaColor::White))
                    .expect("choose white on entry");
            }
            Pending::ChoosePlayer { player, options } => {
                assert_eq!(player, P0, "the caster chooses the opponent");
                assert_eq!(options, vec![P1], "an opponent, never the caster");
                opponent = true;
                engine
                    .apply(P0, PlayerAction::ChoosePlayer(P1))
                    .expect("choose the opponent on entry");
            }
            Pending::Priority { player, .. } if !stack_is_empty(&engine) => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            _ => break,
        }
    }
    assert!(stack_is_empty(&engine), "Call to Arms resolved");
    assert!(
        colour && opponent,
        "a color and an opponent are chosen as it enters"
    );

    assert_eq!(pt(&engine, my_lions), (3, 2), "white creature gets +1/+1");
    assert_eq!(
        pt(&engine, their_lions),
        (3, 2),
        "opponent's white creature also gets +1/+1"
    );
    assert_eq!(
        pt(&engine, my_bears),
        (2, 2),
        "non-white creature is unaffected"
    );
}
