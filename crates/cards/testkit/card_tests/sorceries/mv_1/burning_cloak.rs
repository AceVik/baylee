//! `cards/sorceries/mv_1/burning_cloak.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn burning_cloak() -> CardIndex {
    card_index("4527c622-cebe-41eb-9178-4895cccffe99")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Burning Cloak (4527c622-cebe-41eb-9178-4895cccffe99) — {R} Sorcery:
/// "Target creature gets +2/+0 until end of turn. Burning Cloak deals 2 damage
/// to that creature." On a 2/3, the creature becomes 4/3 with 2 damage marked,
/// survives, and reverts at end of turn.
#[test]
fn burning_cloak_pumps_power_marks_two_damage_and_wears_off_at_end_of_turn() {
    let mut engine = lk::game(
        SEED,
        &[mountain(), aurochs()],
        &[grizzly_bears()],
        &[burning_cloak()],
        &[],
    );
    let beast = lk::obj(&engine, P0, aurochs());
    let bears = lk::obj(&engine, P1, grizzly_bears());

    cast_from_hand(&mut engine, P0, burning_cloak());
    let menu = aim_at(&mut engine, P0, beast);
    assert!(menu.contains(&beast) && menu.contains(&bears));
    lk::settle(&mut engine);

    assert_eq!(pt(&engine, beast), (4, 3), "+2/+0 until end of turn");
    assert_eq!(
        lk::damage_on(&engine, beast),
        2,
        "2 damage to that creature"
    );
    assert_eq!(pt(&engine, bears), (2, 2), "untapped bystander untouched");
    assert_eq!(lk::damage_on(&engine, bears), 0);

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, beast), (2, 3), "pump ends in cleanup");
    assert_eq!(lk::damage_on(&engine, beast), 0, "marked damage clears");
}

/// Burning Cloak — "+2/+0" boosts power only, so a 2/2 targeted by Burning
/// Cloak becomes 4/2 with 2 damage marked and dies to state-based actions.
#[test]
fn burning_cloak_kills_a_two_toughness_creature_with_its_two_damage() {
    let mut engine = lk::game(
        SEED,
        &[mountain()],
        &[grizzly_bears()],
        &[burning_cloak()],
        &[],
    );
    let bears = lk::obj(&engine, P1, grizzly_bears());

    cast_from_hand(&mut engine, P0, burning_cloak());
    aim_at(&mut engine, P0, bears);
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, bears), Zone::Graveyard);
    assert!(in_graveyard(&engine, P0, burning_cloak()).is_some());
}
