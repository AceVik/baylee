//! `cards/creatures/mv_3/wandering_mage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn mage_with_three_bolts(card: CardIndex) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(
            0,
            &[card, lightning_bolt(), lightning_bolt(), lightning_bolt()],
        )
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                island(),
                island(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
                HILL_GIANT,
            ],
        )
        .battlefield(1, &[plains(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast Wandering Mage ({W}{U}{B}).
    cast_from_hand(&mut engine, p0, card);
    pass_until(&mut engine, stack_is_empty);

    engine
}

fn bolt_creature(engine: &mut Engine<RegistryLookup>, player: PlayerId, target: ObjectId) {
    cast_from_hand(engine, player, lightning_bolt());
    aim_at(engine, player, target);
    pass_until(engine, stack_is_empty);
}

fn bolt_player(engine: &mut Engine<RegistryLookup>, player: PlayerId, target: PlayerId) {
    let bolt = in_hand(engine, player, lightning_bolt()).unwrap();
    engine
        .apply(player, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![target],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

#[test]
fn wandering_mage_prevents_damage_across_all_three_abilities() {
    let p0 = PlayerId::new(0);
    let card = card_index("ac9c81a1-f444-4051-b309-f1af1b5df12a");
    let mut engine = mage_with_three_bolts(card);

    let mage = on_battlefield(&engine, p0, card).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let bear = on_battlefield(&engine, PlayerId::new(1), grizzly_bears()).unwrap();

    // Ability 0 ({W}, Pay 1 life): Prevent the next 2 damage to target creature (Hill Giant).
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, card, 0);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 19);

    bolt_creature(&mut engine, p0, giant);
    assert_eq!(
        engine.state().object(giant).unwrap().damage,
        1,
        "2 of Lightning Bolt's 3 damage should be prevented"
    );

    // Ability 1 ({U}): Prevent the next 1 damage to target Cleric or Wizard creature.
    // Hill Giant is neither Cleric nor Wizard; Wandering Mage is both.
    activate(&mut engine, p0, card, 1);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(options.contains(&mage));
    assert!(!options.contains(&giant));
    aim_at(&mut engine, p0, mage);
    pass_until(&mut engine, stack_is_empty);

    bolt_creature(&mut engine, p0, mage);
    assert_eq!(
        engine.state().object(mage).unwrap().damage,
        2,
        "1 of Lightning Bolt's 3 damage to Wandering Mage should be prevented"
    );

    // Ability 2 ({B}, Put a -1/-1 counter on a creature you control):
    // Prevent the next 2 damage to target player or planeswalker (p0).
    activate(&mut engine, p0, card, 2);
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(
        options.is_empty(),
        "no creature is a legal target: only a player or a planeswalker"
    );
    assert!(player_options.contains(&p0));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    // The -1/-1 counter goes on a creature *you* control.
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the counter's creature, got {:?}",
            engine.pending()
        );
    };
    assert!(options.contains(&giant) && options.contains(&mage));
    assert!(
        !options.contains(&bear),
        "the opponent's creature cannot take the counter"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(giant)
            .unwrap()
            .counters
            .get(CounterKind::M1M1),
        1
    );

    bolt_player(&mut engine, p0, p0);
    assert_eq!(
        engine.state().players[0].life,
        18,
        "2 of Lightning Bolt's 3 damage to p0 should be prevented (19 -> 18)"
    );
}

#[test]
fn wandering_mage_prevention_ends_with_the_turn() {
    let p0 = PlayerId::new(0);
    let card = card_index("ac9c81a1-f444-4051-b309-f1af1b5df12a");
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[card, lightning_bolt()])
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                island(),
                swamp(),
                mountain(),
                HILL_GIANT,
            ],
        )
        .battlefield(1, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, card);
    pass_until(&mut engine, stack_is_empty);
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    // {W}, pay 1 life: prevent the next 2 damage to Hill Giant this turn.
    activate(&mut engine, p0, card, 0);
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 19);
    // The turn passes (p1's, then p0's next): the shield is gone, so a full
    // Lightning Bolt kills the 3/3.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    bolt_creature(&mut engine, p0, giant);
    assert!(
        in_graveyard(&engine, p0, HILL_GIANT).is_some(),
        "last turn's prevention did not carry over: 3 damage kills Hill Giant"
    );
}
