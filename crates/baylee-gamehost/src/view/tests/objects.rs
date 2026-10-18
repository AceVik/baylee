use super::*;

#[test]
fn second_instance_player_targets_remain_distinct_in_the_public_stack_view() {
    let mut state = GameState::from_preset(&mixed_print_preset(), &Registry).unwrap();
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let card = state.zones.list(ZoneLocation::Library(p0))[0];
    let source = state
        .move_object(
            card,
            ZoneLocation::Stack,
            baylee_engine::zone::ZonePosition::Top,
            baylee_engine::event::Cause::Effect,
        )
        .unwrap();
    let object = state.object_mut(source).unwrap();
    object.target_players.insert(p0);
    object.set_second_players([p0, p1].into_iter().collect());
    let public = super::super::object::public_object(&state, source, p0).unwrap();
    assert_eq!(
        public.targets,
        [
            TargetRef::Player(p0),
            TargetRef::Player(p0),
            TargetRef::Player(p1)
        ]
    );
}

#[test]
fn second_generation_copy_keeps_vesuvans_quote_and_a_tokens_original_index() {
    use baylee_core::generated::index;
    use baylee_engine::copiable_abilities::compose;
    use baylee_engine::object::AbilityList;
    let mut state = GameState::from_preset(&commander_preset(), &Registry).unwrap();
    let definition = baylee_cards::by_index(index::VESUVAN_DOPPELGANGER).unwrap();
    let own = AbilityList::from_static(
        definition.abilities_for_face(0),
        PrintedFace::new(definition.index, 0),
        None,
    );
    let AbilityDef::CopyOnEnter { mods, .. } = own.abilities[0] else {
        panic!("Vesuvan copy clause");
    };
    let token_id = baylee_cards::tokens::token_id(&baylee_cards::tokens::TREASURE);
    let token = AbilityList::from_static(
        baylee_cards::tokens::TREASURE.abilities,
        None,
        std::num::NonZeroU16::new(token_id + 1),
    );
    let first = compose(token, &own, 0, mods, None);
    let second = compose(first, &AbilityList::NONE, 0, &[], None);
    let quoted = stacked(&mut state, index::CLONE, 1, second.clone());
    let Some(baylee_view::StackItem::Ability {
        ability,
        rules,
        text,
        token,
        ..
    }) = stack_item(&quoted)
    else {
        panic!("stack ability");
    };
    assert_eq!(
        ability,
        Some(baylee_core::ids::AbilityRef::new(
            index::VESUVAN_DOPPELGANGER,
            0
        ))
    );
    assert_eq!(
        rules,
        Some(rules_face(
            PrintedFace::new(index::VESUVAN_DOPPELGANGER, 0).unwrap()
        ))
    );
    assert_eq!(
        text,
        stack_text(PrintedFace::new(index::VESUVAN_DOPPELGANGER, 0).unwrap(), 0)
    );
    assert_eq!(
        token, None,
        "the extra ability is quoted by Vesuvan, not printed on Treasure"
    );
    let inherited = stacked(&mut state, index::CLONE, 0, second);
    let Some(baylee_view::StackItem::Ability {
        ability,
        rules,
        token,
        ..
    }) = stack_item(&inherited)
    else {
        panic!("stack ability");
    };
    assert_eq!(ability, None);
    assert_eq!(rules, None);
    assert_eq!(
        token,
        Some(baylee_view::TokenAbility {
            token: token_id,
            index: 0
        })
    );
}

/// A stack entry indexes the sentences of the face its ability was taken
/// from. Sheoldred's back face is the only one in the pool that puts an
/// ability on the stack; read against the face the source shows *now*,
/// it would print the other side's sentence as precise text.
#[test]
fn a_stack_entry_names_the_face_its_ability_was_taken_from() {
    let engine = Engine::new(&commander_preset(), Registry).expect("game starts");
    let mut state = engine.state().clone();
    let sheoldred = baylee_cards::all()
        .find(|d| d.name() == "Sheoldred")
        .expect("the pool has Sheoldred");

    let back = PrintedFace::new(sheoldred.index, 1).expect("fits");
    let (index, line) = (0..sheoldred.abilities_for_face(1).len())
        .filter_map(|i| u32::try_from(i).ok())
        .find_map(|i| baylee_cards::lines::ability_line(sheoldred.index, 1, i).map(|l| (i, l)))
        .expect("the back face puts an ability on the stack");
    let own = stacked(
        &mut state,
        sheoldred.index,
        index,
        baylee_engine::object::AbilityList {
            acquisition: 0,
            token: None,
            abilities: sheoldred.abilities_for_face(1).into(),
            printed: Some(back),
        },
    );
    let Some(baylee_view::StackItem::Ability { text, rules, .. }) = stack_item(&own) else {
        panic!("an ability on the stack is an ability");
    };
    assert_eq!(
        rules,
        Some(RulesFace {
            card: sheoldred.index,
            face: 1
        })
    );
    assert_eq!(
        text,
        Some(baylee_view::StackText {
            face: 1,
            line: line.line,
            of: line.of
        }),
        "the back face's sentence, whatever the source shows now"
    );
}

