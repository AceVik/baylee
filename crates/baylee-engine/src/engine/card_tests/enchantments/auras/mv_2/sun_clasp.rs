//! `cards/enchantments/auras/mv_2/sun_clasp.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn sun_clasp() -> CardIndex {
    card_index("cb8f5ee4-285a-4c37-adcb-514b3e4bdad3")
}

/// `Sun Clasp` ({1}{W}, Enchantment — Aura, `Coverage::Implemented`) grants
/// enchanted creature +1/+3 and has `{W}: Return enchanted creature to its
/// owner's hand.`
#[test]
fn sun_clasp_pumps_enchanted_creature_and_returns_it_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), GRIZZLY_BEARS, HILL_GIANT],
        )
        .hand(0, &[sun_clasp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let other = on_battlefield(&engine, p0, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, sun_clasp());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (3, 5));
    assert_eq!(pt(&engine, other), (3, 3));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, sun_clasp(), 2);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, GRIZZLY_BEARS).is_some());
    assert_eq!(
        engine.state().object(other).unwrap().zone,
        Zone::Battlefield
    );
}

/// "Return enchanted creature to its *owner's* hand": Sun Clasp on an opponent's
/// creature, activated by its controller, puts the creature in the opponent's
/// hand and not in its own.
#[test]
fn sun_clasp_returns_an_opponents_creature_to_its_owners_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[sun_clasp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, sun_clasp());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bear), (3, 5));

    activate(&mut engine, p0, sun_clasp(), 2);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p1, GRIZZLY_BEARS).is_some(),
        "back in its owner's hand"
    );
    assert!(in_hand(&engine, p0, GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_none());
}
