//! Willow Priestess — `{2}{G}{G}` 2/2 Creature — Faerie Druid (Homelands)
//!
//! `{T}`: You may put a Faerie permanent card from your hand onto the battlefield.
//! `{2}{G}`: Target green creature gains protection from black until end of turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn willow_priestess() -> CardIndex {
    card_index("630eab2b-c1fd-46bb-8ae0-ff837e46db1c")
}

fn scryb_sprites() -> CardIndex {
    card_index("a1f20695-6f08-4d5c-9fba-b0018bee298e")
}

fn terror() -> CardIndex {
    card_index("b81f041d-98db-4408-9472-c483e4a502bc")
}

fn activate_priestess(
    engine: &mut Engine<RegistryLookup>,
    player: PlayerId,
    priestess: ObjectId,
    ability_index: u32,
) {
    engine
        .apply(
            player,
            PlayerAction::ActivateAbility {
                source: priestess,
                ability_index,
            },
        )
        .unwrap();
}

#[test]
fn willow_priestess_puts_faerie_from_hand_and_grants_green_creature_protection_from_black() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[scryb_sprites(), grizzly_bears()])
        .hand(1, &[terror()])
        .battlefield(0, &[willow_priestess(), forest(), forest(), forest()])
        .battlefield(1, &[swamp(), swamp(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let priestess = on_battlefield(&engine, p0, willow_priestess()).unwrap();
    let sprites_card = in_hand(&engine, p0, scryb_sprites()).unwrap();
    let bears_card = in_hand(&engine, p0, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    // Activate {T}: put Scryb Sprites from hand onto the battlefield.
    activate_priestess(&mut engine, p0, priestess, 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    // "A Faerie permanent card": the Grizzly Bears in the same hand is not offered.
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(options.contains(&sprites_card));
    assert!(!options.contains(&bears_card));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![bears_card],
                },
            )
            .is_err()
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![sprites_card],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let sprites_bf = on_battlefield(&engine, p0, scryb_sprites()).unwrap();
    assert!(is_tapped(&engine, priestess), "the ability's cost is {{T}}");
    assert!(
        in_hand(&engine, p0, grizzly_bears()).is_some(),
        "the Bears stay in hand"
    );

    // Activate {2}{G}: target green creature (Scryb Sprites) gains protection from black until end of turn.
    tap_all_mana(&mut engine, p0);
    activate_priestess(&mut engine, p0, priestess, 1);
    // "Target green creature": the red Gray Ogre is not a target.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![ogre],
                    players: vec![],
                },
            )
            .is_err()
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![sprites_bf],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Verify p1 cannot target Scryb Sprites with black Terror!
    let terror_card = in_hand(&engine, p1, terror()).unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: terror_card })
        .unwrap();
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::ChooseTargets {
                    objects: vec![sprites_bf],
                    players: vec![],
                },
            )
            .is_err(),
        "Scryb Sprites has protection from black and cannot be targeted by Terror"
    );
}

/// "Until end of turn": on the opponent's turn the Sprites can be targeted by a black spell again.
#[test]
fn willow_priestess_protection_from_black_ends_with_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(1, &[terror()])
        .battlefield(
            0,
            &[
                willow_priestess(),
                scryb_sprites(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let priestess = on_battlefield(&engine, p0, willow_priestess()).unwrap();
    let sprites = on_battlefield(&engine, p0, scryb_sprites()).unwrap();

    tap_all_mana(&mut engine, p0);
    activate_priestess(&mut engine, p0, priestess, 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![sprites],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    let terror_card = in_hand(&engine, p1, terror()).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: terror_card })
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![sprites],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(sprites).unwrap().zone,
        Zone::Graveyard,
        "the protection ended at the end of the turn it was given"
    );
}