/// A copy's ability and standing-policy handle name the original printed
/// clause. Its source object independently preserves physical identity.
#[test]
fn a_copys_stack_entry_names_the_card_it_copied() {
    let engine = Engine::new(&commander_preset(), Registry).expect("game starts");
    let mut state = engine.state().clone();
    let solemn = baylee_cards::all()
        .find(|d| d.name() == "Solemn Simulacrum")
        .expect("the pool has Solemn Simulacrum");
    let spark_double = baylee_cards::all()
        .find(|d| d.name() == "Spark Double")
        .expect("the pool has Spark Double");

    let (index, line) = (0..solemn.abilities_for_face(0).len())
        .filter_map(|i| u32::try_from(i).ok())
        .find_map(|i| baylee_cards::lines::ability_line(solemn.index, 0, i).map(|l| (i, l)))
        .expect("Solemn's triggers have sentences");
    let copied = stacked(
        &mut state,
        spark_double.index,
        index,
        baylee_engine::object::AbilityList {
            acquisition: 0,
            token: None,
            abilities: solemn.abilities_for_face(0).into(),
            printed: PrintedFace::new(solemn.index, 0),
        },
    );
    let Some(baylee_view::StackItem::Ability {
        ability,
        text,
        rules,
        ..
    }) = stack_item(&copied)
    else {
        panic!("an ability on the stack is an ability");
    };
    assert_eq!(
        ability.map(|a| a.card),
        Some(solemn.index),
        "the stable handle follows the original clause, not a composed runtime index"
    );
    assert_eq!(
        rules,
        Some(RulesFace {
            card: solemn.index,
            face: 0
        }),
        "but the ability is printed on the card it copied"
    );
    assert_eq!(
        text,
        Some(baylee_view::StackText {
            face: 0,
            line: line.line,
            of: line.of
        })
    );

    let token = stacked(
        &mut state,
        spark_double.index,
        0,
        baylee_engine::object::AbilityList {
            acquisition: 0,
            token: None,
            abilities: solemn.abilities_for_face(0).into(),
            printed: None,
        },
    );
    let Some(baylee_view::StackItem::Ability { text, rules, .. }) = stack_item(&token) else {
        panic!("an ability on the stack is an ability");
    };
    assert_eq!(
        (text, rules),
        (None, None),
        "a list no card prints has no sentence to point at"
    );
}

/// `rules` is the card itself for everything that is not a copy, and is
/// withheld exactly where `card` is: a face-down permanent's controller
/// sees it, and nobody else does.
#[test]
fn an_object_names_its_own_card_unless_it_may_not_be_known() {
    let engine = Engine::new(&commander_preset(), Registry).expect("game starts");
    let mut state = engine.state().clone();
    let mut seen = 0;
    for seat in [0u8, 1] {
        let view = player_view(
            &state,
            PlayerId::new(seat),
            0,
            None,
            &SeatContext::default(),
            &[],
        );
        for object in view.battlefield.iter().chain(view.command.iter().flatten()) {
            let Some(card) = object.card else { continue };
            assert_eq!(
                object.rules,
                Some(RulesFace {
                    card: card.index,
                    face: card.face
                }),
                "{}",
                object.name
            );
            seen += 1;
        }
    }
    assert!(seen > 0, "the preset puts cards where a view shows them");

    let hidden = state
        .zones
        .list(ZoneLocation::Battlefield)
        .first()
        .copied()
        .or_else(|| state.commanders[0].first().map(|c| c.object))
        .expect("some object to turn face down");
    let owner = state.object(hidden).expect("it exists").controller;
    state
        .object_mut(hidden)
        .expect("it exists")
        .status
        .insert(baylee_engine::object::Status::FACE_DOWN);
    let other = PlayerId::new(1 - owner.get());
    for (seat, entitled) in [(owner, true), (other, false)] {
        let view = player_view(&state, seat, 0, None, &SeatContext::default(), &[]);
        let Some(object) = view
            .battlefield
            .iter()
            .chain(view.command.iter().flatten())
            .find(|o| o.id == hidden)
        else {
            continue;
        };
        assert_eq!(object.card.is_some(), entitled);
        assert!(
            object.rules.is_none(),
            "a face-down permanent has its face-down rules, not its printed abilities; the owner can still inspect object.card"
        );
    }
}

