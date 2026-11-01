//! `cards/enchantments/auras/mv_2/fire_whip.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::choice::GRANTED_ABILITY;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fire_whip() -> CardIndex {
    card_index("9d4372f0-eedf-4897-9f03-f96fb14ac4a6")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// P0 has a Bears and a Lions, P1 a Lions, and Fire Whip in hand. The Whip
/// is cast at the menu it offers, which is returned with the Aura's id.
fn whipped_bears() -> (Engine<RegistryLookup>, ObjectId, Vec<ObjectId>) {
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), savannah_lions()],
        &[savannah_lions()],
        &[fire_whip()],
        &[],
    );
    let bears = lk::obj(&e, P0, grizzly_bears());
    lk::float(
        &mut e,
        P0,
        &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
    );
    cast_with_floating(&mut e, P0, fire_whip());
    let menu = aim_at(&mut e, P0, bears);
    pass_until(&mut e, stack_is_empty);
    (e, bears, menu)
}

/// Fire Whip (9d4372f0-eedf-4897-9f03-f96fb14ac4a6) — "Enchant creature you
/// control. Enchanted creature has '{T}: This creature deals 1 damage to any
/// target.'" Only your creatures are on the menu; only the enchanted one
/// pings; the ping taps it, hits a player or a creature, and comes back on
/// the next untap.
#[test]
fn fire_whip_gives_the_enchanted_creature_a_tap_ping_for_any_target() {
    let (mut e, bears, menu) = whipped_bears();
    let their_lions = lk::obj(&e, P1, savannah_lions());
    let my_lions = lk::obj(&e, P0, savannah_lions());
    assert!(menu.contains(&bears) && menu.contains(&my_lions));
    assert!(
        !menu.contains(&their_lions),
        "\"creature you control\": the opponent's Lions are not enchantable: {menu:?}"
    );

    assert!(lk::offered(&e, grizzly_bears(), P0, GRANTED_ABILITY));
    assert!(
        !lk::offered(&e, savannah_lions(), P0, GRANTED_ABILITY),
        "the Lions are not enchanted"
    );
    activate(&mut e, P0, grizzly_bears(), GRANTED_ABILITY);
    lk::aim(&mut e, &[], &[P1]);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, P1), 19);
    assert!(lk::tapped(&e, bears), "{{T}} is the cost");
    assert!(
        !lk::offered(&e, grizzly_bears(), P0, GRANTED_ABILITY),
        "tapped, it cannot ping again"
    );

    // Next turn it untaps and pings a creature instead.
    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    activate(&mut e, P0, grizzly_bears(), GRANTED_ABILITY);
    lk::aim(&mut e, &[their_lions], &[]);
    lk::settle(&mut e);
    assert!(
        in_graveyard(&e, P1, savannah_lions()).is_some(),
        "one damage kills the 2/1"
    );
    assert_eq!(lk::life(&e, P1), 19, "and the player was not hit again");
}

/// Fire Whip — "Sacrifice this Aura: It deals 1 damage to any target." The
/// Aura is the one sacrificed; the damage reaches the target; and with the
/// Aura gone the creature has no ping on the next turn.
#[test]
fn fire_whip_sacrificed_deals_one_damage_and_takes_the_ping_with_it() {
    let (mut e, bears, _menu) = whipped_bears();
    let their_lions = lk::obj(&e, P1, savannah_lions());

    activate(&mut e, P0, fire_whip(), 2);
    lk::aim(&mut e, &[their_lions], &[]);
    lk::settle(&mut e);
    assert!(in_graveyard(&e, P0, fire_whip()).is_some(), "sacrificed");
    assert!(
        in_graveyard(&e, P1, savannah_lions()).is_some(),
        "its one damage killed the Lions"
    );
    assert_eq!(lk::life(&e, P1), 20);

    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(!lk::tapped(&e, bears));
    assert!(
        !lk::offered(&e, grizzly_bears(), P0, GRANTED_ABILITY),
        "no Aura, no ping"
    );
}
