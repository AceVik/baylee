//! `cards/enchantments/mv_3/downdraft.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn downdraft() -> CardIndex {
    card_index("c2508190-5aca-4511-adc8-2072cd228119")
}

fn air_elemental() -> CardIndex {
    card_index("7744bae4-a8b7-44a5-9b4c-0048ad4cc448")
}

fn wind_drake() -> CardIndex {
    card_index("d6ffdaf0-ac08-4de9-bbce-2eab2f86bcca")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Downdraft out, P0 with a Wind Drake (2/2 flier) and a Bears, P1 with an
/// Air Elemental (4/4 flier).
fn board() -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let e = lk::game(
        SEED,
        &[downdraft(), wind_drake(), grizzly_bears()],
        &[air_elemental()],
        &[],
        &[],
    );
    let drake = lk::obj(&e, P0, wind_drake());
    let bears = lk::obj(&e, P0, grizzly_bears());
    let elemental = lk::obj(&e, P1, air_elemental());
    (e, drake, bears, elemental)
}

fn flies(e: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    keywords(e, id).contains(KeywordSet::FLYING)
}

/// Downdraft (c2508190-5aca-4511-adc8-2072cd228119) — "{G}: Target creature
/// loses flying until end of turn." Only the target loses it, and only until
/// end of turn.
#[test]
fn downdraft_green_mana_removes_flying_from_the_target_until_end_of_turn() {
    let (mut e, drake, _bears, elemental) = board();
    assert!(flies(&e, elemental) && flies(&e, drake));

    lk::float(&mut e, P0, &[(ManaColor::Green, 1)]);
    activate(&mut e, P0, downdraft(), 0);
    aim_at(&mut e, P0, elemental);
    lk::settle(&mut e);
    assert!(!flies(&e, elemental), "the target lost flying");
    assert!(flies(&e, drake), "the other flier did not");
    assert_eq!(lk::pool_total(&e, P0), 0, "{{G}} paid");

    reach_their_main_phase(&mut e, P1);
    assert!(flies(&e, elemental), "until end of turn only");
}

/// Downdraft — "Sacrifice this enchantment: It deals 2 damage to each
/// creature with flying." Every flier takes 2, whoever controls it (the
/// Drake dies); the Bears are not fliers; the Enchantment is sacrificed.
#[test]
fn downdraft_sacrificed_hurts_every_flier_and_nothing_else() {
    let (mut e, _drake, bears, elemental) = board();
    activate(&mut e, P0, downdraft(), 1);
    lk::settle(&mut e);

    assert!(in_graveyard(&e, P0, downdraft()).is_some(), "sacrificed");
    assert_eq!(lk::damage_on(&e, elemental), 2, "the opponent's flier");
    assert!(
        in_graveyard(&e, P0, wind_drake()).is_some(),
        "and your own, 2 damage on a 2/2"
    );
    assert_eq!(lk::damage_on(&e, bears), 0, "no flying, no damage");
}

/// Downdraft — "each creature with flying" is read as the ability resolves:
/// an Air Elemental that lost flying to the first ability takes nothing.
#[test]
fn downdraft_sacrifice_skips_a_creature_that_lost_flying() {
    let (mut e, _drake, _bears, elemental) = board();
    lk::float(&mut e, P0, &[(ManaColor::Green, 1)]);
    activate(&mut e, P0, downdraft(), 0);
    aim_at(&mut e, P0, elemental);
    lk::settle(&mut e);

    activate(&mut e, P0, downdraft(), 1);
    lk::settle(&mut e);
    assert_eq!(lk::damage_on(&e, elemental), 0, "it no longer has flying");
    assert!(in_graveyard(&e, P0, wind_drake()).is_some());
}
