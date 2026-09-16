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

/// The layer refresh visits `Zones::stack_projectable` rather than the
/// stack, because an ability on the stack has no characteristic a layer can
/// change and a deck can leave six figures of them there. That shortcut is
/// only sound while the subset actually excludes abilities and includes
/// everything else — and both halves fail silently, so they need a test on
/// a stack that really holds a triggered ability.
#[test]
fn a_triggered_ability_stays_out_of_the_projection_set() {
    let mut engine = Duel::new(9, island())
        .battlefield(0, &[smothering_tithe()])
        .start();
    keep_mulligans(&mut engine);

    let mut saw_ability_on_stack = false;
    for _ in 0..60 {
        // The invariant, checked at every single step of the walk.
        let state = engine.state();
        let stack = state.zones.list(ZoneLocation::Stack).clone();
        let mut expected: Vec<ObjectId> = stack
            .iter()
            .copied()
            .filter(|id| {
                state
                    .object(*id)
                    .is_some_and(|o| o.kind != crate::object::ObjectKind::AbilityOnStack)
            })
            .collect();
        let mut actual = state.zones.stack_projectable().to_vec();
        expected.sort_unstable();
        actual.sort_unstable();
        assert_eq!(
            actual, expected,
            "projection set drifted from stack {stack:?}"
        );
        saw_ability_on_stack |= stack.len() > expected.len();

        if saw_ability_on_stack && !tokens_on_battlefield(&engine).is_empty() {
            break;
        }
        match engine.pending().clone() {
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
    assert!(
        saw_ability_on_stack,
        "the tithe trigger has to have been on the stack for this to prove anything"
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

/// CR 701.47a: amass chooses an *Army*. Searching for the named creature
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

    // The ETB deals 1 damage to any target and amasses Orcs 1. The arrow is
    // aimed at the opponent's face on purpose: this arm used to answer with
    // `options.first()`, and once the ability started actually asking, the
    // first option was the Bowmasters itself — a 1/1 shooting itself dead,
    // three assertions below.
    for _ in 0..40 {
        if !tokens_on_battlefield(&engine).is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![PlayerId::new(1)],
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
        "and amass Orcs made it an Orc too (CR 701.47a)"
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

fn ondu_cleric() -> CardIndex {
    card_index("f4232466-dd6a-49bf-be6c-95905c3ded17")
}
fn rite_of_replication() -> CardIndex {
    card_index("fb60739e-1dc3-481d-a056-ad72e665c680")
}

/// A copy's ability on the stack is addressed by no card, and says so.
///
/// A token copy has no card (CR 111.1), and the handle that names an
/// ability on the stack — `AbilityLoc::card`, which leaves the engine as
/// the view's `StackItem::Ability` and as the name a player's standing
/// answer is filed under — used to be filled in with `CardIndex::new(0)`
/// when there was nothing to fill it with. That is not a hole; it is a
/// claim, and the card at index 0 is a real one. Every token's, token
/// copy's and emblem's ability answered to the same stranger's name, so
/// one "always say yes to this trigger" would have covered all of them at
/// once.
///
/// Rite of Replication on an Ondu Cleric is the smallest board that puts
/// both halves on the stack at the same moment: the copy entering is an
/// Ally entering, so the original rallies *and* the copy rallies, and the
/// two entries have to disagree about this one field.
#[test]
fn a_copys_ability_is_addressed_by_no_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7, island())
        .battlefield(0, &[ondu_cleric(), island(), island(), island(), island()])
        .hand(0, &[rite_of_replication()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let rite = super::testkit::in_hand(&engine, p0, rite_of_replication()).expect("the rite");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    engine
        .apply(p0, PlayerAction::CastSpell { card: rite })
        .unwrap();
    let cleric = super::testkit::on_battlefield(&engine, p0, ondu_cleric()).expect("the cleric");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![cleric],
                players: Vec::new(),
            },
        )
        .unwrap();
    // Rite's kicker is an optional additional cost; five more mana is not
    // what this board is for.
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    // Both seats pass, the Rite resolves, and the rallies go on the stack.
    super::testkit::pass_until(&mut engine, |e| {
        e.state()
            .zones
            .list(ZoneLocation::Stack)
            .iter()
            .filter(|id| {
                e.state()
                    .object(**id)
                    .is_some_and(|o| o.kind == ObjectKind::AbilityOnStack)
            })
            .count()
            == 2
    });
    let addressed: Vec<Option<CardIndex>> = engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .filter_map(|id| engine.state().object(*id))
        .filter_map(|o| o.ability)
        .map(|loc| loc.card)
        .collect();
    assert_eq!(
        addressed.iter().filter(|c| c.is_none()).count(),
        1,
        "the copy's rally is addressed by no card: {addressed:?}"
    );
    assert_eq!(
        addressed
            .iter()
            .filter(|c| **c == Some(ondu_cleric()))
            .count(),
        1,
        "and the printed cleric's rally still names the cleric: {addressed:?}"
    );
}

fn sokka() -> CardIndex {
    card_index("6b68acc2-b9d5-495b-8054-c04bae1349f1")
}
fn brainstorm() -> CardIndex {
    card_index("36cd2364-d113-47d1-b2c4-b088d9eb88dd")
}

/// Casts one Brainstorm and runs the table back to a quiet main phase.
///
/// Everything it answers is somebody's own decision — Brainstorm's two
/// cards back on top are taken from the *end* of the list so the second
/// copy is never the one put back, and there is no other question on this
/// board.
#[track_caller]
fn cast_a_brainstorm(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let spell = super::testkit::in_hand(engine, seat, brainstorm()).expect("a Brainstorm in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        let _ = engine.apply(seat, PlayerAction::ActivateManaAbility { source });
    }
    engine
        .apply(seat, PlayerAction::CastSpell { card: spell })
        .unwrap();
    for _ in 0..24 {
        if super::testkit::stack_is_empty(engine)
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
        {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                ..
            } => {
                let back: Vec<_> = options.into_iter().rev().take(min as usize).collect();
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: back })
                    .unwrap();
            }
            other => panic!("nothing else should be asked here, got {other:?}"),
        }
    }
    panic!("the Brainstorm never finished resolving");
}

