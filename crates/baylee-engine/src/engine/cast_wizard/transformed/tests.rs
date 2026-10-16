use super::*;
use crate::engine::synthetic::{self, SyntheticLookup};
use crate::object::Rider;
use baylee_cards_dsl::{CardDef, CounterKind, Effect, FaceDef, Filter, Modifier};
use baylee_core::types::TypeSet;
const A: PlayerId = PlayerId::new(0);
const B: PlayerId = PlayerId::new(1);
const BATTLE: u32 = 98_291;

fn game(back: FaceDef) -> (Engine<SyntheticLookup>, ObjectId) {
    static COUNTER: &[AbilityDef] = &[baylee_cards_dsl::spell!(
        &[Effect::CounterTargetAbility],
        targets = Some(TargetReq::one(TargetSpec::AbilityOnStack(&Filter::Any)))
    )];
    static COPY: &[AbilityDef] = &[baylee_cards_dsl::spell!(
        &[Effect::CopyTargetAbility],
        targets = Some(TargetReq::one(TargetSpec::AbilityOnStack(&Filter::Any)))
    )];
    let front = FaceDef {
        types: TypeSet::BATTLE,
        defense: Some(3),
        castable_from_hand: true,
        subtypes: &[baylee_core::generated::subtypes::battle::SIEGE],
        ..synthetic::land_face("defeat probe")
    };
    let def = Box::leak(Box::new(CardDef {
        faces: Box::leak(Box::new([front, back])),
        ..*synthetic::creature(BATTLE, "defeat probe", 1, 1, &[])
    }));
    let counter = Box::leak(Box::new(CardDef {
        faces: Box::leak(Box::new([FaceDef {
            types: TypeSet::INSTANT,
            mana_cost: ManaCost::parse("{0}"),
            castable_from_hand: true,
            abilities: COUNTER,
            ..synthetic::land_face("counter defeat probe")
        }])),
        ..*synthetic::creature(BATTLE + 1, "counter defeat probe", 1, 1, &[])
    }));
    let copier = Box::leak(Box::new(CardDef {
        faces: Box::leak(Box::new([FaceDef {
            types: TypeSet::INSTANT,
            mana_cost: ManaCost::parse("{0}"),
            castable_from_hand: true,
            abilities: COPY,
            ..synthetic::land_face("copy defeat probe")
        }])),
        ..*synthetic::creature(BATTLE + 2, "copy defeat probe", 1, 1, &[])
    }));
    let single = synthetic::creature(BATTLE + 3, "single faced physical probe", 1, 1, &[]);
    let mdfc = Box::leak(Box::new(CardDef {
        index: baylee_core::ids::CardIndex::new(BATTLE + 4),
        faces: Box::leak(Box::new([
            synthetic::creature_face("modal physical front", 1, 1, &[]),
            FaceDef {
                castable_from_hand: true,
                ..def.faces[1]
            },
        ])),
        ..*def
    }));
    let mut preset = synthetic::preset(731, &[BATTLE]);
    let mut hand = preset.seats[0].starting_battlefield[0];
    hand.card = baylee_core::ids::CardIndex::new(BATTLE + 1);
    let mut copy_hand = hand;
    copy_hand.card = baylee_core::ids::CardIndex::new(BATTLE + 2);
    preset.seats[0].starting_hand = Some(vec![hand, copy_hand]);
    let mut engine = Engine::new(
        &preset,
        SyntheticLookup::new(vec![def, counter, copier, single, mdfc]),
    )
    .unwrap();
    let id = synthetic::permanents(&engine, BATTLE)[0];
    // Isolate defeat from entry; canonical Battle tests exercise actual entry.
    let object = engine.state.object_mut(id).unwrap();
    object.counters.set(CounterKind::Defense, 3);
    object.riders.push(Rider::BattleProtector(B));
    synthetic::keep_mulligans(&mut engine);
    walk(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player: A, .. })
    });
    (engine, id)
}
fn back() -> FaceDef {
    synthetic::creature_face("transformed probe", 4, 4, &[])
}
fn walk(engine: &mut Engine<SyntheticLookup>, done: impl Fn(&Engine<SyntheticLookup>) -> bool) {
    for _ in 0..200 {
        if done(engine) {
            return;
        }
        let pending = engine.pending().clone();
        assert!(
            synthetic::walk_past(engine, &pending),
            "unexpected {pending:?}"
        );
    }
    panic!("condition not reached: {:?}", engine.pending());
}
fn defeat(engine: &mut Engine<SyntheticLookup>, id: ObjectId) {
    crate::replacement::remove_counters(&mut engine.state, id, CounterKind::Defense, 3);
    engine.collect_triggers();
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Stack).len(),
        1,
        "defeat uses the ordinary stack"
    );
}
fn offer(engine: &mut Engine<SyntheticLookup>) {
    walk(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::PlayFromExile,
                ..
            }
        )
    });
}
fn choose(engine: &mut Engine<SyntheticLookup>, cards: Vec<ObjectId>) {
    engine
        .apply(
            A,
            crate::choice::PlayerAction::ChooseObjects { objects: cards },
        )
        .unwrap();
}
#[test]
fn defeat_can_be_declined_or_cast_the_back_and_resumes_its_parent() {
    for accept in [false, true] {
        let (mut engine, id) = game(back());
        defeat(&mut engine, id);
        offer(&mut engine);
        assert_eq!(
            engine.state.object(id).unwrap().zone,
            crate::zone::Zone::Exile
        );
        choose(&mut engine, if accept { vec![id] } else { vec![] });
        walk(&mut engine, |e| {
            e.state.zones.list(ZoneLocation::Stack).is_empty()
        });
        let object = engine.state.object(id).unwrap();
        assert_eq!(
            object.zone,
            if accept {
                crate::zone::Zone::Battlefield
            } else {
                crate::zone::Zone::Exile
            }
        );
        if accept {
            assert_eq!(object.face_index, 1);
            assert_eq!(object.characteristics().power, Some(4));
        }
        assert!(engine.effect_plays.is_empty());
    }
}
#[test]
fn defeat_does_not_follow_a_blinked_source_and_zero_is_not_last_counter_removal() {
    let (mut engine, id) = game(back());
    defeat(&mut engine, id);
    engine
        .state
        .move_object(id, ZoneLocation::Exile(A), ZonePosition::Top, Cause::Effect)
        .unwrap();
    engine
        .state
        .move_object(id, ZoneLocation::Hand(A), ZonePosition::Top, Cause::Effect)
        .unwrap();
    walk(&mut engine, |e| {
        e.state.zones.list(ZoneLocation::Stack).is_empty()
    });
    assert_eq!(
        engine.state.object(id).unwrap().zone,
        crate::zone::Zone::Hand
    );
    let before = engine.state.journal.last_seq();
    crate::replacement::remove_counters(&mut engine.state, id, CounterKind::Defense, 1);
    assert!(crate::trigger::collect(&engine.state, &engine.lookup, before).is_empty());
}
#[test]
fn transformed_sorcery_resolves_and_mandatory_additional_life_is_paid() {
    static EFFECTS: &[Effect] = &[Effect::gain_life(7)];
    static SPELL: &[AbilityDef] = &[baylee_cards_dsl::spell!(EFFECTS)];
    let face = FaceDef {
        types: TypeSet::SORCERY,
        abilities: SPELL,
        mandatory_additional_costs: &[CostPart::PayLife(2)],
        ..synthetic::land_face("transformed sorcery")
    };
    let (mut engine, id) = game(face);
    let life = engine.state.players[0].life;
    defeat(&mut engine, id);
    offer(&mut engine);
    choose(&mut engine, vec![id]);
    walk(&mut engine, |e| {
        e.state.zones.list(ZoneLocation::Stack).is_empty()
    });
    assert_eq!(engine.state.players[0].life, life + 5);
    assert_eq!(
        engine.state.object(id).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
}
#[test]
fn a_cast_prohibition_still_exiles_the_siege_without_casting_it() {
    let (mut engine, id) = game(back());
    engine
        .state
        .effects
        .register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: B,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::Ability,
            timestamp: 1,
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
            filter: crate::effects::EffectFilter::Dsl(&Filter::Any),
            modifier: Modifier::OpponentsCantCast(&Filter::CREATURE),
        });
    defeat(&mut engine, id);
    offer(&mut engine);
    choose(&mut engine, vec![id]);
    walk(&mut engine, |e| {
        e.state.zones.list(ZoneLocation::Stack).is_empty()
    });
    assert_eq!(
        engine.state.object(id).unwrap().zone,
        crate::zone::Zone::Exile
    );
    assert!(
        !engine
            .state
            .journal
            .entries()
            .iter()
            .any(|e| matches!(e.event, GameEvent::SpellCast { object, .. } if object == id))
    );
}

