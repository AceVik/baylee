//! Preparation copies persist in exile and use ordinary casting and payment.
use super::*;
use crate::object::{AbilityList, Rider};
use baylee_cards_dsl::{Filter, ManaColor, SpendRider, TargetSpec};

const SCHOLAR: u32 = 7490;
const LESSON: u32 = 7491;
const OTHER: u32 = 7492;
const RESTRICTED: u32 = 7493;
const RIDER_MANA: u32 = 7494;
static SCHOLAR_ABILITIES: &[AbilityDef] = &[
    AbilityDef::Prepared {
        card: CardIndex::new(LESSON),
        face: 0,
    },
    free(&[Effect::BecomePrepared], None),
];
static OTHER_ABILITIES: &[AbilityDef] = &[AbilityDef::Prepared {
    card: CardIndex::new(OTHER),
    face: 0,
}];
static LESSON_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
    effects: &[Effect::gain_life(1)],
    targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    second_targets: None,
    condition: None,
}];

static RESTRICTED_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::mana_ability!(&[Effect::mana(
    ManaColor::Green,
    1
)
.restricted(&Filter::ARTIFACT, SpendRider::None)])];
static RIDER_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::mana_ability!(&[Effect::mana(
    ManaColor::Green,
    1
)
.when_spent(&Filter::INSTANT_OR_SORCERY, SpendRider::Uncounterable)])];

fn table() -> (Bench, ObjectId) {
    let mut engine = bench(
        7490,
        vec![
            card(
                SCHOLAR,
                creature_face("Scholar", "{0}", 2, 2),
                KeywordSet::EMPTY,
                SCHOLAR_ABILITIES,
            ),
            card(
                LESSON,
                face("Lesson", "{G}", TypeSet::INSTANT),
                KeywordSet::EMPTY,
                LESSON_ABILITIES,
            ),
            card(
                OTHER,
                face("Other", "{0}", TypeSet::SORCERY),
                KeywordSet::EMPTY,
                &[],
            ),
            super::super::synthetic::land(RESTRICTED, "Restricted", RESTRICTED_ABILITIES),
            super::super::synthetic::land(RIDER_MANA, "Rider", RIDER_ABILITIES),
        ],
        [
            Seat::with(&[SCHOLAR, RESTRICTED, RIDER_MANA]).holding(&[OTHER]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let scholar = the(&engine, ZoneLocation::Battlefield, SCHOLAR);
    activate(&mut engine, me(), scholar, 1);
    settle(&mut engine, me());
    (engine, scholar)
}

fn prepared(engine: &Bench, source: ObjectId) -> bool {
    engine
        .state()
        .object(source)
        .is_some_and(|object| object.riders.contains(&Rider::Prepared))
}

fn floating(engine: &mut Bench) {
    let source = the(engine, ZoneLocation::Battlefield, RIDER_MANA);
    activate(engine, me(), source, 0);
}

#[test]
fn a_prepared_copy_exists_before_casting_and_survives_turn_boundaries() {
    let (mut engine, scholar) = table();
    let copy = engine
        .state()
        .prepared_copy(scholar)
        .expect("prepared copy in exile");
    assert_eq!(
        objects(&engine, ZoneLocation::Exile(me()), LESSON),
        vec![copy]
    );
    assert!(!engine.state().object(copy).unwrap().is_card());
    activate(&mut engine, me(), scholar, 1);
    settle(&mut engine, me());
    assert_eq!(
        objects(&engine, ZoneLocation::Exile(me()), LESSON),
        vec![copy],
        "already prepared cannot create another"
    );
    to_main(&mut engine, them());
    to_main(&mut engine, me());
    assert_eq!(engine.state().prepared_copy(scholar), Some(copy));
    assert_eq!(
        objects(&engine, ZoneLocation::Exile(me()), LESSON),
        vec![copy]
    );
}

#[test]
fn a_prepared_cast_selects_targets_cancels_and_spends_mana_riders_normally() {
    let (mut engine, scholar) = table();
    floating(&mut engine);
    let copy = engine.state().prepared_copy(scholar).unwrap();
    for shortcut in [true, false] {
        let action = if shortcut {
            PlayerAction::ActivateAbility {
                source: scholar,
                ability_index: crate::choice::PREPARED_CAST,
            }
        } else {
            PlayerAction::CastSpell { card: copy }
        };
        engine.apply(me(), action).unwrap();
        let Pending::ChooseTargets { options, .. } = engine.pending() else {
            panic!("a real casting target choice: {:?}", engine.pending())
        };
        assert!(options.contains(&scholar));
        assert!(
            prepared(&engine, scholar),
            "announcement does not unprepare"
        );
        engine.apply(me(), PlayerAction::CancelCast).unwrap();
        assert!(prepared(&engine, scholar));
        assert_eq!(engine.state().prepared_copy(scholar), Some(copy));
        assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    }
    engine
        .apply(me(), PlayerAction::CastSpell { card: copy })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![scholar],
                players: vec![],
            },
        )
        .unwrap();
    assert!(
        !prepared(&engine, scholar),
        "only the successful cast unprepares"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(
        engine
            .state()
            .object(copy)
            .unwrap()
            .riders
            .contains(&Rider::Uncounterable)
    );
    let before = life(&engine, me());
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), before + 1);
    assert!(
        engine.state().object(copy).is_none(),
        "resolved copy ceases to exist"
    );
}

