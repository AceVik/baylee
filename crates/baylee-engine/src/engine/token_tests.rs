//! Tokens as first-class permanents.
//!
//! A token has no card, and for a long time the engine read every ability off
//! the card in the registry — so a token could not have any. Everything it
//! said on its face was decoration: a Treasure could not be cracked, an Army
//! could not be found, and a token with a trigger never triggered because the
//! trigger scan skipped card-less objects before it looked at them.
//!
//! These tests pin the three halves of the fix: the abilities come from the
//! token definition, the definition survives on the object that was created
//! from it, and amass finds an Army rather than any creature of the named
//! type.

use super::testkit::{Duel, RegistryLookup, card_index, keep_mulligans, reach_main_phase};
use super::*;
use crate::zone::ZoneLocation;
use baylee_core::ids::{CardIndex, ObjectId};

fn smothering_tithe() -> CardIndex {
    card_index("153376c9-dffd-458c-8ce3-a4c8269bc4e9")
}
fn orcish_bowmasters() -> CardIndex {
    card_index("ea5103f5-27e0-4eb1-902c-7f34652d6bf3")
}
fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

/// Every token on the battlefield, in creation order.
fn tokens_on_battlefield(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_none() && o.token.is_some())
        })
        .collect()
}

/// The object keeps the definition it was made from. Without it the engine
/// has no way back to the token's rules, and the client has no way back to
/// its art — a Treasure would be an anonymous artifact on both counts.
#[test]
fn a_token_remembers_which_token_it_is() {
    let mut engine = Duel::new(9, island())
        .battlefield(0, &[smothering_tithe()])
        .start();
    keep_mulligans(&mut engine);

    // The opponent's draw step triggers the tithe; declining the {2} makes
    // the Treasure.
    let p1 = PlayerId::new(1);
    for _ in 0..60 {
        if !tokens_on_battlefield(&engine).is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::YesNo { player, .. } if player == p1 => {
                // "You may pay {2}" — no.
                engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    let tokens = tokens_on_battlefield(&engine);
    assert_eq!(
        tokens.len(),
        1,
        "the declined tax made exactly one Treasure"
    );
    let treasure = engine.state().object(tokens[0]).expect("token exists");
    let def = treasure.token.expect("the token knows what it is");
    assert_eq!(def.name, "Treasure");
    assert!(
        std::ptr::eq(def, &raw const baylee_cards::tokens::TREASURE),
        "it is the registry's Treasure, not a copy — the copy would have no art key"
    );
    assert_eq!(
        baylee_cards::tokens::token_id(def),
        9,
        "and it answers to a stable id the client can key art on"
    );
}

/// The point of the whole exercise: a Treasure is a Treasure because the
/// engine reads the ability off the token definition. Before that, this
/// returned an empty list and the player was left holding an artifact that
/// did nothing.
#[test]
fn a_treasure_carries_the_ability_printed_on_it() {
    let treasure = &baylee_cards::tokens::TREASURE;
    let mut obj = crate::object::GameObject::new_bare(
        ObjectId::new(1, 0),
        PlayerId::new(0),
        crate::object::ObjectKind::Permanent,
        crate::object::Characteristics {
            name: baylee_core::ids::NameRef::new(0),
            mana_cost: baylee_core::mana::ManaCost::ZERO,
            colors: baylee_core::color::ColorSet::EMPTY,
            types: baylee_core::types::TypeSet::ARTIFACT,
            supertypes: baylee_core::types::SupertypeSet::EMPTY,
            subtypes: baylee_core::types::SubtypeSet::EMPTY,
            keywords: baylee_cards_dsl::KeywordSet::EMPTY,
            power: None,
            toughness: None,
            loyalty: None,
            color_identity: baylee_core::color::ColorSet::EMPTY,
            produced_colors: baylee_core::color::ColorSet::EMPTY,
            produced_colorless: false,
        },
    );

    assert!(
        obj.abilities(&RegistryLookup).is_empty(),
        "a card-less object with no token definition has nothing to offer"
    );

    obj.token = Some(treasure);
    let abilities = obj.abilities(&RegistryLookup);
    assert_eq!(abilities.len(), 1, "the sacrifice outlet is there");
    let baylee_cards_dsl::AbilityDef::Activated {
        cost, mana_ability, ..
    } = &abilities[0]
    else {
        panic!("a Treasure's ability is activated, got {:?}", abilities[0]);
    };
    assert!(mana_ability, "it is a mana ability (CR 605.1a)");
    assert!(
        cost.parts.contains(&baylee_cards_dsl::CostPart::TapSelf)
            && cost
                .parts
                .contains(&baylee_cards_dsl::CostPart::SacrificeSelf),
        "and it costs tapping and sacrificing it"
    );
}

/// CR 701.44a: amass chooses an *Army*. Searching for the named creature
/// type instead meant "amass Orcs 1" grew Orcish Bowmasters — an Orc Archer,
/// and no Army at all — rather than creating the Army it is supposed to.
#[test]
#[allow(clippy::too_many_lines)] // one game, played from the cast to the assertion
fn amass_makes_an_army_instead_of_growing_the_orc_that_cast_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(11, island())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[orcish_bowmasters()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let bowmasters_card = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine
        .apply(
            p0,
            PlayerAction::CastSpell {
                card: bowmasters_card,
            },
        )
        .unwrap();

    // The ETB deals 1 damage to a target opponent and amasses Orcs 1.
    for _ in 0..40 {
        if !tokens_on_battlefield(&engine).is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: options.first().copied().into_iter().collect(),
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected while resolving the ETB: {other:?}"),
        }
    }

    let tokens = tokens_on_battlefield(&engine);
    assert_eq!(tokens.len(), 1, "amass created exactly one Army");
    let army = engine.state().object(tokens[0]).expect("army token");
    let chars = army.characteristics();
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::ARMY),
        "the token is an Army"
    );
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::ORC),
        "and amass Orcs made it an Orc too (CR 701.44b)"
    );
    assert!(
        chars.colors.contains(baylee_core::color::Color::Black),
        "amass tokens are black"
    );
    assert_eq!(
        army.counters.get(baylee_cards_dsl::CounterKind::P1P1),
        1,
        "with the counter amass put on it"
    );
    assert_eq!(army.controller, p0);

    // The Bowmasters is an Orc Archer and no Army: searching for the named
    // type instead of Army is what used to grow it here.
    let bowmasters = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == orcish_bowmasters()))
        })
        .expect("bowmasters landed");
    assert_eq!(
        engine
            .state()
            .object(bowmasters)
            .map(|o| o.counters.get(baylee_cards_dsl::CounterKind::P1P1)),
        Some(0),
        "and the Bowmasters itself grew nothing"
    );
}
