use super::*;

#[test]
fn permission_target_projection_preserves_historical_entitlement_without_source_eligibility() {
    use baylee_core::ids::{DamageSourceRef, GrantedActionId};
    use baylee_engine::choice::{GrantedActionKind, GrantedActionOffer};
    use baylee_engine::object::Status;
    use baylee_engine::zone::ZonePosition;
    for hidden in [false, true] {
        let mut state = GameState::from_preset(&mixed_print_preset(), &Registry).unwrap();
        let viewer = PlayerId::new(1);
        let target = state.zones.list(ZoneLocation::Battlefield)[0];
        if hidden {
            state
                .object_mut(target)
                .unwrap()
                .status
                .insert(Status::FACE_DOWN);
        }
        state.refresh_characteristics();
        let reference = DamageSourceRef {
            object: target,
            version: state.object(target).unwrap().version,
        };
        state
            .granted_actions
            .push(baylee_engine::granted::GrantedAction {
                player: viewer,
                offer: GrantedActionOffer {
                    id: GrantedActionId::new(0),
                    source: reference,
                    ability: None,
                    timing: baylee_cards_dsl::SpecialActionTiming::Priority,
                    cost: baylee_cards_dsl::SpecialActionCost::Mana(
                        baylee_core::mana::ManaCost::ZERO,
                    ),
                    effect: GrantedActionKind::PreventNextDamage {
                        target: TargetRef::Object(reference),
                        amount: 1,
                    },
                },
            });
        state
            .move_object(
                target,
                ZoneLocation::Hand(PlayerId::new(0)),
                ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .unwrap();
        let view = player_view(&state, viewer, 1, None, &SeatContext::default(), &[]);
        let old = view.target_object(reference).unwrap();
        assert!(!old.is_current);
        assert_eq!(old.card.is_some(), !hidden);
        assert_eq!(old.rules.is_some(), !hidden);
        assert!(old.referenced_by.is_empty());
        assert!(view.damage_sources.is_empty());
        let bystander = player_view(
            &state,
            PlayerId::new(0),
            1,
            None,
            &SeatContext::default(),
            &[],
        );
        assert!(bystander.target_object(reference).is_none());
    }
}

#[test]
fn stack_target_projection_keeps_both_groups_and_historical_identity() {
    use baylee_core::ids::DamageSourceRef;
    use baylee_engine::object::{ObjectKind, PrintedFace, Status};
    use baylee_engine::zone::ZonePosition;
    for hidden in [false, true] {
        let mut state = GameState::from_preset(&mixed_print_preset(), &Registry).unwrap();
        let viewer = PlayerId::new(1);
        let first = state.zones.list(ZoneLocation::Battlefield)[0];
        let second = state.zones.list(ZoneLocation::Battlefield)[1];
        let identity = state.object(first).unwrap().card.unwrap();
        let copied = teferi_time_raveler();
        state
            .object_mut(first)
            .unwrap()
            .take_abilities(baylee_engine::object::AbilityList {
                abilities: baylee_engine::copiable_abilities::AbilityDefs::Static(&[]),
                printed: PrintedFace::new(copied, 0),
                token: None,
            });
        if hidden {
            state
                .object_mut(first)
                .unwrap()
                .status
                .insert(Status::FACE_DOWN);
        }
        state.refresh_characteristics();
        let first_ref = DamageSourceRef {
            object: first,
            version: state.object(first).unwrap().version,
        };
        let second_ref = DamageSourceRef {
            object: second,
            version: state.object(second).unwrap().version,
        };
        let name = state.names.intern("two target groups");
        let spell = state.create_bare(
            PlayerId::new(0),
            ObjectKind::Spell,
            name,
            ZoneLocation::Stack,
        );
        let obj = state.object_mut(spell).unwrap();
        obj.targets.push(first);
        obj.set_second([second].into_iter().collect(), None);
        state
            .move_object(
                first,
                ZoneLocation::Hand(PlayerId::new(0)),
                ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .unwrap();
        state
            .move_object(
                first,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .unwrap();
        // The returned object's public identity must never reveal an old
        // face-down target; an old face-up target keeps its known print.
        state
            .object_mut(first)
            .unwrap()
            .status
            .remove(Status::FACE_DOWN);
        let mut view = player_view(&state, viewer, 1, None, &SeatContext::default(), &[]);
        assert_eq!(
            view.stack[0].targets,
            vec![TargetRef::Object(first_ref), TargetRef::Object(second_ref)]
        );
        let old = view.target_object(first_ref).unwrap();
        assert!(!old.is_current);
        assert_eq!(old.card.is_some(), !hidden);
        assert_eq!(old.rules.is_some(), !hidden);
        assert!(view.target_object(second_ref).unwrap().is_current);
        assert!(
            view.target_object(DamageSourceRef {
                object: first,
                version: state.object(first).unwrap().version
            })
            .is_none()
        );
        view.battlefield.clear();
        view.stack.clear();
        view.hand.clear();
        view.damage_sources.clear();
        view.target_objects.retain(|row| row.source == first_ref);
        if hidden {
            assert_eq!(view.prints().count(), 0);
            assert_eq!(view.cards().count(), 0);
        } else {
            assert_eq!(view.prints().collect::<Vec<_>>(), vec![identity.print]);
            assert!(view.cards().any(|card| card == identity.index));
            assert!(view.cards().any(|card| card == copied));
        }
        let decoded: baylee_view::PlayerView =
            serde_json::from_str(&serde_json::to_string(&view).unwrap()).unwrap();
        assert_eq!(decoded.target_objects, view.target_objects);
    }
}

#[test]
fn historical_source_projection_retains_a_ceased_tokens_registry_identity() {
    use baylee_core::ids::{DamageSourceRef, SourceChoiceId};
    use baylee_engine::object::{AbilityLoc, ObjectKind, Rider};
    use baylee_engine::zone::ZonePosition;
    let mut state = GameState::from_preset(&mixed_print_preset(), &Registry).unwrap();
    let player = PlayerId::new(0);
    let name = state.names.intern("Soldier");
    let token = state.create_bare(
        player,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    state.object_mut(token).unwrap().token = Some(&baylee_cards::tokens::SOLDIER_1_1_WHITE);
    let source = DamageSourceRef {
        object: token,
        version: state.object(token).unwrap().version,
    };
    let ability = state.create_bare(
        player,
        ObjectKind::AbilityOnStack,
        name,
        ZoneLocation::Stack,
    );
    state.object_mut(ability).unwrap().ability = Some(AbilityLoc {
        card: None,
        source: token,
        index: 0,
    });
    state
        .object_mut(ability)
        .unwrap()
        .riders
        .push(Rider::AbilitySourceVersion(source.version));
    state
        .move_object(
            token,
            ZoneLocation::Graveyard(player),
            ZonePosition::Top,
            baylee_engine::event::Cause::Effect,
        )
        .unwrap();
    state.zones.remove(token, ZoneLocation::Graveyard(player));
    state.arena.remove(token);
    let options = baylee_engine::prevention::source_options(
        &mut state,
        &baylee_cards_dsl::Filter::Any,
        player,
        ability,
    );
    let pending = Pending::ChooseDamageSource {
        player,
        choice: SourceChoiceId::new(10),
        options,
    };
    let row = damage_sources(&state, player, Some(&pending), &SeatContext::default())
        .into_iter()
        .find(|r| r.source == source)
        .unwrap();
    assert!(!row.is_current);
    assert!(row.card.is_none());
    assert_eq!(row.name, "Soldier");
    assert_eq!(
        row.token,
        Some(baylee_cards::tokens::token_id(
            &baylee_cards::tokens::SOLDIER_1_1_WHITE
        ))
    );
}

#[test]
fn historical_source_projection_uses_its_own_visibility_not_the_returned_cards_identity() {
    use baylee_core::ids::{DamageSourceRef, SourceChoiceId};
    use baylee_engine::object::{AbilityLoc, ObjectKind, Rider, Status};
    use baylee_engine::zone::ZonePosition;
    for originally_face_down in [false, true] {
        let mut state = GameState::from_preset(&mixed_print_preset(), &Registry).unwrap();
        let controller = PlayerId::new(0);
        let viewer = PlayerId::new(1);
        let id = state.zones.list(ZoneLocation::Battlefield)[0];
        if originally_face_down {
            state
                .object_mut(id)
                .unwrap()
                .status
                .insert(Status::FACE_DOWN);
        }
        state.refresh_characteristics();
        let source = DamageSourceRef {
            object: id,
            version: state.object(id).unwrap().version,
        };
        let name = state.names.intern("historical source ability");
        let ability = state.create_bare(
            controller,
            ObjectKind::AbilityOnStack,
            name,
            ZoneLocation::Stack,
        );
        let obj = state.object_mut(ability).unwrap();
        obj.ability = Some(AbilityLoc {
            card: Some(island()),
            source: id,
            index: 0,
        });
        obj.riders.push(Rider::AbilitySourceVersion(source.version));
        state
            .move_object(
                id,
                ZoneLocation::Hand(controller),
                ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .unwrap();
        // The next incarnation has the opposite face-down status. Its
        // visibility must never decide what this historical row reveals.
        if !originally_face_down {
            state
                .object_mut(id)
                .unwrap()
                .status
                .insert(Status::FACE_DOWN);
        }
        let options = baylee_engine::prevention::source_options(
            &mut state,
            &baylee_cards_dsl::Filter::Any,
            viewer,
            ability,
        );
        assert!(options.contains(&source));
        let pending = Pending::ChooseDamageSource {
            player: viewer,
            choice: SourceChoiceId::new(4),
            options,
        };
        let rows = damage_sources(&state, viewer, Some(&pending), &SeatContext::default());
        let row = rows.iter().find(|row| row.source == source).unwrap();
        assert!(!row.is_current);
        assert_eq!(row.zone, baylee_view::LogZone::Battlefield);
        assert_eq!(row.card.is_none(), originally_face_down);
        assert_eq!(row.rules.is_none(), originally_face_down);
        assert_eq!(row.name == "Face-down", originally_face_down);
        assert_eq!(row.referenced_by, vec![ability]);
        assert!(
            damage_sources(&state, controller, Some(&pending), &SeatContext::default()).is_empty()
        );
    }
}

#[test]
fn historical_source_alone_entitles_a_new_seat_to_its_print_and_copied_rules() {
    use baylee_core::ids::{DamageSourceRef, SourceChoiceId};
    use baylee_engine::object::{AbilityLoc, ObjectKind, PrintedFace, Rider};
    use baylee_engine::zone::ZonePosition;
    let mut state = GameState::from_preset(&mixed_print_preset(), &Registry).unwrap();
    let player = PlayerId::new(1);
    let source_id = state.zones.list(ZoneLocation::Battlefield)[0];
    let source = DamageSourceRef {
        object: source_id,
        version: state.object(source_id).unwrap().version,
    };
    let identity = state.object(source_id).unwrap().card.unwrap();
    let copied = teferi_time_raveler();
    state
        .object_mut(source_id)
        .unwrap()
        .take_abilities(baylee_engine::object::AbilityList {
            abilities: baylee_engine::copiable_abilities::AbilityDefs::Static(&[]),
            printed: PrintedFace::new(copied, 0),
            token: None,
        });
    let name = state.names.intern("source ability");
    let ability = state.create_bare(
        PlayerId::new(0),
        ObjectKind::AbilityOnStack,
        name,
        ZoneLocation::Stack,
    );
    state.object_mut(ability).unwrap().ability = Some(AbilityLoc {
        card: None,
        source: source_id,
        index: 0,
    });
    state
        .object_mut(ability)
        .unwrap()
        .riders
        .push(Rider::AbilitySourceVersion(source.version));
    state
        .move_object(
            source_id,
            ZoneLocation::Hand(PlayerId::new(0)),
            ZonePosition::Top,
            baylee_engine::event::Cause::Effect,
        )
        .unwrap();
    let options = baylee_engine::prevention::source_options(
        &mut state,
        &baylee_cards_dsl::Filter::Any,
        player,
        ability,
    );
    let pending = Pending::ChooseDamageSource {
        player,
        choice: SourceChoiceId::new(9),
        options,
    };
    let mut view = player_view(
        &state,
        player,
        1,
        Some(&pending),
        &SeatContext::default(),
        &[],
    );
    view.battlefield.clear();
    view.stack.clear();
    view.hand.clear();
    view.damage_sources.retain(|row| row.source == source);
    assert_eq!(view.prints().collect::<Vec<_>>(), vec![identity.print]);
    assert!(view.cards().any(|card| card == identity.index));
    assert!(view.cards().any(|card| card == copied));
    let wire = serde_json::to_string(&view).unwrap();
    let decoded: baylee_view::PlayerView = serde_json::from_str(&wire).unwrap();
    assert_eq!(decoded.damage_sources, view.damage_sources);
}
