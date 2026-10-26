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

/// P0's Grizzly Bears, killed and danced back: the returned object.
fn danced_bears(hand: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = lk::game(SEED, &[index::GRIZZLY_BEARS], &[], hand, &[]);
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
    assert!(
        lk::tapped(&engine, reanimated),
        "Dance returns the creature tapped"
    );
    (engine, reanimated)
}

/// Walks to P0's next upkeep with Dance's trigger on the stack.
fn to_my_upkeep(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| e.state().turn.active != P0);
    pass_until(engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
}

#[test]
fn dance_of_the_dead_reanimates_creature_pumps_it_and_sacrifices_it_when_aura_leaves() {
    let (mut engine, reanimated) = danced_bears(&[dance_of_the_dead(), index::DISENCHANT]);
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

#[test]
fn dance_of_the_dead_keeps_its_creature_tapped_unless_the_upkeep_price_is_paid() {
    let (mut engine, reanimated) = danced_bears(&[dance_of_the_dead()]);

    // Next upkeep: the untap step left it tapped; decline the {1}{B}.
    to_my_upkeep(&mut engine);
    assert!(
        lk::tapped(&engine, reanimated),
        "enchanted creature doesn't untap during its controller's untap step"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, false);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        lk::tapped(&engine, reanimated),
        "not paid: the creature stays tapped"
    );

    // The upkeep after: pay {1}{B} and it untaps.
    to_my_upkeep(&mut engine);
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, true);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !lk::tapped(&engine, reanimated),
        "if the player pays {{1}}{{B}}, untap that creature"
    );
    assert_eq!(lk::pool_total(&engine, P0), 0, "the {{1}}{{B}} was spent");
}