#[test]
fn prepared_spells_cannot_spend_mana_restricted_to_artifact_spells() {
    let (mut engine, scholar) = table();
    let source = the(&engine, ZoneLocation::Battlefield, RESTRICTED);
    activate(&mut engine, me(), source, 0);
    let copy = engine.state().prepared_copy(scholar).unwrap();
    assert!(casting::can_cast(engine.state(), &engine.lookup, me(), copy).is_err());
    assert!(
        !offer(&engine)
            .1
            .abilities
            .contains(&(scholar, crate::choice::PREPARED_CAST))
    );
    assert!(
        engine
            .apply(
                me(),
                PlayerAction::ActivateAbility {
                    source: scholar,
                    ability_index: crate::choice::PREPARED_CAST
                }
            )
            .is_err()
    );
    assert!(prepared(&engine, scholar));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
}

#[test]
fn prepared_permission_follows_control_but_the_copy_keeps_its_owner() {
    let (mut engine, scholar) = table();
    let copy = engine.state().prepared_copy(scholar).unwrap();
    engine
        .state
        .object_mut(scholar)
        .unwrap()
        .set_controller(them());
    engine.state.sync_prepared_copies();
    assert!(casting::play_permission(engine.state(), me(), copy).is_none());
    assert!(casting::play_permission(engine.state(), them(), copy).is_some());
    assert_eq!(engine.state().object(copy).unwrap().owner, me());
    assert_eq!(engine.state().object(copy).unwrap().zone_owner, Some(me()));
}

#[test]
fn prepared_copy_is_frozen_until_phasing_creates_a_new_one() {
    let (mut engine, scholar) = table();
    let copy = engine.state().prepared_copy(scholar).unwrap();
    engine
        .state
        .object_mut(scholar)
        .unwrap()
        .take_abilities(AbilityList::from_static(OTHER_ABILITIES, None, None));
    assert_eq!(
        engine
            .state()
            .object(copy)
            .unwrap()
            .card
            .unwrap()
            .index
            .get(),
        LESSON
    );
    engine.state.phase_out(&[scholar]);
    assert!(casting::play_permission(engine.state(), me(), copy).is_none());
    assert!(
        prepared(&engine, scholar),
        "phasing preserves the designation"
    );
    // No SBA between the two events: the old permission must still expire.
    engine.state.phase_in(scholar);
    let new = engine.state().prepared_copy(scholar).unwrap();
    assert_ne!(new, copy);
    assert_eq!(
        engine
            .state()
            .object(new)
            .unwrap()
            .card
            .unwrap()
            .index
            .get(),
        OTHER
    );
    crate::sba::run(&mut engine.state, &engine.lookup);
    assert!(engine.state().object(copy).is_none());
    assert!(engine.state().object(new).is_some());
    engine.state.phase_in(scholar);
    assert_eq!(
        engine.state().prepared_copy(scholar),
        Some(new),
        "already phased in creates nothing"
    );
}

#[test]
fn leaving_and_returning_does_not_restore_the_prepared_designation() {
    let (mut engine, scholar) = table();
    let copy = engine.state().prepared_copy(scholar).unwrap();
    engine
        .state
        .move_object(
            scholar,
            ZoneLocation::Hand(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine
        .state
        .move_object(
            scholar,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine.state.sync_prepared_copies();
    crate::sba::run(&mut engine.state, &engine.lookup);
    assert!(!prepared(&engine, scholar));
    assert!(engine.state().prepared_copy(scholar).is_none());
    assert!(engine.state().object(copy).is_none());
}

#[test]
fn moving_the_copy_elsewhere_does_not_replenish_it() {
    let (mut engine, scholar) = table();
    let copy = engine.state().prepared_copy(scholar).unwrap();
    engine
        .state
        .move_object(
            copy,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine.state.sync_prepared_copies();
    crate::sba::run(&mut engine.state, &engine.lookup);
    engine.state.prepare(scholar);
    assert!(prepared(&engine, scholar));
    assert!(engine.state().prepared_copy(scholar).is_none());
}

#[test]
fn a_token_copy_has_prepare_characteristics_independent_of_its_body() {
    let (mut engine, _) = table();
    let name = engine.state.names.intern("Frog copy");
    let token =
        engine
            .state
            .create_bare(me(), ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    let object = engine.state.object_mut(token).unwrap();
    object.take_abilities(AbilityList::from_static(SCHOLAR_ABILITIES, None, None));
    let base = object.base_mut();
    base.types = TypeSet::CREATURE;
    base.power = Some(1);
    base.toughness = Some(1);
    engine.state.prepare(token);
    let copy = engine.state().prepared_copy(token).unwrap();
    let object = engine.state().object(copy).unwrap();
    assert_eq!(object.card.unwrap().index.get(), LESSON);
    assert_eq!(object.base.types, TypeSet::INSTANT);
    assert_eq!(object.base.power, None);
    assert_eq!(engine.state().names.get(object.base.name), "Lesson");
}
