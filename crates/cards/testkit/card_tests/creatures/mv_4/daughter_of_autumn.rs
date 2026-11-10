//! Daughter of Autumn — `{2}{G}{G}` 2/4 Legendary Creature — Avatar (Homelands)
//!
//! `{W}`: The next 1 damage that would be dealt to target white creature this
//! turn is dealt to Daughter of Autumn instead.
//!
//! (`Coverage::Partial`: the redirection is not said. The card used to
//! prevent the next 1 damage to the white creature outright, which spares it
//! without Daughter of Autumn taking the damage; she is now a 2/4 body, and
//! a Lightning Bolt at a white creature is dealt in full.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn daughter_of_autumn() -> CardIndex {
    card_index("45f3b9f2-3fce-4f91-bcbd-de069e5f9e4c")
}

fn benalish_hero() -> CardIndex {
    card_index("4c81cfb7-8765-4e28-ae33-4287fa9a86cc")
}

#[test]
fn daughter_of_autumn_does_not_prevent_or_redirect_damage_to_a_white_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[lightning_bolt()])
        .battlefield(
            0,
            &[
                daughter_of_autumn(),
                benalish_hero(), // 1/1 white creature
                plains(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let daughter = on_battlefield(&engine, p0, daughter_of_autumn()).unwrap();
    let hero = on_battlefield(&engine, p0, benalish_hero()).unwrap();
    let bolt_hand = in_hand(&engine, p0, lightning_bolt()).unwrap();
    assert_eq!(pt(&engine, daughter), (2, 4));
    assert!(
        !ability_offered(&engine, daughter, 0),
        "the {{W}} redirection is not played, so there is nothing to activate"
    );

    // Lightning Bolt (3 damage) at the 1/1 Benalish Hero kills it, and
    // Daughter of Autumn takes none of it.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt_hand })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![hero],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(hero).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().object(daughter).unwrap().damage, 0);
}
