//! `cards/enchantments/auras/mv_1/crown_of_flames.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn crown_of_flames() -> CardIndex {
    card_index("f14fb965-5ebc-427a-a5b8-149f54c64171")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// p0 has Grizzly Bears, four Mountains and a Crown of Flames in hand; p1 has
/// a Hill Giant. The Crown is cast at `target`'s side of the table: `mine`
/// picks the Bears, otherwise the Giant. The Crown has resolved and p0 holds
/// priority with three Mountains' worth of {R} floating.
fn a_crowned_creature(mine: bool) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                grizzly_bears(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .hand(0, &[crown_of_flames()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, P1, hill_giant()).unwrap();
    let land = on_battlefield(&engine, P0, mountain()).unwrap();

    cast_from_hand(&mut engine, P0, crown_of_flames());
    let menu = aim_at(&mut engine, P0, if mine { bears } else { giant });
    assert!(menu.contains(&bears) && menu.contains(&giant));
    assert!(
        !menu.contains(&land),
        "\"Enchant creature\" leaves the lands off the menu: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, crown_of_flames()).is_some());
    (engine, bears, giant)
}

/// Crown of Flames (f14fb965-5ebc-427a-a5b8-149f54c64171) — {R} Enchantment — Aura:
/// "Enchant creature. {R}: Enchanted creature gets +1/+0 until end of turn. {R}: Return this Aura to its owner's hand."
#[test]
fn crown_of_flames_pumps_only_the_enchanted_creature() {
    let (mut engine, bears, giant) = a_crowned_creature(true);

    activate(&mut engine, P0, crown_of_flames(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (3, 2));
    assert_eq!(
        pt(&engine, giant),
        (3, 3),
        "the other creature is untouched"
    );
}

#[test]
fn crown_of_flames_pumps_an_opponents_creature_it_enchants() {
    let (mut engine, bears, giant) = a_crowned_creature(false);

    activate(&mut engine, P0, crown_of_flames(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, giant), (4, 3));
    assert_eq!(pt(&engine, bears), (2, 2));
}

#[test]
fn crown_of_flames_pump_stacks_and_lasts_only_until_end_of_turn() {
    let (mut engine, bears, _) = a_crowned_creature(true);

    activate(&mut engine, P0, crown_of_flames(), 1);
    pass_until(&mut engine, stack_is_empty);
    activate(&mut engine, P0, crown_of_flames(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (4, 2), "each {{R}} is another +1/+0");

    reach_their_main_phase(&mut engine, P1);

    assert!(on_battlefield(&engine, P0, crown_of_flames()).is_some());
    assert_eq!(pt(&engine, bears), (2, 2), "until end of turn");
}

#[test]
fn crown_of_flames_returns_to_its_owners_hand_and_the_pump_stays_for_the_turn() {
    let (mut engine, bears, _) = a_crowned_creature(true);

    activate(&mut engine, P0, crown_of_flames(), 1);
    pass_until(&mut engine, stack_is_empty);
    activate(&mut engine, P0, crown_of_flames(), 2);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P0, crown_of_flames()).is_some());
    assert!(on_battlefield(&engine, P0, crown_of_flames()).is_none());
    assert_eq!(
        pt(&engine, bears),
        (3, 2),
        "the +1/+0 was already given and lasts until end of turn"
    );
}
