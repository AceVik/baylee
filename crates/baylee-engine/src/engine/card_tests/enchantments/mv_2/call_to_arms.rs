//! `cards/enchantments/mv_2/call_to_arms.rs`, played.
//!
//! Oracle:
//! - "As this enchantment enters, choose a color and an opponent."
//! - "White creatures get +1/+1 as long as the chosen color is the most common color among nontoken permanents the chosen player controls but isn't tied for most common."
//! - "When the chosen color isn't the most common color among nontoken permanents the chosen player controls or is tied for most common, sacrifice this enchantment."

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
fn call_to_arms_chooses_a_color_on_entry_and_pumps_white_creatures() {
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
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("choose color on entry");
    pass_until(&mut engine, stack_is_empty);

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
