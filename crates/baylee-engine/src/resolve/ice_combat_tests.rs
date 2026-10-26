//! Resolution subjects select the triggering attacker and original damage source.
use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
fn resolution(source: ObjectId, event: Option<ObjectId>) -> Resolution {
    Resolution {
        source,
        on_stack: source,
        controller: PlayerId::new(0),
        effects: vec![],
        pc: 0,
        targets: SmallVec::new(),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        event_mana: None,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        target_players: baylee_core::ids::SeatSet::new(),
        event_object: event,
        awaiting: None,
        targeted: false,
        mana_ability: false,
        countered_source: None,
        this_way_from: None,
        coin_stake: None,
    }
}
fn board() -> GameState {
    let mut game = preset(619, &[]);
    game.seats.push(game.seats[0].clone());
    GameState::from_preset(&game, &SyntheticLookup::new(vec![])).unwrap()
}
fn permanent(state: &mut GameState) -> ObjectId {
    let name = state.names.intern("Synthetic source");
    state.create_bare(
        PlayerId::new(0),
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    )
}
#[test]
fn defending_player_follows_triggering_attacker_instead_of_aura_source() {
    let mut state = board();
    let aura = permanent(&mut state);
    let attacker = permanent(&mut state);
    state
        .combat
        .declare_attackers([crate::combat::AttackerInfo {
            creature: attacker,
            defending: baylee_core::ids::Defender::Player(PlayerId::new(1)),
            blocked: false,
            band: None,
        }]);
    let res = resolution(aura, Some(attacker));
    assert_eq!(
        players_of(PlayerRel::DefendingPlayer, &state, PlayerId::new(0), &res),
        vec![PlayerId::new(1)]
    );
}
#[test]
fn fixed_source_prevention_binds_original_source_without_a_choice() {
    let mut state = board();
    let source = permanent(&mut state);
    let identity = state.source_identity(source).unwrap();
    let mut res = resolution(source, None);
    res.subject = SubjectContext::default().with_source_version(Some(identity.version));
    assert!(
        life::ice_combat::exec(
            &mut state,
            &mut res,
            Effect::PreventNextDamageFromSource {
                to: PlayerRel::You,
                combat_only: false
            }
        )
        .is_none()
    );
    let shield = state.shields.iter().next().unwrap();
    assert_eq!(
        shield.protects,
        crate::prevention::Shielded::Player(PlayerId::new(0))
    );
    let crate::prevention::ShieldKind::NextFrom { source: chosen, .. } = shield.kind else {
        panic!("source shield");
    };
    assert_eq!(
        (chosen.id, chosen.version),
        (identity.object, identity.version)
    );
}

