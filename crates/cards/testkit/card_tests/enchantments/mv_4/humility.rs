//! `cards/enchantments/mv_4/humility.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn humility() -> CardIndex {
    card_index("ed7bdb3e-5c51-4547-9266-76a791e0b2b0")
}

/// Humility (`ed7bdb3e-5c51-4547-9266-76a791e0b2b0`) — {2}{W}{W} Enchantment:
/// "All creatures lose all abilities and have base power and toughness 1/1."
#[test]
fn humility_removes_all_abilities_and_sets_all_creatures_to_one_one() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::SERRA_ANGEL,
                index::PRODIGAL_SORCERER,
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[humility()])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, humility());
    pass_until(&mut engine, stack_is_empty);
    let sorcerer = on_battlefield(&engine, P0, index::PRODIGAL_SORCERER).unwrap();
    assert!(!ability_offered(&engine, sorcerer, 0));
    let angel = on_battlefield(&engine, P0, index::SERRA_ANGEL).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();

    assert_eq!(pt(&engine, angel), (1, 1));
    assert_eq!(pt(&engine, giant), (1, 1));
    assert!(!keywords(&engine, angel).contains(KeywordSet::FLYING));
    assert!(!keywords(&engine, angel).contains(KeywordSet::VIGILANCE));
}