#[test]
fn the_defeat_trigger_can_be_countered_without_exiling_or_casting_the_siege() {
    let (mut engine, id) = game(back());
    defeat(&mut engine, id);
    let trigger = engine.state.zones.list(ZoneLocation::Stack)[0];
    let counter = engine
        .state
        .zones
        .list(ZoneLocation::Hand(A))
        .iter()
        .copied()
        .find(|&card| engine.state.object(card).unwrap().card.unwrap().index.get() == BATTLE + 1)
        .unwrap();
    engine.start_cast_wizard(A, counter).unwrap();
    engine
        .apply(
            A,
            crate::choice::PlayerAction::ChooseObjects {
                objects: vec![trigger],
            },
        )
        .unwrap();
    walk(&mut engine, |e| {
        e.state.zones.list(ZoneLocation::Stack).is_empty()
    });
    assert_eq!(
        engine.state.object(id).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    assert!(
        !engine
            .state
            .journal
            .entries()
            .iter()
            .any(|e| matches!(e.event, GameEvent::SpellCast { object, .. } if object == id))
    );
}

#[test]
fn defeat_event_keeps_controller_and_intrinsic_ability_when_source_changes_before_collection() {
    let (mut engine, id) = game(back());
    let from = engine.state.journal.last_seq();
    crate::replacement::remove_counters(&mut engine.state, id, CounterKind::Defense, 3);
    engine.state.object_mut(id).unwrap().set_controller(B);
    engine
        .state
        .effects
        .register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: B,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::Ability,
            timestamp: 1,
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
            filter: crate::effects::EffectFilter::object(&engine.state, id),
            modifier: Modifier::LoseAllAbilities,
        });
    engine.state.refresh_characteristics();
    let triggers = crate::trigger::collect(&engine.state, &engine.lookup, from);
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].controller, A);
    assert_eq!(
        triggers[0].source_version,
        Some(engine.state.object(id).unwrap().version)
    );
}

