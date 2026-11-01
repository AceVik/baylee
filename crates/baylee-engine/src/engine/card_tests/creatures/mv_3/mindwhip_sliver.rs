//! Mindwhip Sliver's granted ability retains its target and timing.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::{legends_kit as lk, partials_w1b_kit as kit};
use baylee_core::generated::index;

fn mindwhip_sliver() -> CardIndex {
    card_index("e9e43839-7cc1-40ff-a8ec-a47316de6440")
}

#[test]
fn granted_ability_targets_either_player_and_discards_only_their_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for victim in [p0, p1] {
        let mut e = lk::game(
            SEED,
            &[mindwhip_sliver(), index::GRIZZLY_BEARS],
            &[],
            &[index::HILL_GIANT],
            &[index::HILL_GIANT],
        );
        let sliver = on_battlefield(&e, p0, mindwhip_sliver()).unwrap();
        let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
        lk::float(&mut e, p0, &[(ManaColor::Colorless, 2)]);
        assert!(
            kit::offered(&e, bears).is_empty(),
            "non-Slivers gain nothing"
        );
        let ability = kit::offered(&e, sliver)[0];
        kit::activate_object(&mut e, p0, sliver, ability);
        lk::aim(&mut e, &[], &[victim]);
        lk::settle(&mut e);
        let other = if victim == p0 { p1 } else { p0 };
        assert!(in_graveyard(&e, victim, index::HILL_GIANT).is_some());
        assert!(in_hand(&e, other, index::HILL_GIANT).is_some());
        assert!(in_graveyard(&e, p0, mindwhip_sliver()).is_some());
    }
}

#[test]
fn granted_ability_is_not_available_on_an_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(SEED, &[mindwhip_sliver()], &[], &[], &[index::HILL_GIANT]);
    let sliver = on_battlefield(&e, p0, mindwhip_sliver()).unwrap();
    reach_their_main_phase(&mut e, p1);
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    lk::float(&mut e, p0, &[(ManaColor::Colorless, 2)]);
    assert!(kit::offered(&e, sliver).is_empty());
}

/// The static grant reaches an opponent's Sliver and belongs to that controller.
#[test]
fn opposing_sliver_also_gets_the_targeted_sorcery_ability() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[mindwhip_sliver()],
        &[index::METALLIC_SLIVER],
        &[index::HILL_GIANT],
        &[],
    );
    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::Colorless, 2)]);
    let sliver = on_battlefield(&e, p1, index::METALLIC_SLIVER).unwrap();
    let ability = kit::offered(&e, sliver)[0];
    kit::activate_object(&mut e, p1, sliver, ability);
    lk::aim(&mut e, &[], &[p0]);
    lk::settle(&mut e);
    assert!(in_graveyard(&e, p1, index::METALLIC_SLIVER).is_some());
    assert!(in_graveyard(&e, p0, index::HILL_GIANT).is_some());
    assert!(on_battlefield(&e, p0, mindwhip_sliver()).is_some());
}
