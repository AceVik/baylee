//! `cards/creatures/mv_3/cho_arrim_legate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn subject_card() -> CardIndex {
    card_index("780c9cb2-9d98-4789-87e0-cb81db477204")
}

/// Cho-Arrim Legate — {2}{W} 1/2 Creature — Human Soldier:
/// "Protection from black" (Partial: conditional free cast unimplemented).
#[test]
fn cho_arrim_legate_enters_with_protection_from_black() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                swamp(),
                swamp(),
                swamp(),
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[subject_card(), index::DARK_BANISHING])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, subject_card());
    pass_until(&mut engine, stack_is_empty);

    let legate = on_battlefield(&engine, P0, subject_card()).unwrap();
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&engine, legate), (1, 2));

    cast_from_hand(&mut engine, P0, index::DARK_BANISHING);
    let menu = aim_at(&mut engine, P0, bears);
    assert!(!menu.contains(&legate));
}