#[test]
fn transformed_modal_back_keeps_its_face_while_choosing_mode_and_player_target() {
    use baylee_cards_dsl::{Amount, ModeCount, SpellMode};
    static MODES: &[SpellMode] = &[
        SpellMode {
            effects: &[Effect::gain_life(2)],
            targets: None,
            second_targets: None,
            cost_override: None,
            additional_cost: None,
        },
        SpellMode {
            effects: &[Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::AnyTarget,
            }],
            targets: Some(TargetReq::one(TargetSpec::AnyOpponent)),
            second_targets: None,
            cost_override: None,
            additional_cost: None,
        },
    ];
    static SPELL: &[AbilityDef] = &[AbilityDef::ModalSpell {
        modes: MODES,
        choose: ModeCount::ONE,
    }];
    let (mut engine, id) = game(FaceDef {
        types: TypeSet::SORCERY,
        abilities: SPELL,
        ..synthetic::land_face("modal transformed probe")
    });
    defeat(&mut engine, id);
    offer(&mut engine);
    choose(&mut engine, vec![id]);
    let captured = engine.snapshot_hash();
    engine.cast_wizard.as_mut().unwrap().forced_face = None;
    assert_ne!(
        engine.snapshot_hash(),
        captured,
        "the forced face affects future casting and must be hashed"
    );
    engine.cast_wizard.as_mut().unwrap().forced_face = Some(1);
    let Pending::ChooseCastMode { options, .. } = engine.pending() else {
        panic!("back-face modes")
    };
    assert_eq!(options.len(), 2);
    assert!(
        options
            .iter()
            .all(|option| matches!(option.kind, CastModeKind::Mode(_)))
    );
    engine
        .apply(A, crate::choice::PlayerAction::ChooseMode(1))
        .unwrap();
    let Pending::ChoosePlayer { options, .. } = engine.pending() else {
        panic!("back-face target: {:?}", engine.pending())
    };
    assert_eq!(options, &[B]);
    engine
        .apply(A, crate::choice::PlayerAction::ChoosePlayer(B))
        .unwrap();
    walk(&mut engine, |e| {
        e.state.zones.list(ZoneLocation::Stack).is_empty()
    });
    assert_eq!(engine.state.players[1].life, 17);
    assert_eq!(engine.state.players[0].life, 20);
}

