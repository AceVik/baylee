//! `cards/enchantments/auras/mv_1/kithkin_armor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::aura_kit as kit;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn kithkin_armor() -> CardIndex {
    card_index("35e2730e-b3aa-4080-aae8-7b3ad1e59848")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Kithkin Armor (35e2730e-b3aa-4080-aae8-7b3ad1e59848) — "Enchanted creature
/// can't be blocked by creatures with power 3 or greater." The Hill Giant (3)
/// may not block the enchanted Bears and a declaration that tries is
/// refused; the Bears (power 2) may; and the unenchanted Lions are blockable
/// by the Giant.
#[test]
fn kithkin_armor_stops_blockers_with_power_three_or_more_for_the_enchanted_creature() {
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), savannah_lions()],
        &[hill_giant(), grizzly_bears()],
        &[kithkin_armor()],
        &[],
    );
    let bears = lk::obj(&e, P0, grizzly_bears());
    let lions = lk::obj(&e, P0, savannah_lions());
    let giant = lk::obj(&e, P1, hill_giant());
    let their_bears = lk::obj(&e, P1, grizzly_bears());
    kit::cast_aura(&mut e, P0, kithkin_armor(), bears, &[(ManaColor::White, 1)]);

    let offer = lk::attack(&mut e, &[bears, lions]);

    assert!(
        !lk::may_block(&offer, giant, bears),
        "power 3 may not block the enchanted creature"
    );
    assert!(lk::may_block(&offer, their_bears, bears), "power 2 may");
    assert!(
        lk::may_block(&offer, giant, lions),
        "and the Giant may block an unenchanted one"
    );
    assert!(
        e.apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, bears)],
            },
        )
        .is_err(),
        "the illegal block is refused"
    );
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(their_bears, bears), (giant, lions)],
        },
    )
    .expect("the legal blocks are accepted");
}

/// Kithkin Armor — control for the test above: with no Aura the Giant may
/// block the same Bears.
#[test]
fn without_kithkin_armor_the_giant_may_block_the_bears() {
    let mut e = lk::game(SEED, &[grizzly_bears()], &[hill_giant()], &[], &[]);
    let bears = lk::obj(&e, P0, grizzly_bears());
    let giant = lk::obj(&e, P1, hill_giant());

    let offer = lk::attack(&mut e, &[bears]);

    assert!(lk::may_block(&offer, giant, bears));
}

/// Kithkin Armor — the sacrifice ability ("prevent the next damage from a
/// source of your choice") is not carried: preventing all damage from every
/// source for the turn would be more than printed. When it exists this
/// assertion moves to tests of the prevention.
#[test]
fn kithkin_armor_carries_no_sacrifice_ability() {
    let mut e = lk::game(SEED, &[grizzly_bears()], &[], &[kithkin_armor()], &[]);
    let bears = lk::obj(&e, P0, grizzly_bears());
    let aura = kit::cast_aura(&mut e, P0, kithkin_armor(), bears, &[(ManaColor::White, 1)]);

    assert!(
        !lk::offered(&e, kithkin_armor(), P0, 2),
        "no activated ability on offer"
    );
    assert_eq!(kit::attached_to(&e, aura), Some(bears));
}
