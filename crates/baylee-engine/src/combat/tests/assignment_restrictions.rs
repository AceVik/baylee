//! A masked creature reveals only if it would assign damage. Suppressing
//! assignments must therefore be checked before that preview too.

use super::{P0, P1, attack, block, creature, empty_state, permit};
use crate::combat::damage::{collect_combat_damage, prepare_damage_step};
use crate::object::{Rider, Status};
use baylee_cards_dsl::{KeywordSet, Modifier};

#[test]
fn no_assignment_does_not_reveal_a_masked_attacker_or_blocker() {
    for attacking in [true, false] {
        let mut state = empty_state();
        let masked = creature(&mut state, P0, 5, 5, KeywordSet::EMPTY);
        let other = creature(&mut state, P1, 0, 9, KeywordSet::EMPTY);
        let object = state.object_mut(masked).unwrap();
        object.original_base = Some(std::sync::Arc::clone(&object.base));
        let base = object.base_mut();
        base.power = Some(2);
        base.toughness = Some(2);
        object.status.insert(Status::FACE_DOWN);
        object.riders.push(Rider::Masked);
        permit(&mut state, masked, Modifier::AssignsNoCombatDamage);
        if attacking {
            attack(&mut state, masked, P1);
            block(&mut state, other, masked);
        } else {
            attack(&mut state, other, P0);
            block(&mut state, masked, other);
        }
        assert!(!prepare_damage_step(&mut state, false));
        assert!(collect_combat_damage(&mut state, false).is_empty());
        assert!(
            state
                .object(masked)
                .unwrap()
                .status
                .contains(Status::FACE_DOWN)
        );
    }
}
