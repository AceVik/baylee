//! Preflight must not turn missing public knowledge into a negative proof.
use super::*;
use baylee_cards_dsl::TargetReq;
use baylee_core::ids::ObjectId;

#[test]
fn an_unknown_candidate_abandons_the_count_in_every_position() {
    let mut hidden = printed(2, 1, "Hidden", 2);
    hidden.card = None;
    hidden.token = None;
    let mut creature = printed(1, 0, "Known creature", 1);
    creature.types = TypeSet::CREATURE;
    let mut land = printed(3, 0, "Known land", 3);
    land.types = TypeSet::LAND;
    let mut objects = vec![creature, hidden, land];
    for _ in 0..objects.len() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, objects.clone())
            .build();
        assert_eq!(
            legal_targets(&view, &TargetSpec::Object(&Filter::CREATURE)),
            None
        );
        assert!(requirement_possible(
            &view,
            TargetReq::exactly(TargetSpec::Object(&Filter::CREATURE), 3)
        ));
        objects.rotate_left(1);
    }
}

#[test]
fn known_target_counts_respect_each_minimum_without_using_the_maximum() {
    for population in 0..=4_u8 {
        let objects: Vec<_> = (0..population)
            .map(|slot| {
                let mut object = printed(u32::from(slot), 0, "Creature", 1);
                object.types = TypeSet::CREATURE;
                object
            })
            .collect();
        let view = ViewBuilder::new(2).with_battlefield(0, objects).build();
        for minimum in 0..=5 {
            let mut req = TargetReq::exactly(TargetSpec::Object(&Filter::CREATURE), minimum);
            req.max = u8::MAX;
            assert_eq!(requirement_possible(&view, req), population >= minimum);
        }
    }
}

#[test]
fn hidden_fields_cannot_make_a_filter_or_its_negation_readable() {
    static HIDDEN_TEST: Filter = Filter::Not(&Filter::CREATURE);
    let view = ViewBuilder::new(2).build();
    let mut object = printed(1, 0, "Sensitive name", 1);
    object.card = None;
    object.token = None;
    object.types = TypeSet::CREATURE;
    object.keywords = KeywordSet::FLYING.bits();
    for filter in [
        Filter::Any,
        Filter::CREATURE,
        Filter::Named("Sensitive name"),
        Filter::HasKeyword(KeywordSet::FLYING),
    ] {
        assert_eq!(matches(&view, &object, &filter), None);
    }
    assert_eq!(matches(&view, &object, &HIDDEN_TEST), None);
}

#[test]
fn a_reused_attachment_host_slot_does_not_match_the_old_handle() {
    static ATTACHED: Filter = Filter::AttachedTo(&Filter::CREATURE);
    let mut aura = printed(1, 0, "Aura", 1);
    aura.attached_to = Some(ObjectId::new(2, 0));
    let mut replacement = printed(2, 0, "New host", 2);
    replacement.id = ObjectId::new(2, 1);
    replacement.types = TypeSet::CREATURE;
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![aura.clone(), replacement])
        .build();
    assert_eq!(matches(&view, &aura, &ATTACHED), None);
    assert_eq!(matches(&view, &aura, &Filter::Not(&ATTACHED)), None);
}

#[test]
fn visible_keyword_masks_use_any_bit_and_empty_masks_match_nothing() {
    let view = ViewBuilder::new(2).build();
    let mut object = printed(1, 0, "Keyword fixture", 1);
    for present in [
        KeywordSet::EMPTY,
        KeywordSet::FLYING,
        KeywordSet::TRAMPLE,
        KeywordSet::FLYING.union(KeywordSet::TRAMPLE),
    ] {
        object.keywords = present.bits();
        assert_eq!(
            matches(&view, &object, &Filter::HasKeyword(KeywordSet::EMPTY)),
            Some(false)
        );
        assert_eq!(
            matches(
                &view,
                &object,
                &Filter::HasKeyword(KeywordSet::FLYING.union(KeywordSet::TRAMPLE))
            ),
            Some(present != KeywordSet::EMPTY)
        );
    }
}

#[test]
fn spell_and_ability_target_specs_do_not_count_each_others_stack_rows() {
    let mut spell = printed(1, 0, "Spell", 1);
    spell.stack_item = Some(StackItem::Spell);
    let mut ability = printed(2, 1, "Ability", 2);
    ability.stack_item = Some(StackItem::Ability {
        source: ObjectId::new(9, 0),
        ability: None,
        rules: None,
        token: None,
        text: None,
    });
    for (rows, spells, abilities) in [
        (vec![], 0, 0),
        (vec![spell.clone()], 1, 0),
        (vec![ability.clone()], 0, 1),
        (vec![spell, ability], 1, 1),
    ] {
        let view = ViewBuilder::new(2).with_stack(rows).build();
        assert_eq!(
            legal_targets(&view, &TargetSpec::Spell(&Filter::Any)),
            Some(spells)
        );
        assert_eq!(
            legal_targets(&view, &TargetSpec::AbilityOnStack(&Filter::Any)),
            Some(abilities)
        );
        assert_eq!(
            legal_targets(&view, &TargetSpec::SpellOrAbility(&Filter::Any)),
            Some(spells + abilities)
        );
    }
}

#[test]
fn empty_public_zones_prove_no_targets_even_for_unreadable_filters() {
    let view = ViewBuilder::new(2).build();
    for spec in [
        TargetSpec::Object(&Filter::CmcExactlyX),
        TargetSpec::Spell(&Filter::CmcExactlyX),
        TargetSpec::CardInGraveyard(&Filter::CmcExactlyX, PlayerRel::You),
    ] {
        assert_eq!(legal_targets(&view, &spec), Some(0));
        assert!(!requirement_possible(&view, TargetReq::one(spec)));
    }
}
