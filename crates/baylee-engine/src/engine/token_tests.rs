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

use super::testkit;
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
            produced_chosen: false,
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

// ---------------------------------------------------------------------------
// The lands the transcoder finished from a `DB$ Token` line
//
// Eleven cards, one rule's output, and what is worth proving is the same for
// each: that the sentence the card prints puts the token the card names on
// the battlefield. A test per card would be eleven copies of one walk, so the
// activated ten are a table and the one with a trigger is its own test —
// which is the shape the two halves actually differ in.
//
// The table holds the printed sentence's own numbers, not the code's: how
// many tokens, and what they are called. That is the whole point of having
// it, because the code is what is under test.
// ---------------------------------------------------------------------------

/// `(oracle id, the card, how many tokens it makes, what they are called)`.
///
/// Sliver Hive is absent and is named here rather than left out quietly: its
/// ability is "activate only if you control a Sliver", and there is no Sliver
/// creature in this pool for [`testkit::arena`] to put on the board. It has
/// a test of its own — `a_changeling_token_is_the_sliver_sliver_hive_asks_for`
/// — built on the one thing in the pool that can satisfy it, the changeling
/// token the land beside it in this same batch makes.
const TOKEN_LANDS: &[(&str, &str, usize, &str)] = &[
    (
        "f8f4fc60-725d-46d8-8e8f-e68e00d20589",
        "Castle Ardenvale",
        1,
        "Human",
    ),
    (
        "e3eb6f90-ccfc-41e7-bff6-0b378226bc7e",
        "Abundant Countryside",
        1,
        "Shapeshifter",
    ),
    (
        "0498ea14-53d9-4655-a616-0ff3cf73de4e",
        "Foundry of the Consuls",
        2,
        "Thopter",
    ),
    (
        "963f2848-15bd-441b-a55c-635f53b7b63f",
        "Gargoyle Castle",
        1,
        "Gargoyle",
    ),
    (
        "2e413d18-2c55-49be-9e49-54d170c978a2",
        "Gnottvold Slumbermound",
        1,
        "Troll Warrior",
    ),
    (
        "79638767-fbc7-451a-b29f-d93f2ac6f102",
        "Kher Keep",
        1,
        "Kobolds of Kher Keep",
    ),
    (
        "eb8ec34c-ae07-4a09-940f-ee965146a787",
        "Memorial to Glory",
        2,
        "Soldier",
    ),
    (
        "5a620d20-f14e-43d0-8e57-c2a197e2ec51",
        "Urza's Factory",
        1,
        "Assembly-Worker",
    ),
    (
        "88fb9e82-28b9-4275-a1e2-cb3a9bfda127",
        "Vitu-Ghazi, the City-Tree",
        1,
        "Saproling",
    ),
    (
        "49e43de3-460b-4562-aef6-da43bd56debc",
        "Spawning Bed",
        3,
        "Eldrazi Scion",
    ),
];

/// Every one of them, pressed on a board that can pay for it.
///
/// Every offer is pressed and not just the first, each on a board of its
/// own: a land that makes a token also taps for mana, and which of the two
/// is ability 0 is a fact about the order the card happens to list them in.
/// A test that pressed the first one measured Castle Ardenvale's mana
/// ability and reported that the card makes no token.
///
/// The assertion that matters twice over is the **id**: a token the ledger
/// never numbered reaches the table wearing no picture at all, and that is
/// the failure a card file cannot show, because the card names a constant
/// whether or not the constant has a row.
#[test]
fn every_land_that_prints_a_token_ability_makes_the_token_it_names() {
    let seat = PlayerId::new(0);
    for (oracle, name, count, token_name) in TOKEN_LANDS {
        let card = card_index(oracle);
        let (engine, objects) = testkit::arena(card).unwrap_or_else(|| panic!("{name}: no board"));
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("{name}: the arena did not end at a priority");
        };
        let offers = testkit::presses(&legal, &objects);
        assert!(!offers.is_empty(), "{name}: was offered nothing to press");

        let mut made = Vec::new();
        for (slot, deed) in offers {
            let (mut engine, objects) =
                testkit::arena(card).unwrap_or_else(|| panic!("{name}: no board"));
            engine
                .apply(seat, deed.action(objects[slot]))
                .unwrap_or_else(|err| panic!("{name}: refused its own offer: {err:?}"));
            match testkit::drive_to_rest(&mut engine, seat) {
                testkit::Rest::Reached => {}
                other => panic!("{name}: {other:?}"),
            }
            let tokens = tokens_on_battlefield(&engine);
            if tokens.is_empty() {
                continue;
            }
            for id in &tokens {
                let def = engine
                    .state()
                    .object(*id)
                    .expect("the token is on the battlefield")
                    .token
                    .unwrap_or_else(|| panic!("{name}: made a token that is not one"));
                assert_eq!(def.name, *token_name, "{name}: made the wrong token");
                assert_ne!(
                    baylee_cards::tokens::token_id(def),
                    u16::MAX,
                    "{name}: made a token the ledger never numbered, so it has no art key"
                );
            }
            made.push(tokens.len());
        }
        assert_eq!(
            made,
            vec![*count],
            "{name}: one ability makes {count} {token_name}(s) and nothing else makes any"
        );
    }
}

