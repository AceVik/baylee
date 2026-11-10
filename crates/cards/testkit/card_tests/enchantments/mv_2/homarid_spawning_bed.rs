//! `cards/enchantments/mv_2/homarid_spawning_bed.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn bed_card() -> CardIndex {
    card_index("173f2b8e-8072-4fdd-a469-8a1fb0bb6e64")
}
fn warrior_card() -> CardIndex {
    // Homarid Warrior, {4}{U}, mana value five.
    card_index("6facb0f6-fa49-4ce6-9f95-15bb3ce4bcf2")
}
fn tortoise_card() -> CardIndex {
    // Giant Tortoise, {1}{U}, mana value two.
    card_index("2dd50d7f-941f-4deb-a15c-ee2357844c35")
}
fn bears_card() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn board(creatures: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut mine = vec![bed_card(), island(), island(), island()];
    mine.extend_from_slice(creatures);
    let mut e = Duel::new(SEED, swamp()).battlefield(0, &mine).start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

fn tokens(e: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let o = e.state().object(id).expect("on the battlefield");
            !o.is_card() && o.controller == P0
        })
        .collect()
}

/// Presses the Bed and sacrifices `victim`, answering the sacrifice question
/// and returning the menu it offered.
fn sacrifice(e: &mut Engine<RegistryLookup>, victim: ObjectId) -> Vec<ObjectId> {
    tap_all_mana(e, P0);
    activate(e, P0, bed_card(), 0);
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        panic!("the sacrifice is a choice, got {:?}", e.pending())
    };
    assert_eq!(player, P0);
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![victim],
        },
    )
    .expect("the victim was on the menu");
    pass_until(e, stack_is_empty);
    options
}

/// Homarid Spawning Bed (173f2b8e-8072-4fdd-a469-8a1fb0bb6e64) — "{1}{U}{U},
/// Sacrifice a blue creature: Create X 1/1 blue Camarid creature tokens, where
/// X is the sacrificed creature's mana value." Mana value five makes five;
/// only a blue creature is on the menu.
#[test]
fn homarid_spawning_bed_makes_as_many_camarids_as_the_sacrificed_mana_value() {
    let mut e = board(&[warrior_card(), bears_card()]);
    let warrior = on_battlefield(&e, P0, warrior_card()).expect("the Warrior");
    let bears = on_battlefield(&e, P0, bears_card()).expect("the Bears");
    assert!(tokens(&e).is_empty());

    let menu = sacrifice(&mut e, warrior);
    assert!(menu.contains(&warrior), "a blue creature can be paid");
    assert!(!menu.contains(&bears), "a green one cannot");
    assert_eq!(
        e.state().object(warrior).map(|o| o.zone),
        Some(Zone::Graveyard),
        "sacrificed"
    );
    let made = tokens(&e);
    assert_eq!(made.len(), 5, "X is the Warrior's mana value, five");
    for t in made {
        assert_eq!(pt(&e, t), (1, 1), "each Camarid is a 1/1");
    }
}

/// Homarid Spawning Bed — "X is the sacrificed creature's mana value": a
/// cheaper creature makes fewer, so the count is read from the creature, not
/// fixed.
#[test]
fn homarid_spawning_bed_with_a_two_mana_creature_makes_two() {
    let mut e = board(&[tortoise_card()]);
    let tortoise = on_battlefield(&e, P0, tortoise_card()).expect("the Tortoise");
    sacrifice(&mut e, tortoise);
    assert_eq!(tokens(&e).len(), 2);
}

/// Homarid Spawning Bed — "Sacrifice a blue creature": with only a green
/// creature to give, the ability cannot be activated at all.
#[test]
fn homarid_spawning_bed_cannot_be_paid_with_a_nonblue_creature() {
    let mut e = board(&[bears_card()]);
    tap_all_mana(&mut e, P0);
    let bed = on_battlefield(&e, P0, bed_card()).expect("the Bed");
    assert!(
        !priority_offer(&e).abilities.contains(&(bed, 0)),
        "no blue creature, no activation"
    );
}