#[test]
fn a_siege_without_abilities_does_not_trigger_when_its_last_defense_is_removed() {
    let (mut engine, id) = game(back());
    engine
        .state
        .effects
        .register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: A,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::Ability,
            timestamp: 1,
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
            filter: crate::effects::EffectFilter::object(&engine.state, id),
            modifier: Modifier::LoseAllAbilities,
        });
    engine.state.refresh_characteristics();
    let from = engine.state.journal.last_seq();
    crate::replacement::remove_counters(&mut engine.state, id, CounterKind::Defense, 3);
    assert!(crate::trigger::collect(&engine.state, &engine.lookup, from).is_empty());
}

#[test]
fn token_single_faced_and_modal_double_faced_copy_sieges_exile_without_cast_offers() {
    for physical in 0..3 {
        let token = physical == 0;
        let (mut engine, id) = game(back());
        // Preserve copied Siege characteristics while varying the underlying
        // physical object. Copy effects never change this physical card identity.
        let object = engine.state.object_mut(id).unwrap();
        if token {
            object.card = None;
        } else {
            object.card.as_mut().unwrap().index =
                baylee_core::ids::CardIndex::new(BATTLE + if physical == 1 { 3 } else { 4 });
        }
        defeat(&mut engine, id);
        walk(&mut engine, |e| {
            e.state.zones.list(ZoneLocation::Stack).is_empty()
        });
        if token {
            assert!(engine.state.object(id).is_none());
        } else {
            assert_eq!(
                engine.state.object(id).unwrap().zone,
                crate::zone::Zone::Exile
            );
        }
        assert!(!matches!(
            engine.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::PlayFromExile,
                ..
            }
        ));
    }
}

#[test]
fn copied_defeat_trigger_retains_source_incarnation_and_only_one_copy_can_exile_it() {
    for move_source in [false, true] {
        let (mut engine, id) = game(back());
        defeat(&mut engine, id);
        let original = engine.state.zones.list(ZoneLocation::Stack)[0];
        let source_version = engine.state.object(id).unwrap().version;
        let copier = engine
            .state
            .zones
            .list(ZoneLocation::Hand(A))
            .iter()
            .copied()
            .find(|&card| {
                engine.state.object(card).unwrap().card.unwrap().index.get() == BATTLE + 2
            })
            .unwrap();
        engine.start_cast_wizard(A, copier).unwrap();
        engine
            .apply(
                A,
                crate::choice::PlayerAction::ChooseObjects {
                    objects: vec![original],
                },
            )
            .unwrap();
        walk(&mut engine, |e| {
            e.state.object(copier).unwrap().zone == crate::zone::Zone::Graveyard
        });
        let stack = engine.state.zones.list(ZoneLocation::Stack);
        assert_eq!(stack.len(), 2);
        let copy = engine.state.object(*stack.last().unwrap()).unwrap();
        assert_eq!(copy.ability.unwrap().source, id);
        assert!(
            copy.riders
                .contains(&Rider::AbilitySourceVersion(source_version))
        );
        if move_source {
            engine
                .state
                .move_object(id, ZoneLocation::Exile(A), ZonePosition::Top, Cause::Effect)
                .unwrap();
            engine
                .state
                .move_object(id, ZoneLocation::Hand(A), ZonePosition::Top, Cause::Effect)
                .unwrap();
        } else {
            offer(&mut engine);
            choose(&mut engine, vec![]);
        }
        walk(&mut engine, |e| {
            e.state.zones.list(ZoneLocation::Stack).is_empty()
        });
        assert_eq!(
            engine.state.object(id).unwrap().zone,
            if move_source {
                crate::zone::Zone::Hand
            } else {
                crate::zone::Zone::Exile
            }
        );
    }
}

