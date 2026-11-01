//! `cards/creatures/mv_1/goblin_cohort.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn cohort() -> CardIndex {
    card_index("88f9a7e3-0b35-43e4-b16a-19c54a1e4e2c")
}

fn can_attack_after(spell: Option<CardIndex>) -> bool {
    let mut e = lk::game(SEED, &[cohort()], &[], spell.as_slice(), &[]);
    let cohort = lk::obj(&e, lk::P0, cohort());
    if let Some(spell) = spell {
        lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
        if spell == index::SHOCK {
            lk::cast_at(&mut e, lk::P0, spell, &[], &[lk::P1]);
        } else {
            cast_with_floating(&mut e, lk::P0, spell);
            lk::settle(&mut e);
        }
    }
    lk::attackers_offered(&mut e).contains(&cohort)
}

#[test]
fn goblin_cohort_needs_a_creature_spell_not_another_spell_this_turn() {
    assert!(!can_attack_after(None));
    assert!(!can_attack_after(Some(index::SHOCK)));
    assert!(can_attack_after(Some(index::RAGING_GOBLIN)));
}
