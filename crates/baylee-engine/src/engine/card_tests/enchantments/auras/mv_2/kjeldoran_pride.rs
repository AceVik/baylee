//! Kjeldoran Pride — `{1}{W}` Enchantment — Aura (Alliances)
//!
//! Enchant creature
//! Enchanted creature gets +1/+2.
//! `{2}{U}`: Attach this Aura to target creature other than enchanted creature.

#[allow(clippy::wildcard_imports)]
use super::*;

fn kjeldoran_pride() -> CardIndex {
    card_index("515fb257-ec07-4978-af16-47a10a22ba3c")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

#[test]
fn kjeldoran_pride_grants_plus_1_plus_2_and_moves_to_another_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[kjeldoran_pride()])
        .battlefield(
            0,
            &[
                grizzly_bears(),
                hill_giant(),
                plains(),
                plains(),
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    let pride_hand = in_hand(&engine, p0, kjeldoran_pride()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: pride_hand })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (3, 4));
    assert_eq!(pt(&engine, giant), (3, 3));

    // Activate {2}{U}: Attach this Aura to target creature other than enchanted creature (Hill Giant).
    let pride = on_battlefield(&engine, p0, kjeldoran_pride()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pride,
                ability_index: 2,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets for Kjeldoran Pride ability");
    };
    assert!(
        !options.contains(&bears),
        "currently enchanted creature is excluded"
    );
    assert!(options.contains(&giant), "other creature is offered");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (2, 2));
    assert_eq!(pt(&engine, giant), (4, 5));
}

/// "Target creature other than enchanted creature": a creature of the other player is a legal home
/// (the printed text says any creature), the Aura is not offered itself and no land is, and the
/// move takes the bonus off the old creature and puts it on the new one.
#[test]
fn kjeldoran_pride_moves_to_either_players_creature_and_never_to_a_noncreature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[kjeldoran_pride()])
        .battlefield(
            0,
            &[
                grizzly_bears(),
                plains(),
                plains(),
                island(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let a_land = on_battlefield(&engine, p0, island()).unwrap();
    let pride_hand = in_hand(&engine, p0, kjeldoran_pride()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: pride_hand })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let pride = on_battlefield(&engine, p0, kjeldoran_pride()).unwrap();
    assert_eq!(pt(&engine, bears), (3, 4));

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pride,
                ability_index: 2,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets for Kjeldoran Pride ability");
    };
    assert_eq!(
        options,
        vec![giant],
        "the other player's creature is the only other creature"
    );
    assert!(!options.contains(&pride) && !options.contains(&a_land));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, giant),
        (4, 5),
        "the Aura's controller gave the opponent the bonus"
    );
    assert_eq!(pt(&engine, bears), (2, 2));
}

/// "{2}{U}": three white mana is not the price, so the move is not offered.
#[test]
fn kjeldoran_pride_cannot_move_without_blue_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[kjeldoran_pride()])
        .battlefield(
            0,
            &[
                grizzly_bears(),
                hill_giant(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let pride_hand = in_hand(&engine, p0, kjeldoran_pride()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: pride_hand })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let pride = on_battlefield(&engine, p0, kjeldoran_pride()).unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == pride),
        "three white mana in the pool pays no {{2}}{{U}}: {:?}",
        legal.abilities
    );
}
