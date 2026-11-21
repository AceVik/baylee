//! `cards/creatures/mv_1/mogg_conscripts.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn conscripts() -> CardIndex {
    card_index("b015686d-07e1-46c7-8e51-2bec646860d6")
}

fn can_attack_after(spell: Option<CardIndex>) -> bool {
    let mut e = lk::game(SEED, &[conscripts()], &[], spell.as_slice(), &[]);
    let mogg = lk::obj(&e, lk::P0, conscripts());
    if let Some(spell) = spell {
        lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
        if spell == index::SHOCK {
            lk::cast_at(&mut e, lk::P0, spell, &[], &[lk::P1]);
        } else {
            cast_with_floating(&mut e, lk::P0, spell);
            lk::settle(&mut e);
        }
    }
    lk::attackers_offered(&mut e).contains(&mogg)
}

/// Mogg Conscripts ({R} 2/2 Goblin):
/// "This creature can't attack unless you've cast a creature spell this turn."
#[test]
fn mogg_conscripts_requires_a_creature_spell_cast_this_turn_and_resets_next_turn() {
    assert!(!can_attack_after(None));
    assert!(!can_attack_after(Some(index::SHOCK)));
    assert!(can_attack_after(Some(index::RAGING_GOBLIN)));

    // Cast a creature to attack this turn, then verify next turn resets the gate.
    let mut e = lk::game(
        SEED,
        &[conscripts(), mountain()],
        &[],
        &[index::RAGING_GOBLIN],
        &[],
    );
    let mogg = lk::obj(&e, lk::P0, conscripts());
    cast_from_hand(&mut e, lk::P0, index::RAGING_GOBLIN);
    lk::settle(&mut e);

    let _ = lk::attack(&mut e, &[mogg]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 18);

    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    assert!(!lk::attackers_offered(&mut e).contains(&mogg));
}