/// A token's keyword trigger fires, and the keyword can be one it was given.
///
/// The card-less half of an ability handle used to be a reason to *stop*:
/// both branches that put a synthetic keyword trigger (prowess, ward) on
/// the stack read the source's card out first and returned when there was
/// none, so a token was queued a trigger it never fired. Nothing about
/// prowess needs a card — it is one line of arithmetic on the creature
/// that has it.
///
/// Sokka is the whole board. He prints prowess, gives it to every other
/// Ally his controller has, and makes an Ally token on each noncreature
/// spell — so the *first* Brainstorm creates the token and the *second*
/// is the spell its borrowed prowess answers.
///
/// The second token is the counter-test and costs nothing: it is made by
/// the same trigger on the same spell, so it arrives after that spell was
/// cast and has nothing to have grown from. One 2/2 and one 1/1 on a
/// board where both are the same token is the pair that says the trigger
/// fired for a reason rather than by accident.
#[test]
fn a_token_grows_on_the_prowess_it_was_lent() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(11, island())
        .battlefield(0, &[sokka(), island(), island(), island(), island()])
        .hand(0, &[brainstorm(), brainstorm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_a_brainstorm(&mut engine, p0);
    let first = *tokens_on_battlefield(&engine)
        .first()
        .expect("Sokka made an Ally token");
    assert_eq!(
        power_and_toughness(&engine, first),
        (1, 1),
        "the token arrives as the 1/1 it is printed as"
    );

    cast_a_brainstorm(&mut engine, p0);
    assert_eq!(
        power_and_toughness(&engine, first),
        (2, 2),
        "and grows on a prowess it holds only because Sokka is standing beside it"
    );
    let second = *tokens_on_battlefield(&engine)
        .iter()
        .find(|id| **id != first)
        .expect("the second Brainstorm made a second Ally token");
    assert_eq!(
        power_and_toughness(&engine, second),
        (1, 1),
        "the token that arrived with the spell had nothing to grow on"
    );
}

/// The projected size of a permanent, which is the only one worth asking
/// about here: prowess is a continuous effect, not a counter.
fn power_and_toughness(engine: &Engine<RegistryLookup>, id: ObjectId) -> (i16, i16) {
    let c = engine
        .state()
        .object(id)
        .expect("the permanent is on the battlefield")
        .characteristics();
    (
        c.power.expect("a creature has power"),
        c.toughness.expect("a creature has toughness"),
    )
}