#[test]
fn fixed_source_shield_retains_departed_source_and_consumes_only_its_next_event() {
    let mut state = board();
    let source = permanent(&mut state);
    let other = permanent(&mut state);
    let identity = state.source_identity(source).unwrap();
    let mut res = resolution(source, None);
    res.subject = SubjectContext::default().with_source_version(Some(identity.version));
    life::ice_combat::exec(
        &mut state,
        &mut res,
        Effect::PreventNextDamageFromSource {
            to: PlayerRel::You,
            combat_only: false,
        },
    );
    state
        .move_object(
            source,
            ZoneLocation::Graveyard(PlayerId::new(0)),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    for (id, version, expected) in [
        (other, None, 18),
        (source, Some(identity.version), 18),
        (source, Some(identity.version), 16),
    ] {
        let assignment = crate::damage::Assignment {
            source: id,
            source_version: version,
            recipient: crate::event::DamageTarget::Player(PlayerId::new(0)),
            amount: 2,
            is_combat: false,
        };
        assert!(
            crate::damage::DamageWork::new(&mut state, vec![assignment])
                .advance(&mut state)
                .is_none()
        );
        assert_eq!(state.players[0].life, expected);
    }
}

#[test]
fn landwalk_choice_offers_all_land_subtypes_and_affects_only_chosen_targets() {
    use baylee_cards_dsl::{Duration, Filter, Modifier};
    use baylee_core::{
        generated::subtypes::{ALL_LAND_TYPES, land},
        types::{SubtypeSet, TypeSet},
    };
    let mut state = board();
    let source = permanent(&mut state);
    let target = permanent(&mut state);
    let blocker = permanent(&mut state);
    for id in [source, target, blocker] {
        state.object_mut(id).unwrap().base_mut().types = TypeSet::CREATURE;
    }
    state.object_mut(blocker).unwrap().controller = PlayerId::new(1);
    let desert = permanent(&mut state);
    state.object_mut(desert).unwrap().controller = PlayerId::new(1);
    let chars = state.object_mut(desert).unwrap().base_mut();
    chars.types = TypeSet::LAND;
    chars.subtypes = SubtypeSet::from_slice(&[land::DESERT]);
    let mut res = resolution(source, None);
    res.targets.push(target);
    let Pending::ChooseSubtype { options, .. } = landwalk::choose(
        &state,
        &mut res,
        TargetSpec::Object(&Filter::CREATURE),
        false,
        &baylee_cards_dsl::Filter::Any,
        Duration::UntilEndOfTurn,
    ) else {
        panic!("subtype choice");
    };
    assert_eq!(options, ALL_LAND_TYPES.iter().collect::<Vec<_>>());
    assert!(crate::combat::can_block(
        &state,
        PlayerId::new(1),
        blocker,
        target
    ));
    assert!(matches!(
        landwalk::resume_with_subtype(&mut state, &mut res, land::DESERT),
        Flow::Complete
    ));
    assert!(!crate::combat::can_block(
        &state,
        PlayerId::new(1),
        blocker,
        target
    ));
    assert!(crate::combat::can_block(
        &state,
        PlayerId::new(1),
        blocker,
        source
    ));
    assert!(state.effects.iter().any(
        |effect| matches!(effect.modifier, Modifier::LandwalkOfSubtype { subtype, .. } if subtype == land::DESERT)
    ));
}
#[test]
fn basic_landwalk_choice_preserves_five_options_and_exact_source_incarnation() {
    use baylee_cards_dsl::Duration;
    use baylee_core::generated::subtypes::land;
    let mut state = board();
    let source = permanent(&mut state);
    let mut res = resolution(source, None);
    let Pending::ChooseSubtype { options, .. } = landwalk::choose(
        &state,
        &mut res,
        TargetSpec::ThisObject,
        true,
        &baylee_cards_dsl::Filter::Any,
        Duration::UntilEndOfTurn,
    ) else {
        panic!("subtype choice");
    };
    assert_eq!(options.len(), 5);
    assert!(!options.contains(&land::DESERT));
    state
        .move_object(
            source,
            ZoneLocation::Exile(PlayerId::new(0)),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            source,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert!(matches!(
        landwalk::resume_with_subtype(&mut state, &mut res, land::FOREST),
        Flow::Complete
    ));
    assert!(state.effects.iter().next().is_none());
}

#[test]
fn snow_landwalk_requires_the_chosen_type_and_snow_on_defending_land() {
    use baylee_cards_dsl::{Duration, Filter};
    use baylee_core::{
        generated::subtypes::land,
        types::{SubtypeSet, SupertypeSet, TypeSet},
    };
    let mut state = board();
    let walker = permanent(&mut state);
    let blocker = permanent(&mut state);
    let land = permanent(&mut state);
    for id in [walker, blocker] {
        state.object_mut(id).unwrap().base_mut().types = TypeSet::CREATURE;
    }
    state.object_mut(blocker).unwrap().controller = PlayerId::new(1);
    state.object_mut(land).unwrap().controller = PlayerId::new(1);
    state.object_mut(land).unwrap().base_mut().types = TypeSet::LAND;
    state.object_mut(land).unwrap().base_mut().subtypes = SubtypeSet::from_slice(&[land::FOREST]);
    let mut res = resolution(walker, None);
    landwalk::choose(
        &state,
        &mut res,
        TargetSpec::ThisObject,
        false,
        &Filter::HasSupertype(SupertypeSet::SNOW),
        Duration::UntilEndOfTurn,
    );
    assert!(matches!(
        landwalk::resume_with_subtype(&mut state, &mut res, land::FOREST),
        Flow::Complete
    ));
    assert!(crate::combat::can_block(
        &state,
        PlayerId::new(1),
        blocker,
        walker
    ));
    state.object_mut(land).unwrap().base_mut().supertypes = SupertypeSet::SNOW;
    assert!(!crate::combat::can_block(
        &state,
        PlayerId::new(1),
        blocker,
        walker
    ));
    state.object_mut(land).unwrap().base_mut().subtypes = SubtypeSet::from_slice(&[land::DESERT]);
    assert!(crate::combat::can_block(
        &state,
        PlayerId::new(1),
        blocker,
        walker
    ));
}

#[test]
fn attached_shield_is_untargeted_and_retains_only_the_original_host_after_source_leaves() {
    use crate::object::Rider;
    let mut state = board();
    let aura = permanent(&mut state);
    let host = permanent(&mut state);
    let stack = permanent(&mut state);
    state.object_mut(host).unwrap().base_mut().keywords = baylee_cards_dsl::KeywordSet::SHROUD;
    state.object_mut(aura).unwrap().attached_to = Some(host);
    let host_version = state.object(host).unwrap().version;
    let mut res = resolution(aura, None);
    res.on_stack = stack;
    res.effects = vec![Effect::PreventNextDamageToAttached {
        amount: Amount::Fixed(1),
    }];
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    assert_eq!(state.shields.iter().count(), 1);
    state.object_mut(stack).unwrap().riders.extend([
        Rider::SourceAttachmentLki(host),
        Rider::SourceAttachmentVersion(host_version),
    ]);
    state
        .move_object(
            aura,
            ZoneLocation::Graveyard(PlayerId::new(0)),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    res.pc = 0;
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    assert_eq!(state.shields.iter().count(), 2);
    state
        .move_object(
            host,
            ZoneLocation::Exile(PlayerId::new(0)),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            host,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let before = state.shields.iter().count();
    res.pc = 0;
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    assert_eq!(
        state.shields.iter().count(),
        before,
        "new host incarnation receives no shield"
    );
}

#[test]
fn target_iteration_runs_once_for_each_target_and_preserves_order() {
    let mut state = board();
    let source = permanent(&mut state);
    let one = permanent(&mut state);
    let two = permanent(&mut state);
    let other = permanent(&mut state);
    let mut res = resolution(source, None);
    res.targets.extend([one, two, other]);
    let old = state.object(other).unwrap();
    res.target_lki = Some(vec![TargetLki {
        id: other,
        version: old.version,
        controller: old.controller,
        chars: old.characteristics().clone(),
    }]);
    state
        .move_object(
            other,
            ZoneLocation::Exile(PlayerId::new(0)),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            other,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    res.effects = vec![Effect::ForEachTarget {
        effects: &[Effect::TapEventObject],
    }];
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    for id in [one, two] {
        assert!(
            state
                .object(id)
                .unwrap()
                .status
                .contains(crate::object::Status::TAPPED)
        );
    }
    assert!(
        !state
            .object(other)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
    assert_eq!(res.event_object, Some(two));
}
