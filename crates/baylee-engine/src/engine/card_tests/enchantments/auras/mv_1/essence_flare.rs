//! `cards/enchantments/auras/mv_1/essence_flare.rs`, played.
//!
//! Oracle:
//! - "Enchant creature"
//! - "Enchanted creature gets +2/+0."
//! - "At the beginning of the upkeep of enchanted creature's controller, put a -0/-1 counter on that creature."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn essence_flare() -> CardIndex {
    card_index("9bdc79c9-c8b0-4db1-89d9-0ca7920b6576")
}

#[test]
fn essence_flare_grants_plus_two_plus_zero_and_adds_m0m1_counter_at_upkeep() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[essence_flare()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    lk::cast_at(&mut engine, P0, essence_flare(), &[bears], &[]);

    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .power,
        Some(4)
    );
    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .toughness,
        Some(2)
    );

    lk::to_step(&mut engine, P1, Step::Upkeep);
    lk::settle(&mut engine);

    assert_eq!(
        minus_zero_one(&engine, bears),
        1,
        "one -0/-1 counter at the upkeep of the enchanted creature's controller"
    );
    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .toughness,
        Some(1)
    );
}

fn minus_zero_one(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    engine
        .state()
        .object(id)
        .unwrap()
        .counters
        .get(baylee_cards_dsl::CounterKind::Minus {
            power: 0,
            toughness: 1,
        })
}

/// "Put a -0/-1 counter on **that creature**": the creature the Aura
/// enchanted, even when the Aura is destroyed with the trigger waiting
/// (CR 113.7a, 608.2h). The Bears keep the counter and lose the +2/+0.
#[test]
fn essence_flare_s_counter_still_lands_when_the_aura_is_destroyed_in_response() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[essence_flare(), index::DISENCHANT],
        &[],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    lk::cast_at(&mut engine, P0, essence_flare(), &[bears], &[]);
    let flare = lk::obj(&engine, P0, essence_flare());

    lk::to_step(&mut engine, P1, Step::Upkeep);
    assert!(
        !stack_is_empty(&engine),
        "the upkeep trigger waits on the stack"
    );
    lk::pass_once(&mut engine);
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
    );
    cast_with_floating(&mut engine, P0, index::DISENCHANT);
    lk::aim(&mut engine, &[flare], &[]);
    lk::settle(&mut engine);

    assert_eq!(
        lk::zone_of(&engine, flare),
        Zone::Graveyard,
        "Disenchant resolved before the trigger"
    );
    assert_eq!(
        minus_zero_one(&engine, bears),
        1,
        "the trigger still put its counter on the creature the Aura had enchanted"
    );
    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .toughness,
        Some(1)
    );
    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .power,
        Some(2),
        "the +2/+0 left with the Aura"
    );
}
