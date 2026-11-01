//! `cards/instants/mv_3/debt_of_loyalty.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::mana::ManaColor;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn debt_of_loyalty() -> CardIndex {
    card_index("7a5cfc97-89cf-43bc-8e48-f8bf7a0c5fcd")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Casts the Debt on `target` with exactly its {1}{W}{W}; returns the menu.
#[track_caller]
fn shield(e: &mut Engine<RegistryLookup>, target: ObjectId) -> Vec<ObjectId> {
    lk::float(e, P0, &[(ManaColor::White, 2), (ManaColor::Colorless, 1)]);
    cast_with_floating(e, P0, debt_of_loyalty());
    let menu = aim_at(e, P0, target);
    lk::settle(e);
    menu
}

/// Bolts `target` with exactly its {R}.
#[track_caller]
fn bolt(e: &mut Engine<RegistryLookup>, target: ObjectId) {
    lk::float(e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(e, P0, lightning_bolt());
    aim_at(e, P0, target);
    lk::settle(e);
}

/// Debt of Loyalty (7a5cfc97-89cf-43bc-8e48-f8bf7a0c5fcd) — "Regenerate
/// target creature." The Bears under the shield survive a lethal Bolt
/// (tapped, damage healed); the Hill Giant beside them, with no shield, dies
/// to the same Bolt.
#[test]
fn debt_of_loyalty_shields_its_target_and_only_its_target() {
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), hill_giant()],
        &[],
        &[debt_of_loyalty(), lightning_bolt(), lightning_bolt()],
        &[],
    );
    let bears = lk::obj(&e, P0, grizzly_bears());
    let giant = lk::obj(&e, P0, hill_giant());

    shield(&mut e, bears);

    bolt(&mut e, giant);
    assert!(
        in_graveyard(&e, P0, hill_giant()).is_some(),
        "no shield on the Giant"
    );

    bolt(&mut e, bears);
    assert!(
        on_battlefield(&e, P0, grizzly_bears()).is_some(),
        "the shield saved the Bears from a lethal Bolt"
    );
    assert!(lk::tapped(&e, bears), "regenerating taps it");
    assert_eq!(lk::damage_on(&e, bears), 0, "and heals all damage");
}

/// Debt of Loyalty — "target creature", not "creature you control": the
/// opponent's Hill Giant is on the menu beside the caster's Bears, and the
/// shield saves it from the Bolt too.
#[test]
fn debt_of_loyalty_may_shield_an_opponents_creature() {
    let mut e = lk::game(
        SEED,
        &[grizzly_bears()],
        &[hill_giant()],
        &[debt_of_loyalty(), lightning_bolt()],
        &[],
    );
    let bears = lk::obj(&e, P0, grizzly_bears());
    let giant = lk::obj(&e, P1, hill_giant());

    let menu = shield(&mut e, giant);
    assert!(
        menu.contains(&giant) && menu.contains(&bears),
        "any creature: {menu:?}"
    );

    bolt(&mut e, giant);
    assert!(
        in_graveyard(&e, P1, hill_giant()).is_none(),
        "the shield saved the Giant"
    );
    assert!(lk::tapped(&e, giant));
}
