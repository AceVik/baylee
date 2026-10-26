//! Characteristic counts and supertype dependencies independent of card definitions.

use super::*;
use baylee_cards_dsl::PtCount;
use baylee_core::types::SupertypeSet;

#[test]
fn supertype_changes_preserve_types_and_other_supertypes() {
    let mut state = fresh();
    let land = permanent(&mut state, "Basic land", TypeSet::LAND, None);
    state.object_mut(land).unwrap().base_mut().supertypes = SupertypeSet::BASIC;
    register(&mut state, 1, Modifier::AddSupertype(SupertypeSet::SNOW));
    let result = recompute(&state, state.object(land).unwrap()).characteristics;
    assert_eq!(
        result.supertypes,
        SupertypeSet::BASIC.union(SupertypeSet::SNOW)
    );
    assert_eq!(result.types, TypeSet::LAND);
    register(&mut state, 2, Modifier::RemoveSupertype(SupertypeSet::SNOW));
    let result = recompute(&state, state.object(land).unwrap()).characteristics;
    assert_eq!(result.supertypes, SupertypeSet::BASIC);
    assert_eq!(result.types, TypeSet::LAND);
}

#[test]
fn supertype_reading_effect_waits_for_addition_or_removal_despite_timestamp() {
    for modifier in [
        Modifier::AddSupertype(SupertypeSet::SNOW),
        Modifier::RemoveSupertype(SupertypeSet::SNOW),
    ] {
        let changes = fx(1, 9, &Filter::Any, modifier);
        let reads = fx(
            2,
            1,
            &Filter::HasSupertype(SupertypeSet::SNOW),
            Modifier::AddType(TypeSet::ARTIFACT),
        );
        let effects = [reads, changes];
        let mut order = vec![0, 1];
        sort_by_dependency(&effects, &mut order, Layer::Type);
        assert_eq!(order, vec![1, 0]);
    }
}

#[test]
fn graveyard_count_counts_cards_in_both_graveyards_and_the_subject_itself() {
    let mut state = fresh();
    let subject = permanent(
        &mut state,
        "Counting creature",
        TypeSet::CREATURE,
        Some((0, 1)),
    );
    register(
        &mut state,
        1,
        Modifier::CharacteristicPT {
            count: PtCount::InAllGraveyards(&Filter::CREATURE),
            toughness_plus: 1,
        },
    );
    assert_eq!(body(&state, subject), (Some(0), Some(1)));
    // Synthetic cards and a token have identical types; only the cards count.
    for (seat, card, types) in [
        (0, true, TypeSet::CREATURE),
        (1, true, TypeSet::CREATURE),
        (1, false, TypeSet::CREATURE),
        (0, true, TypeSet::LAND),
    ] {
        let name = state.names.intern("Graveyard object");
        let id = state.create_bare(
            PlayerId::new(seat),
            crate::object::ObjectKind::Card,
            name,
            crate::zone::ZoneLocation::Graveyard(PlayerId::new(seat)),
        );
        let object = state.object_mut(id).unwrap();
        object.base_mut().types = types;
        if card {
            object.card = Some(crate::object::CardRef {
                index: baylee_core::ids::CardIndex::new(1),
                print: baylee_core::ids::PrintRef::new(0),
            });
        }
    }
    assert_eq!(body(&state, subject), (Some(2), Some(3)));
    state.object_mut(subject).unwrap().card = Some(crate::object::CardRef {
        index: baylee_core::ids::CardIndex::new(1),
        print: baylee_core::ids::PrintRef::new(0),
    });
    state
        .move_object(
            subject,
            crate::zone::ZoneLocation::Graveyard(me()),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    let object = state.object(subject).unwrap();
    let mut read_board = false;
    assert_eq!(
        counts::pt_count(
            &state,
            object,
            object.characteristics(),
            me(),
            PtCount::InAllGraveyards(&Filter::CREATURE),
            crate::text_changes::RuleContext {
                source: subject,
                text: crate::text_changes::TextChangeMap::IDENTITY,
                resolved_name: None,
                reference: None
            },
            &mut read_board
        ),
        3
    );
}

#[test]
fn chosen_player_count_is_zero_without_choice_and_excludes_other_controllers() {
    let mut state = fresh();
    let subject = permanent(
        &mut state,
        "Chosen controller count",
        TypeSet::CREATURE,
        Some((1, 1)),
    );
    let other = permanent(
        &mut state,
        "Opponent creature",
        TypeSet::CREATURE,
        Some((1, 1)),
    );
    state.object_mut(other).unwrap().base_controller = PlayerId::new(1);
    state.object_mut(other).unwrap().controller = PlayerId::new(1);
    register(
        &mut state,
        1,
        Modifier::CharacteristicPT {
            count: PtCount::ChosenPlayerControls(&Filter::CREATURE),
            toughness_plus: 0,
        },
    );
    assert_eq!(body(&state, subject), (Some(0), Some(0)));
    state
        .object_mut(subject)
        .unwrap()
        .set_chosen_opponent(Some(PlayerId::new(1)));
    assert_eq!(body(&state, subject), (Some(1), Some(1)));
    permanent(
        &mut state,
        "Unchosen creature",
        TypeSet::CREATURE,
        Some((1, 1)),
    );
    assert_eq!(body(&state, subject), (Some(1), Some(1)));
}