/// Khalni Garden is the only one of the eleven whose token comes off a
/// trigger, and it has to be *played* rather than placed: a starting
/// battlefield is a placement and nothing that enters fires from it, so a
/// board built the other way would count no token and pass for the wrong
/// reason.
#[test]
fn a_land_that_makes_a_token_as_it_enters_makes_one() {
    let khalni_garden = card_index("b2d5ba45-8674-4428-89db-c2bbbf0bf5c5");
    let (mut engine, land) =
        testkit::play_land_face(khalni_garden, 0).expect("Khalni Garden is playable on turn one");
    let seat = PlayerId::new(0);
    assert!(
        tokens_on_battlefield(&engine).is_empty(),
        "the trigger is still on the stack"
    );
    match testkit::drive_to_rest(&mut engine, seat) {
        testkit::Rest::Reached => {}
        other => panic!("the trigger never resolved: {other:?}"),
    }

    let tokens = tokens_on_battlefield(&engine);
    assert_eq!(tokens.len(), 1, "one Plant, off one trigger");
    let def = engine
        .state()
        .object(tokens[0])
        .expect("the Plant is on the battlefield")
        .token
        .expect("it knows what it is");
    assert_eq!(def.name, "Plant");
    assert_eq!(
        (def.power, def.toughness),
        (Some(0), Some(1)),
        "a 0/1, which is what the card prints"
    );
    assert!(
        engine
            .state()
            .object(land)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield),
        "and the land itself stayed"
    );
}

/// Thopter Foundry was `Coverage::Partial` for one reason — "the pool has no
/// Thopter" — and the ledger now has one, read out of this very card's own
/// reference script. So the sentence is whole for the first time: the token
/// **and** the life, in the order the card prints them.
#[test]
fn thopter_foundry_makes_the_thopter_its_text_promises() {
    let seat = PlayerId::new(0);
    let foundry = card_index("88bef744-550e-4f33-b1ff-a8ee990ec754");
    let (engine, objects) = testkit::arena(foundry).expect("Thopter Foundry has a board");
    let life_before = engine.state().players[0].life;
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the arena did not end at a priority");
    };

    let mut made = 0usize;
    for (slot, deed) in testkit::presses(&legal, &objects) {
        let (mut engine, objects) = testkit::arena(foundry).expect("Thopter Foundry has a board");
        engine
            .apply(seat, deed.action(objects[slot]))
            .expect("the foundry refused its own offer");
        match testkit::drive_to_rest(&mut engine, seat) {
            testkit::Rest::Reached => {}
            other => panic!("{other:?}"),
        }
        let tokens = tokens_on_battlefield(&engine);
        if tokens.is_empty() {
            continue;
        }
        made += tokens.len();
        let def = engine
            .state()
            .object(tokens[0])
            .expect("the Thopter is on the battlefield")
            .token
            .expect("it knows what it is");
        assert_eq!(def.name, "Thopter");
        assert_eq!((def.power, def.toughness), (Some(1), Some(1)));
        assert!(
            def.colors.contains(baylee_core::color::Color::Blue),
            "a 1/1 *blue* Thopter"
        );
        assert!(
            def.keywords.contains(baylee_cards_dsl::KeywordSet::FLYING),
            "with flying"
        );
        assert_ne!(
            baylee_cards::tokens::token_id(def),
            u16::MAX,
            "and an id the client can key art on"
        );
        assert_eq!(
            engine.state().players[0].life,
            life_before + 1,
            "the other half of the same sentence"
        );
    }
    assert_eq!(made, 1, "one activation, one Thopter");
}