#[test]
fn unconsumed_defeat_facts_change_snapshot_hash_but_consumed_history_does_not() {
    let (mut engine, id) = game(back());
    let reference = engine.state.source_identity(id).unwrap();
    let len = engine.state.journal.len();
    let original_scan = engine.trigger_scan_seq;
    let mut unconsumed = Vec::new();
    let mut consumed = Vec::new();
    for (controller, times) in [(A, 1), (B, 1), (A, 2)] {
        engine.state.journal.cancel_from(len);
        engine.trigger_scan_seq = original_scan;
        engine.state.journal.record(GameEvent::BattleDefeated {
            source: reference,
            controller,
            timestamp: 1,
            times,
        });
        unconsumed.push(engine.snapshot_hash());
        engine.trigger_scan_seq = engine.state.journal.last_seq();
        consumed.push(engine.snapshot_hash());
    }
    assert_ne!(unconsumed[0], unconsumed[1]);
    assert_ne!(unconsumed[0], unconsumed[2]);
    assert_eq!(consumed[0], consumed[1]);
    assert_eq!(consumed[0], consumed[2]);
}

#[test]
fn a_copied_defeat_trigger_alone_does_not_hold_the_zero_defense_siege() {
    let (mut engine, id) = game(back());
    defeat(&mut engine, id);
    let original = engine.state.zones.list(ZoneLocation::Stack)[0];
    let cast_on_original = |engine: &mut Engine<SyntheticLookup>, index| {
        let card = engine
            .state
            .zones
            .list(ZoneLocation::Hand(A))
            .iter()
            .copied()
            .find(|&card| engine.state.object(card).unwrap().card.unwrap().index.get() == index)
            .unwrap();
        engine.start_cast_wizard(A, card).unwrap();
        engine
            .apply(
                A,
                crate::choice::PlayerAction::ChooseObjects {
                    objects: vec![original],
                },
            )
            .unwrap();
        walk(engine, |e| {
            e.state.object(card).unwrap().zone == crate::zone::Zone::Graveyard
        });
    };
    cast_on_original(&mut engine, BATTLE + 2);
    let copied = *engine.state.zones.list(ZoneLocation::Stack).last().unwrap();
    let copied_hash = engine.snapshot_hash();
    engine
        .state
        .object_mut(copied)
        .unwrap()
        .riders
        .retain(|rider| *rider != Rider::AbilityCopy);
    assert_ne!(
        engine.snapshot_hash(),
        copied_hash,
        "copy provenance changes the Siege state-based action and must be hashed"
    );
    engine
        .state
        .object_mut(copied)
        .unwrap()
        .riders
        .push(Rider::AbilityCopy);
    assert!(
        engine
            .state
            .object(copied)
            .unwrap()
            .riders
            .contains(&Rider::AbilityCopy)
    );
    assert!(
        !engine
            .state
            .object(original)
            .unwrap()
            .riders
            .contains(&Rider::AbilityCopy)
    );
    assert_eq!(
        engine.state.object(id).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    cast_on_original(&mut engine, BATTLE + 1);
    assert_eq!(engine.state.zones.list(ZoneLocation::Stack), &[copied]);
    assert_eq!(
        engine.state.object(id).unwrap().zone,
        crate::zone::Zone::Graveyard,
        "only the actually triggered ability delayed the zero-defense state-based action"
    );
    walk(&mut engine, |e| {
        e.state.zones.list(ZoneLocation::Stack).is_empty()
    });
    assert_eq!(
        engine.state.object(id).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    assert!(!matches!(
        engine.pending(),
        Pending::ChooseCards {
            prompt: ChoicePrompt::PlayFromExile,
            ..
        }
    ));
}
