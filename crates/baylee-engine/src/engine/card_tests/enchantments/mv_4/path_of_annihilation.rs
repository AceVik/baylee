//! `cards/enchantments/mv_4/path_of_annihilation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn path_of_annihilation() -> CardIndex {
    card_index("eca6ffeb-463c-40b0-90b4-7e09e5579c10")
}

fn eldrazi_devastator() -> CardIndex {
    card_index("bc08ac66-969c-4905-a2d6-16ff565be475")
}

/// Devoid and "When this enchantment enters, create two 0/1 colorless
/// Eldrazi Spawn creature tokens": cast from hand, it is colorless and
/// leaves two 0/1 Spawn.
#[test]
fn path_of_annihilation_is_colorless_and_makes_two_spawn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[path_of_annihilation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    float(&mut engine, p0, ManaColor::Green, 4);
    cast_with_floating(&mut engine, p0, path_of_annihilation());
    pass_until(&mut engine, stack_is_empty);
    let path = on_battlefield(&engine, p0, path_of_annihilation()).expect("resolved");
    assert!(
        engine
            .state()
            .object(path)
            .expect("object")
            .characteristics()
            .colors
            .is_empty(),
        "Devoid"
    );
    let spawn = tokens_of(&engine, p0);
    assert_eq!(spawn.len(), 2, "two Spawn");
    for s in spawn {
        assert_eq!(pt(&engine, s), (0, 1), "0/1");
    }
}

/// "Eldrazi you control have '{T}: Add one mana of any color.'": an Eldrazi
/// of mine taps for mana, a creature that is no Eldrazi is not offered it.
#[test]
fn path_of_annihilation_lets_my_eldrazi_tap_for_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                path_of_annihilation(),
                eldrazi_devastator(),
                grizzly_bears(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let eldrazi = on_battlefield(&engine, p0, eldrazi_devastator()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");

    let legal = priority_offer(&engine);
    let granted = crate::choice::GRANTED_ABILITY;
    assert!(
        legal.abilities.contains(&(eldrazi, granted)),
        "the Eldrazi is offered the granted ability: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.iter().all(|&(id, _)| id != bears),
        "Bears are no Eldrazi"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: eldrazi,
                ability_index: granted,
            },
        )
        .expect("the granted ability activates");
    while matches!(engine.pending(), Pending::ChooseColor { .. }) {
        let (player, action) = answer_one(&engine).expect("a colour is chosen");
        engine.apply(player, action).unwrap();
    }
    assert!(stack_is_empty(&engine), "a mana ability uses no stack");
    assert_eq!(engine.state().players[0].mana_pool.total(), 1, "one mana");
    assert!(is_tapped(&engine, eldrazi), "{{T}} is part of the cost");
}

/// "Whenever you cast a creature spell with mana value 7 or greater, you
/// gain 4 life": a mana value 8 creature does, a mana value 2 one does not.
#[test]
fn path_of_annihilation_gains_four_for_a_big_creature_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[path_of_annihilation()])
        .hand(0, &[eldrazi_devastator(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    float(&mut engine, p0, ManaColor::Green, 2);
    cast_with_floating(&mut engine, p0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 20, "mana value 2: nothing");

    float(&mut engine, p0, ManaColor::Green, 8);
    cast_with_floating(&mut engine, p0, eldrazi_devastator());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 24, "mana value 8: gain 4");
}