/// Sliver Hive is the twelfth card of the batch and the one
/// [`testkit::arena`] cannot press: "activate only if you control a Sliver",
/// and this pool prints no Sliver creature at all. The only thing that can
/// satisfy it is a changeling — CR 702.73, every creature type — and the
/// pool has exactly one way to get one: Abundant Countryside, the land
/// standing beside it in the same batch.
///
/// So the board is built by hand, and the test says the whole sentence in
/// order: the ability is **not** offered first, the Shapeshifter arrives,
/// and then it is. The first half is what makes the second worth anything —
/// without it this would pass just as well against a card with no condition
/// on it.
#[test]
fn a_changeling_token_is_the_sliver_sliver_hive_asks_for() {
    let seat = PlayerId::new(0);
    let hive = card_index("e7286688-ffbe-4d25-ad55-27990f005368");
    let countryside = card_index("e3eb6f90-ccfc-41e7-bff6-0b378226bc7e");

    let mut field = vec![hive, countryside];
    field.extend(testkit::basics());
    let mut engine = Duel::new(testkit::SEED, testkit::basic_forest())
        .battlefield(0, &field)
        .start();
    assert!(
        testkit::walk_to_own_main(&mut engine, seat),
        "seat 0 reaches its own main phase"
    );

    let hive_object = *testkit::mine(&engine, seat, hive, crate::zone::Zone::Battlefield)
        .first()
        .expect("the Hive is on the battlefield");
    let country_object = *testkit::mine(&engine, seat, countryside, crate::zone::Zone::Battlefield)
        .first()
        .expect("the Countryside is on the battlefield");

    // Eleven generic between the two abilities, off twenty basics. The two
    // lands under test keep their own {T} — an ability whose cost taps its
    // source must not have spent the source paying for the mana.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("seat 0 does not hold priority");
    };
    for source in legal.mana_abilities {
        if source == hive_object || source == country_object {
            continue;
        }
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .expect("a basic taps for its own mana");
    }

    let hive_before = abilities_offered_on(&engine, hive_object);

    let (_, deed) = *testkit::presses(&offered(&engine), std::slice::from_ref(&country_object))
        .last()
        .expect("the Countryside offers its token ability");
    engine
        .apply(seat, deed.action(country_object))
        .expect("the Countryside refused its own offer");
    match testkit::drive_to_rest(&mut engine, seat) {
        testkit::Rest::Reached => {}
        other => panic!("the Shapeshifter never arrived: {other:?}"),
    }
    let shapeshifter = *tokens_on_battlefield(&engine)
        .first()
        .expect("the Countryside made a Shapeshifter");
    assert!(
        engine
            .state()
            .object(shapeshifter)
            .expect("it is on the battlefield")
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::SLIVER),
        "a changeling is every creature type, Sliver included (CR 702.73)"
    );

    // The condition, said as a difference rather than as an index: the Hive
    // prints three abilities and two of them are mana, so "is anything
    // offered" is answered `true` on an empty board. What the Shapeshifter
    // changes is that **one more** is.
    let hive_after = abilities_offered_on(&engine, hive_object);
    let gained: Vec<u32> = hive_after
        .iter()
        .copied()
        .filter(|i| !hive_before.contains(i))
        .collect();
    assert_eq!(
        gained.len(),
        1,
        "the Shapeshifter unlocked exactly one ability on the Hive \
         (before {hive_before:?}, after {hive_after:?})"
    );
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source: hive_object,
                ability_index: gained[0],
            },
        )
        .expect("the Hive refused its own offer");
    match testkit::drive_to_rest(&mut engine, seat) {
        testkit::Rest::Reached => {}
        other => panic!("the Sliver never arrived: {other:?}"),
    }

    let sliver = *tokens_on_battlefield(&engine)
        .iter()
        .find(|id| **id != shapeshifter)
        .expect("the Hive made a second token");
    let def = engine
        .state()
        .object(sliver)
        .expect("it is on the battlefield")
        .token
        .expect("it knows what it is");
    assert_eq!(def.name, "Sliver");
    assert_eq!((def.power, def.toughness), (Some(1), Some(1)));
    assert_ne!(
        baylee_cards::tokens::token_id(def),
        u16::MAX,
        "and an id the client can key art on"
    );
}

/// A token a *reader* wrote carries the ability it prints, and the engine
/// offers it.
///
/// This is the whole of what changed: `TokenDef::abilities` has existed since
/// Treasure, but only a person could fill it, so every generated token was a
/// permanent that did nothing. Spawning Bed's three Eldrazi Scions each print
/// "Sacrifice this token: Add {C}", and the test spends one of them — an
/// ability that reaches the offer list, pays its own cost and leaves mana
/// floating is an ability, where a definition compared against a literal only
/// says a reader typed one.
#[test]
fn a_generated_token_can_be_sacrificed_for_the_mana_it_prints() {
    let seat = PlayerId::new(0);
    let spawning_bed = card_index("49e43de3-460b-4562-aef6-da43bd56debc");
    let (mut engine, objects) = testkit::arena(spawning_bed).expect("a board for Spawning Bed");
    let offers = testkit::presses(&offered(&engine), &objects);

    // The land's own token ability, found by pressing each offer on a board
    // of its own — which is what `every_land_that_prints_a_token_ability…`
    // does and for the same reason: a land that makes tokens also taps.
    let mut scions = Vec::new();
    for (slot, deed) in offers {
        let (mut probe, probe_objects) = testkit::arena(spawning_bed).expect("a board");
        if probe.apply(seat, deed.action(probe_objects[slot])).is_err() {
            continue;
        }
        if !matches!(
            testkit::drive_to_rest(&mut probe, seat),
            testkit::Rest::Reached
        ) {
            continue;
        }
        let made = tokens_on_battlefield(&probe);
        if made.len() == 3 {
            scions = made;
            engine = probe;
            break;
        }
    }
    assert_eq!(scions.len(), 3, "Spawning Bed makes three Eldrazi Scions");

    let scion = scions[0];
    let abilities = abilities_offered_on(&engine, scion);
    assert_eq!(
        abilities.len(),
        1,
        "the Scion offers exactly its own sacrifice outlet, got {abilities:?}"
    );
    let index = *abilities.iter().next().expect("one ability");

    let before = engine.state().players[0].mana_pool.total();
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source: scion,
                ability_index: index,
            },
        )
        .expect("the engine offered it, so it takes it");
    assert!(
        engine.state().object(scion).is_none(),
        "the Scion paid for its own ability with itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        before + 1,
        "and left one mana floating"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Colorless),
        1,
        "colorless, which is what the token prints"
    );
    assert_eq!(
        tokens_on_battlefield(&engine).len(),
        2,
        "and the other two are untouched"
    );
}

