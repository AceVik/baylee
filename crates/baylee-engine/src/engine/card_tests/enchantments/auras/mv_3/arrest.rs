//! `cards/enchantments/auras/mv_3/arrest.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "81728b98-8cf9-4734-a318-69184bb4d15c"
fn arrest() -> CardIndex {
    card_index("81728b98-8cf9-4734-a318-69184bb4d15c")
}

/// Arrest ({2}{W} Enchantment — Aura):
/// "Enchant creature"
/// "Enchanted creature can't attack or block, and its activated abilities can't be activated."
#[test]
fn arrest_prevents_attacking_blocking_and_activating_abilities() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::LLANOWAR_ELVES, index::PRODIGAL_SORCERER],
        &[arrest()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let elves = lk::obj(&engine, P1, index::LLANOWAR_ELVES);
    let sorcerer = lk::obj(&engine, P1, index::PRODIGAL_SORCERER);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
    );
    lk::cast_at(&mut engine, P0, arrest(), &[elves], &[]);

    // Enchanted Llanowar Elves cannot block when Grizzly Bears attacks.
    let offer = lk::attack(&mut engine, &[bears]);
    assert!(
        !lk::may_block(&offer, elves, bears),
        "enchanted Llanowar Elves cannot block"
    );
    assert!(
        lk::may_block(&offer, sorcerer, bears),
        "unenchanted Prodigal Sorcerer can still block"
    );
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);

    // On P1's turn, enchanted Llanowar Elves cannot activate its mana ability and cannot attack.
    reach_their_main_phase(&mut engine, P1);
    assert!(
        !lk::offered(&engine, index::LLANOWAR_ELVES, P1, 0),
        "Arrest stops enchanted Llanowar Elves from activating its mana ability"
    );
    assert!(
        lk::offered(&engine, index::PRODIGAL_SORCERER, P1, 0),
        "unenchanted Prodigal Sorcerer can still activate its ability"
    );

    let able = lk::attackers_offered(&mut engine);
    assert!(
        !able.contains(&elves),
        "enchanted Llanowar Elves cannot attack"
    );
    assert!(
        able.contains(&sorcerer),
        "unenchanted Prodigal Sorcerer can attack"
    );
}