/// A permanent phased out with what it is attached to (CR 702.26g) is
/// shown as phased out, the status CR 110.5 names, and as nothing else:
/// the engine's note of how it phased out is not a status a client knows.
#[test]
fn a_permanent_phased_out_indirectly_is_shown_as_phased_out() {
    use baylee_engine::object::Status;
    let engine = Engine::new(&mixed_print_preset(), Registry).expect("game starts");
    let mut state = engine.state().clone();
    let id = state
        .zones
        .list(ZoneLocation::Battlefield)
        .first()
        .copied()
        .expect("the preset puts Islands on the battlefield");
    let status = &mut state.object_mut(id).expect("it exists").status;
    status.insert(Status::PHASED_OUT);
    status.insert(Status::PHASED_OUT_INDIRECTLY);
    for seat in [0u8, 1] {
        let view = player_view(
            &state,
            PlayerId::new(seat),
            0,
            None,
            &SeatContext::default(),
            &[],
        );
        let object = view
            .battlefield
            .iter()
            .find(|o| o.id == id)
            .expect("a phased-out permanent is on the battlefield");
        assert_eq!(
            object.status.bits(),
            Status::PHASED_OUT.bits(),
            "seat {seat} is shown more than \"phased out\""
        );
    }
}

#[test]
fn prepared_view_uses_the_exiled_spells_face_after_its_permanent_changes() {
    use baylee_core::generated::index;
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_battlefield = vec![DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: index::FATEHOLD_CHRONOLOGIST,
        print: PrintRef::new(0),
    }];
    let engine = Engine::new(&preset, Registry).unwrap();
    let mut state = engine.state().clone();
    let source = state
        .battlefield_seen()
        .find(|id| {
            state.object(*id).is_some_and(|object| {
                object
                    .card
                    .is_some_and(|card| card.index == index::FATEHOLD_CHRONOLOGIST)
            })
        })
        .unwrap();
    let copy = state.prepared_copy(source).unwrap();
    let other = baylee_cards::by_index(index::EMERITUS_OF_WOE).unwrap();
    state.object_mut(source).unwrap().take_abilities(
        baylee_engine::object::AbilityList::from_static(
            other.abilities,
            PrintedFace::new(other.index, 0),
            None,
        ),
    );
    for seat in [PlayerId::new(0), PlayerId::new(1)] {
        let view = player_view(&state, seat, 0, None, &SeatContext::default(), &[]);
        assert_eq!(
            view.object(source).unwrap().prepared_spell,
            Some(RulesFace {
                card: index::FATEHOLD_CHRONOLOGIST,
                face: 1,
            })
        );
        assert_eq!(view.object(copy).unwrap().name, "Peer Review");
    }
}

