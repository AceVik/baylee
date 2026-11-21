//! `cards/sorceries/mv_5/jagged_lightning.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn jagged_lightning() -> CardIndex {
    card_index("b5d5cfc0-17e0-42d1-b5bd-9f27b11191f9")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Jagged Lightning (b5d5cfc0-17e0-42d1-b5bd-9f27b11191f9) — {3}{R}{R} Sorcery:
/// "Jagged Lightning deals 3 damage to each of two target creatures."
#[test]
fn jagged_lightning_deals_three_damage_to_each_of_two_target_creatures() {
    let mut engine = lk::game(
        SEED,
        &[
            mountain(),
            mountain(),
            mountain(),
            mountain(),
            mountain(),
            sol_ring(),
        ],
        &[hill_giant(), fangren_hunter(), grizzly_bears()],
        &[jagged_lightning()],
        &[],
    );
    let ring = lk::obj(&engine, P0, sol_ring());
    let giant = lk::obj(&engine, P1, hill_giant());
    let hunter = lk::obj(&engine, P1, fangren_hunter());
    let bears = lk::obj(&engine, P1, grizzly_bears());

    tap_mana_except(&mut engine, P0, ring);
    cast_with_floating(&mut engine, P0, jagged_lightning());
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (2, 2));
    assert!(options.contains(&giant) && options.contains(&hunter) && options.contains(&bears));
    assert!(!options.contains(&ring));

    lk::aim(&mut engine, &[giant, hunter], &[]);
    lk::settle(&mut engine);

    assert_eq!(
        lk::zone_of(&engine, giant),
        Zone::Graveyard,
        "3 damage kills 3/3"
    );
    assert_eq!(lk::zone_of(&engine, hunter), Zone::Battlefield);
    assert_eq!(
        lk::damage_on(&engine, hunter),
        3,
        "4/4 survives with 3 damage"
    );
    assert_eq!(lk::damage_on(&engine, bears), 0);
}

/// Jagged Lightning — requires two targetable creatures on the battlefield.
#[test]
fn jagged_lightning_cannot_be_cast_with_only_one_creature_in_play() {
    let mut engine = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), mountain(), mountain()],
        &[hill_giant()],
        &[jagged_lightning()],
        &[],
    );
    tap_all_mana(&mut engine, P0);

    assert!(!lk::castable(&engine, P0, jagged_lightning()));
}
