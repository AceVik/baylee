//! `cards/enchantments/auras/mv_2/dance_of_the_dead.rs`, played.
//!
//! Oracle:
//! - "Enchant creature card in a graveyard"
//! - "When this Aura enters, if it's on the battlefield, it loses 'enchant creature card in a graveyard' and gains 'enchant creature put onto the battlefield with this Aura.' Put enchanted creature card onto the battlefield tapped under your control and attach this Aura to it. When this Aura leaves the battlefield, that creature's controller sacrifices it."
//! - "Enchanted creature gets +1/+1 and doesn't untap during its controller's untap step."
//! - "At the beginning of the upkeep of enchanted creature's controller, that player may pay {1}{B}. If the player does, untap that creature."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "15d5b198-edbe-40f3-87e2-d72f7f0c58ab"
fn dance_of_the_dead() -> CardIndex {
    card_index("15d5b198-edbe-40f3-87e2-d72f7f0c58ab")
}

#[test]
fn dance_of_the_dead_reanimates_creature_pumps_it_and_sacrifices_it_when_aura_leaves() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[dance_of_the_dead(), index::DISENCHANT],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    bury(&mut engine, &[bears]);
    let dead_bears = in_graveyard(&engine, P0, index::GRIZZLY_BEARS).expect("in graveyard");

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut engine, P0, dance_of_the_dead());
    lk::aim(&mut engine, &[dead_bears], &[]);
    pass_until(&mut engine, stack_is_empty);

    let reanimated =
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).expect("Grizzly Bears returned");
    assert_eq!(
        pt(&engine, reanimated),
        (3, 3),
        "enchanted creature gets +1/+1 from Dance of the Dead"
    );

    // Destroy Dance of the Dead: its leave trigger sacrifices the reanimated creature.
    let dance = on_battlefield(&engine, P0, dance_of_the_dead()).expect("Dance on battlefield");
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
    );
    cast_with_floating(&mut engine, P0, index::DISENCHANT);
    lk::aim(&mut engine, &[dance], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none(),
        "when Dance of the Dead leaves the battlefield, that creature's controller sacrifices it"
    );
}
