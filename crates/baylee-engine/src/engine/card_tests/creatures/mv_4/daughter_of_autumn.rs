//! Daughter of Autumn — `{2}{G}{G}` 2/4 Legendary Creature — Avatar (Homelands)
//!
//! `{W}`: The next 1 damage that would be dealt to target white creature this
//! turn is dealt to Daughter of Autumn instead.
//!
//! (`Coverage::Partial`: `RedirectNextDamage` only redirects to a `PlayerRel`,
//! not to this creature; Daughter of Autumn prevents the next 1 damage that
//! would be dealt to target white creature this turn.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn daughter_of_autumn() -> CardIndex {
    card_index("45f3b9f2-3fce-4f91-bcbd-de069e5f9e4c")
}

fn benalish_hero() -> CardIndex {
    card_index("4c81cfb7-8765-4e28-ae33-4287fa9a86cc")
}

#[test]
fn daughter_of_autumn_prevents_next_one_damage_to_target_white_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[lightning_bolt()])
        .battlefield(
            0,
            &[
                daughter_of_autumn(),
                benalish_hero(), // 1/1 white creature
                grizzly_bears(), // 2/2 green creature (illegal target)
                plains(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let daughter = on_battlefield(&engine, p0, daughter_of_autumn()).unwrap();
    let hero = on_battlefield(&engine, p0, benalish_hero()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let bolt_hand = in_hand(&engine, p0, lightning_bolt()).unwrap();
    assert_eq!(pt(&engine, daughter), (2, 4));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: daughter,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(
        options.contains(&hero),
        "white creature is a legal target for Daughter of Autumn"
    );
    assert!(
        !options.contains(&bears),
        "nonwhite creature is not a legal target for Daughter of Autumn"
    );
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

    // Cast Lightning Bolt (3 damage) at 1/1 Benalish Hero: 1 prevented, 2 dealt -> Hero dies with 2 damage marked.
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
}
