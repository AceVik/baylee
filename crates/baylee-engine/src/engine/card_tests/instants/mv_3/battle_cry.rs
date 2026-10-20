//! `cards/instants/mv_3/battle_cry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn battle_cry() -> CardIndex {
    card_index("b92afdba-5aea-42c8-bdd8-6ba5ff6cba4f")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Battle Cry (b92afdba-5aea-42c8-bdd8-6ba5ff6cba4f) — {2}{W} Instant:
/// - "Untap all white creatures you control."
/// - "Whenever a creature blocks this turn, it gets +0/+1 until end of turn."
///   (`Coverage::Partial`: delayed trigger on blocking creatures not supported).
#[test]
fn battle_cry_untaps_all_white_creatures_you_control_and_leaves_others_tapped() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                savannah_lions(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[savannah_lions()])
        .hand(0, &[battle_cry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_lions = on_battlefield(&engine, P0, savannah_lions()).expect("my Savannah Lions");
    let my_elf = on_battlefield(&engine, P0, llanowar_elves()).expect("my Llanowar Elves");
    let their_lions = on_battlefield(&engine, P1, savannah_lions()).expect("their Savannah Lions");

    // Tap my Savannah Lions, my Llanowar Elves, and their Savannah Lions.
    for id in [my_lions, my_elf, their_lions] {
        engine
            .dev_state_mut(P0)
            .unwrap()
            .object_mut(id)
            .unwrap()
            .status
            .insert(Status::TAPPED);
    }

    tap_all_mana_but(&mut engine, P0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, P0, battle_cry());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !is_tapped(&engine, my_lions),
        "white creature you control is untapped"
    );
    assert!(
        is_tapped(&engine, my_elf),
        "non-white creature you control stays tapped"
    );
    assert!(
        is_tapped(&engine, their_lions),
        "opponent's white creature stays tapped"
    );
}
