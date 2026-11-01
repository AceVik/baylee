//! `cards/enchantments/auras/mv_2/pacifism.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "5f5e0b10-c8cf-450c-bfd3-bcb0528ec330"
fn pacifism() -> CardIndex {
    card_index("5f5e0b10-c8cf-450c-bfd3-bcb0528ec330")
}

#[test]
fn pacifism_prevents_enchanted_creature_from_attacking_or_blocking() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[pacifism()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
    );
    lk::cast_at(&mut engine, P0, pacifism(), &[giant], &[]);

    // Enchanted Hill Giant cannot block when Grizzly Bears attacks.
    let offer = lk::attack(&mut engine, &[bears]);
    assert!(
        !lk::may_block(&offer, giant, bears),
        "enchanted creature cannot block"
    );
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);

    // Enchanted Hill Giant cannot attack on P1's turn.
    reach_their_main_phase(&mut engine, P1);
    let able = lk::attackers_offered(&mut engine);
    assert!(!able.contains(&giant), "enchanted creature cannot attack");
}

/// "Enchanted creature": the opponent's other creature is free to block and
/// to attack, only the enchanted one is not.
#[test]
fn pacifism_leaves_the_enchanted_creatures_neighbour_free_to_block_and_attack() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::HILL_GIANT, index::GRIZZLY_BEARS],
        &[pacifism()],
        &[],
    );
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let their_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
    );
    lk::cast_at(&mut engine, P0, pacifism(), &[giant], &[]);

    let offer = lk::attack(&mut engine, &[my_bears]);
    assert!(lk::may_block(&offer, their_bears, my_bears));
    assert!(!lk::may_block(&offer, giant, my_bears));
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);

    reach_their_main_phase(&mut engine, P1);
    let able = lk::attackers_offered(&mut engine);
    assert!(able.contains(&their_bears));
    assert!(!able.contains(&giant));
}