/// Report 21: a creature under "loses all abilities" (Oko, Thief of
/// Crowns's +1, Final Showdown, Tishana's Tidebinder) carries
/// `ObjectStatus::NO_ABILITIES` in every seat's view, an ordinary one does
/// not, and a face-down permanent never has it (its characteristics are not
/// the seat's to read). On the old code `status_of` did not exist: the view
/// carried only the engine's four public statuses, so the first assertion
/// fails.
#[test]
fn a_creature_that_lost_all_its_abilities_says_so_in_every_seats_view() {
    use baylee_core::ids::EffectId;
    use baylee_engine::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
    use baylee_engine::object::Status;
    use baylee_view::ObjectStatus;

    let bears = baylee_cards::by_oracle_id("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
        .expect("Grizzly Bears is in the pool")
        .index;
    let entry = DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: bears,
        print: PrintRef::new(0),
    };
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_battlefield = vec![entry, entry, entry];
    let mut state = GameState::from_preset(&preset, &Registry).unwrap();
    let bears: Vec<ObjectId> = state.zones.list(ZoneLocation::Battlefield).clone();
    let [lost, ordinary, hidden] = bears[..] else {
        panic!("three Grizzly Bears on the battlefield, got {bears:?}");
    };
    state
        .object_mut(hidden)
        .unwrap()
        .status
        .insert(Status::FACE_DOWN);
    for target in [lost, hidden] {
        let filter = EffectFilter::object(&state, target);
        let modifier = baylee_cards_dsl::Modifier::LoseAllAbilities;
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: Some(target),
            controller: PlayerId::new(0),
            origin: EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp: 1_000,
            duration: baylee_cards_dsl::Duration::Indefinitely,
            filter,
            modifier,
        });
    }
    state.refresh_characteristics();

    for seat in [PlayerId::new(0), PlayerId::new(1)] {
        let view = player_view(&state, seat, 0, None, &SeatContext::default(), &[]);
        let status = |id: ObjectId| {
            view.battlefield
                .iter()
                .find(|o| o.id == id)
                .unwrap_or_else(|| panic!("{id:?} is on the shared battlefield"))
                .status
        };
        assert!(
            status(lost).has_no_abilities(),
            "{seat:?} is told the creature lost its abilities"
        );
        assert!(
            !status(ordinary).has_no_abilities(),
            "{seat:?}: an ordinary creature is not"
        );
        assert!(
            !status(hidden).has_no_abilities(),
            "{seat:?}: a face-down permanent never carries the bit"
        );
        assert!(
            status(hidden).contains(ObjectStatus::FACE_DOWN),
            "the face-down bit itself still rides"
        );
    }
}

#[test]
fn merged_top_printing_is_visible_without_publishing_a_second_command_object() {
    use baylee_engine::object::Rider;
    let p0 = PlayerId::new(0);
    let mut state = GameState::from_preset(&mixed_print_preset(), &Registry).unwrap();
    let root = state.zones.list(ZoneLocation::Battlefield)[0];
    let top = state.zones.list(ZoneLocation::Hand(p0))[0];
    let physical_root = state.object(root).unwrap().card.unwrap();
    let top_print = state.object(top).unwrap().card.unwrap();
    state
        .move_object(
            top,
            ZoneLocation::Command(p0),
            baylee_engine::zone::ZonePosition::Top,
            baylee_engine::event::Cause::Spell,
        )
        .unwrap();
    state
        .object_mut(top)
        .unwrap()
        .riders
        .push(Rider::MergedInto(root));
    state.object_mut(root).unwrap().riders.extend([
        Rider::MergedComponent(top),
        Rider::MergedTop(top),
        Rider::MergedToken(false),
    ]);
    let public = super::super::object::public_object(&state, root, p0).unwrap();
    assert_eq!(public.card.unwrap().print, top_print.print);
    assert_eq!(state.object(root).unwrap().card.unwrap(), physical_root);
    assert!(
        super::super::zones::zone(
            &state,
            ZoneLocation::Command(p0),
            p0,
            baylee_core::ids::SeatSet::new()
        )
        .is_empty()
    );
}

#[test]
fn melded_card_art_does_not_report_token_identity() {
    let p0 = PlayerId::new(0);
    let mut state = GameState::from_preset(&mixed_print_preset(), &Registry).unwrap();
    let root = state.zones.list(ZoneLocation::Battlefield)[0];
    state.object_mut(root).unwrap().token =
        Some(&baylee_cards::tokens::HANWEIR_THE_WRITHING_TOWNSHIP);
    let public = super::super::object::public_object(&state, root, p0).unwrap();
    assert!(public.token.is_some());
    assert!(
        public
            .status
            .contains(baylee_view::ObjectStatus::MELDED_CARD)
    );
    assert!(!public.status.contains(baylee_view::ObjectStatus::TOKEN));
}

#[test]
fn dungeon_and_room_ability_are_not_reported_as_creature_tokens() {
    use baylee_engine::object::ObjectKind;
    let mut state = GameState::from_preset(&mixed_print_preset(), &Registry).unwrap();
    let seat = PlayerId::new(0);
    let name = state.names.intern("Dungeon room");
    for (kind, zone) in [
        (ObjectKind::Emblem, ZoneLocation::Command(seat)),
        (ObjectKind::AbilityOnStack, ZoneLocation::Stack),
    ] {
        let id = state.create_bare(seat, kind, name, zone);
        let public = super::super::object::public_object(&state, id, seat).unwrap();
        assert!(!public.status.contains(baylee_view::ObjectStatus::TOKEN));
    }
}
