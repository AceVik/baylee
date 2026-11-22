//! Everquill Phoenix and the Feather it creates, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn everquill_phoenix_can_spend_a_tapped_feather_to_return_a_phoenix() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let everquill = card_index("56ba78c1-d1eb-4cc1-b70d-98c35d2972ba");
    let phoenix = card_index("f34be4b8-126d-4b5c-8ebb-23b5e5370012");
    let barrier = card_index("90cd8274-3f21-4b78-8910-dcaa5f8fe25d");
    let bears = card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                plains(),
                plains(),
                plains(),
                plains(),
                bears,
                barrier,
            ],
        )
        .hand(0, &[everquill, phoenix, bears])
        .hand(1, &[phoenix])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let returned = hand_to_graveyard(&mut engine, p0, phoenix);
    let wrong_type = hand_to_graveyard(&mut engine, p0, bears);
    let other_graveyard = hand_to_graveyard(&mut engine, p1, phoenix);
    let host = on_battlefield(&engine, p0, bears).unwrap();
    let card = in_hand(&engine, p0, everquill).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    choose_object_target(&mut engine, p0, host);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let [feather] = tokens_of(&engine, p0)[..] else {
        panic!("mutating Everquill Phoenix creates one Feather");
    };

    activate(&mut engine, p0, barrier, 0);
    choose_object_target(&mut engine, p0, feather);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .object(feather)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(
        ability_offered(&engine, feather, 0),
        "a tapped Feather can be sacrificed"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: feather,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("Feather's activation targets a card in your graveyard");
    };
    assert!(options.contains(&returned));
    assert!(!options.contains(&wrong_type));
    assert!(!options.contains(&other_graveyard));
    choose_object_target(&mut engine, p0, returned);
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "sacrifice is part of the cost"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(on_battlefield(&engine, p0, phoenix), Some(returned));
    assert!(
        engine
            .state()
            .object(returned)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}

fn choose_object_target(engine: &mut Engine<RegistryLookup>, player: PlayerId, target: ObjectId) {
    engine
        .apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("the object is a legal target");
}
