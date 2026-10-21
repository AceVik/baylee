//! `cards/enchantments/mv_4/reclamation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn reclamation() -> CardIndex {
    card_index("ff942899-a70d-4fca-b125-9712aa635fff")
}

/// Reclamation (ff942899-a70d-4fca-b125-9712aa635fff) — {2}{G}{W} Enchantment:
/// "Black creatures can't attack unless their controller sacrifices a land of their choice
/// for each black creature they control that's attacking."
#[test]
fn reclamation_requires_a_land_sacrifice_per_black_attacker_and_blocks_attack_without_lands() {
    // 1. With one land, P0's black Festering Goblin sacrifices that land to attack, while
    // green Grizzly Bears attacks without sacrificing a land, and P1's land is untouched.
    let mut engine = lk::game(
        SEED,
        &[index::FESTERING_GOBLIN, index::GRIZZLY_BEARS, index::SWAMP],
        &[reclamation(), index::FOREST],
        &[],
        &[],
    );
    let goblin = lk::obj(&engine, P0, index::FESTERING_GOBLIN);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let my_swamp = lk::obj(&engine, P0, index::SWAMP);

    let _ = lk::attack(&mut engine, &[goblin, bears]);
    assert_eq!(
        lk::zone_of(&engine, my_swamp),
        Zone::Graveyard,
        "attacking with a black creature sacrifices one of its controller's lands"
    );
    assert_eq!(
        all_on_battlefield(&engine, P1, index::FOREST).len(),
        1,
        "Reclamation's controller's lands are untouched"
    );
    assert!(lk::tapped(&engine, goblin) && lk::tapped(&engine, bears));

    // 2. Without any land, a black creature cannot attack, while a green creature still can.
    let mut engine2 = lk::game(
        SEED,
        &[index::FESTERING_GOBLIN, index::GRIZZLY_BEARS],
        &[reclamation()],
        &[],
        &[],
    );
    let goblin2 = lk::obj(&engine2, P0, index::FESTERING_GOBLIN);
    let bears2 = lk::obj(&engine2, P0, index::GRIZZLY_BEARS);
    let offered = lk::attackers_offered(&mut engine2);
    assert!(
        !offered.contains(&goblin2),
        "black creature cannot attack when its controller controls no lands"
    );
    assert!(
        offered.contains(&bears2),
        "non-black creature can still attack without lands"
    );
}