/// The other card the `DB$ Token` batch's tokeniser fix finished, played.
///
/// It makes no token at all and is here because it is the second card the
/// same run wrote: three printed sentences, three abilities, and the middle
/// one is the reason it was refused before — `Cost$ T` is trivial, but the
/// *search* below it costs `Sac<1/CARDNAME/this land>`, whose prose the cost
/// tokeniser used to cut in half.
///
/// The search finds nothing, and that is asserted rather than worked
/// around: this pool compiles no Dragon card at all (the only file naming
/// `creature::DRAGON` is this one), so "put it into your hand" has nothing
/// to put. What the ability still has to do is pay its own cost — and a land
/// that sacrificed itself for a card it did not find is the whole of what a
/// player would notice.
#[test]
fn maelstrom_of_the_spirit_dragon_taps_two_ways_and_hunts_a_dragon() {
    let seat = PlayerId::new(0);
    let card = card_index("49e9fba7-8465-4bbb-95db-73a7e149f494");
    let (engine, objects) = testkit::arena(card).expect("a board for the Maelstrom");
    let land = objects[0];
    assert_eq!(
        abilities_offered_on(&engine, land).len(),
        3,
        "three printed sentences, three abilities"
    );

    let mut outcomes = Vec::new();
    for (slot, deed) in testkit::presses(&offered(&engine), &objects) {
        let (mut probe, probe_objects) = testkit::arena(card).expect("a board");
        if probe.apply(seat, deed.action(probe_objects[slot])).is_err() {
            continue;
        }
        if !matches!(
            testkit::drive_to_rest(&mut probe, seat),
            testkit::Rest::Reached
        ) {
            continue;
        }
        let pool = &probe.state().players[0].mana_pool;
        // The **zone** and not `state().object(…)`: a sacrificed *card* goes
        // to a graveyard and is still an object there, where a sacrificed
        // token ceases to exist and stops resolving. Asking the way the
        // Scion above is asked reported a land that had sacrificed itself as
        // still on the battlefield.
        outcomes.push((
            pool.available(baylee_core::mana::ManaColor::Colorless),
            pool.restricted().len(),
            !probe
                .state()
                .zones
                .list(ZoneLocation::Battlefield)
                .contains(&probe_objects[0]),
        ));
    }

    let colorless = outcomes.iter().filter(|o| o.0 == 1 && o.1 == 0).count();
    assert_eq!(colorless, 1, "one sentence adds {{C}}: {outcomes:?}");
    let restricted = outcomes.iter().filter(|o| o.1 == 1).count();
    assert_eq!(
        restricted, 1,
        "one adds a mana that may only be spent on a Dragon or an Omen: {outcomes:?}"
    );
    let sacrificed: Vec<_> = outcomes.iter().filter(|o| o.2).collect();
    assert_eq!(
        sacrificed.len(),
        1,
        "and one sacrifices the land: {outcomes:?}"
    );
}

/// What the seat is being offered right now.
fn offered(engine: &Engine<RegistryLookup>) -> LegalActions {
    match engine.pending() {
        Pending::Priority { legal, .. } => (**legal).clone(),
        other => panic!("expected a priority, got {other:?}"),
    }
}

/// Which of `object`'s abilities the seat is being offered.
///
/// `LegalActions::abilities` lists a mana ability beside every other, so a
/// conditional ability can only be seen as a *change* in this set.
fn abilities_offered_on(
    engine: &Engine<RegistryLookup>,
    object: ObjectId,
) -> std::collections::BTreeSet<u32> {
    offered(engine)
        .abilities
        .iter()
        .filter(|(source, _)| *source == object)
        .map(|(_, index)| *index)
        .collect()
}
