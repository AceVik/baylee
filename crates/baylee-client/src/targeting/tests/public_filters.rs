//! Public filter semantics and the contexts this preflight still cannot read.

use super::*;
use baylee_client_core::test_support::{target, target_snapshot};
use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId, TargetRef};
use baylee_view::LogZone;

static FLYING: Filter = Filter::HasKeyword(KeywordSet::FLYING);
static CREATURE_WITH_FLYING: Filter = Filter::And(&[Filter::CREATURE, FLYING]);
static CREATURE_WITHOUT_FLYING: Filter = Filter::And(&[Filter::CREATURE, Filter::Not(&FLYING)]);
static TARGETS_MINE: Filter = Filter::TargetsPermanentYouControl;
static ACTIVATED_FROM_ARTIFACT: Filter = Filter::ActivatedAbilityFrom(&Filter::ARTIFACT);

#[test]
fn source_damage_history_stays_unknown_even_with_marked_damage() {
    for damage in [0, 3] {
        let mut object = printed(1, 0, "Damage history fixture", 1);
        object.damage = damage;
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![object.clone()])
            .build();
        for filter in [
            &Filter::DealtDamageToSourceThisTurn,
            &Filter::DealtDamageBySourceThisTurn,
        ] {
            assert_eq!(matches(&view, &object, filter), None);
            assert_eq!(matches(&view, &object, &Filter::Not(filter)), None);
            assert_eq!(legal_targets(&view, &TargetSpec::Object(filter)), None);
        }
    }
}

fn spell(targets: Vec<TargetRef>) -> PublicObject {
    let mut spell = printed(3, 1, "Stack fixture", 3);
    spell.types = TypeSet::INSTANT;
    spell.stack_item = Some(StackItem::Spell);
    spell.targets = targets;
    spell
}

#[test]
fn keywords_read_the_projection_and_match_any_requested_bit() {
    let requested = KeywordSet::FLYING.union(KeywordSet::HASTE);
    let mut object = printed(1, 0, "Keyword fixture", 1);
    let identity = object.card;
    for keywords in [
        KeywordSet::EMPTY,
        KeywordSet::FLYING,
        KeywordSet::HASTE,
        requested,
        KeywordSet::UNCOUNTERABLE,
    ] {
        object.keywords = keywords.bits();
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![object.clone()])
            .build();
        for wanted in [KeywordSet::EMPTY, KeywordSet::FLYING, requested] {
            let filter = Filter::HasKeyword(wanted);
            let expected = keywords.contains(wanted);
            assert_eq!(matches(&view, &object, &filter), Some(expected));
        }
        assert_eq!(
            matches(&view, &object, &Filter::Not(&FLYING)),
            Some(!keywords.contains(KeywordSet::FLYING))
        );
        assert_eq!(
            object.card, identity,
            "grants/removal do not change the card"
        );
    }
}

#[test]
fn keyword_filters_count_creatures_and_preserve_negation() {
    let mut flying = printed(1, 0, "Flying fixture", 1);
    flying.keywords = KeywordSet::FLYING.bits();
    let grounded = printed(2, 1, "Grounded fixture", 2);
    let mut land = printed(4, 1, "Land fixture", 4);
    land.types = TypeSet::LAND;
    land.keywords = KeywordSet::FLYING.bits();
    for objects in [
        vec![],
        vec![grounded.clone()],
        vec![flying.clone()],
        vec![flying, grounded, land],
    ] {
        let flying_count = objects
            .iter()
            .filter(|o| o.types.intersects(TypeSet::CREATURE) && o.keywords != 0)
            .count();
        let grounded_count = objects
            .iter()
            .filter(|o| o.types.intersects(TypeSet::CREATURE) && o.keywords == 0)
            .count();
        let view = ViewBuilder::new(2).with_battlefield(0, objects).build();
        assert_eq!(
            legal_targets(&view, &TargetSpec::Object(&CREATURE_WITH_FLYING)),
            Some(flying_count)
        );
        assert_eq!(
            legal_targets(&view, &TargetSpec::Object(&CREATURE_WITHOUT_FLYING)),
            Some(grounded_count)
        );
    }
}

#[test]
fn keyword_bits_do_not_bypass_the_hidden_object_guard() {
    for keywords in [0, KeywordSet::FLYING.bits()] {
        let mut hidden = printed(1, 0, "Hidden fixture", 1);
        hidden.card = None;
        hidden.rules = None;
        hidden.token = None;
        hidden.keywords = keywords;
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![hidden.clone()])
            .build();
        assert_eq!(matches(&view, &hidden, &FLYING), None);
        assert_eq!(matches(&view, &hidden, &Filter::Not(&FLYING)), None);
        assert_eq!(
            legal_targets(&view, &TargetSpec::Object(&CREATURE_WITH_FLYING)),
            None
        );
    }
    let mut token = printed(1, 0, "Token fixture", 1);
    token.card = None;
    token.rules = None;
    token.token = Some(0);
    token.keywords = KeywordSet::FLYING.bits();
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![token.clone()])
        .build();
    assert_eq!(matches(&view, &token, &FLYING), Some(true));
}

#[test]
fn rider_derived_uncounterability_is_not_a_proven_keyword_characteristic() {
    static UNCOUNTERABLE: Filter = Filter::HasKeyword(KeywordSet::UNCOUNTERABLE);
    let mut object = spell(vec![]);
    object.keywords = KeywordSet::UNCOUNTERABLE.bits();
    let view = ViewBuilder::new(2).with_stack(vec![object.clone()]).build();
    assert_eq!(matches(&view, &object, &UNCOUNTERABLE), None);
    assert_eq!(matches(&view, &object, &Filter::Not(&UNCOUNTERABLE)), None);
    assert_eq!(matches(&view, &object, &FLYING), Some(false));
    object.keywords |= KeywordSet::FLYING.bits();
    assert_eq!(matches(&view, &object, &FLYING), Some(true));
    assert_eq!(
        matches(
            &view,
            &object,
            &Filter::HasKeyword(KeywordSet::FLYING.union(KeywordSet::UNCOUNTERABLE))
        ),
        Some(true),
        "a genuine projected keyword can establish the any-bit match"
    );
    object.stack_item = None;
    assert_eq!(matches(&view, &object, &UNCOUNTERABLE), Some(true));
}

#[test]
fn targets_mine_checks_objects_not_players_owners_or_the_stack_controller() {
    let mine = printed(1, 0, "My fixture", 1);
    let mut theirs = printed(2, 1, "Their fixture", 2);
    theirs.owner = PlayerId::new(0);
    let missing = target(ObjectId::new(99, 0));
    for (targets, expected) in [
        (vec![], Some(false)),
        (vec![TargetRef::Player(PlayerId::new(0))], Some(false)),
        (vec![target(theirs.id)], Some(false)),
        (vec![target(mine.id)], Some(true)),
        (vec![target(theirs.id), target(mine.id)], Some(true)),
        (vec![missing], None),
        (vec![missing, target(mine.id)], Some(true)),
    ] {
        let object = spell(targets);
        let mut view = ViewBuilder::new(2)
            .with_battlefield(0, vec![mine.clone()])
            .with_battlefield(1, vec![theirs.clone()])
            .with_stack(vec![object.clone()])
            .build();
        // ViewBuilder normalizes ownership; restore this control-change case.
        view.battlefield
            .iter_mut()
            .find(|o| o.id == theirs.id)
            .unwrap()
            .owner = view.seat;
        assert_eq!(matches(&view, &object, &TARGETS_MINE), expected);
        assert_eq!(
            matches(&view, &object, &Filter::Not(&TARGETS_MINE)),
            expected.map(|b| !b)
        );
        assert_eq!(
            legal_targets(&view, &TargetSpec::Spell(&TARGETS_MINE)),
            expected.map(usize::from)
        );
    }
}

#[test]
fn targets_mine_uses_current_control_at_any_table_size() {
    for seats in [2, 4] {
        for controller in 0..seats {
            let permanent = printed(1, controller, "Control fixture", 1);
            let object = spell(vec![target(permanent.id)]);
            let view = ViewBuilder::new(seats)
                .with_battlefield(controller, vec![permanent])
                .with_stack(vec![object.clone()])
                .build();
            assert_eq!(
                matches(&view, &object, &TARGETS_MINE),
                Some(controller == 0)
            );
        }
    }
}

#[test]
fn targets_mine_requires_a_current_battlefield_incarnation() {
    let permanent = printed(1, 0, "Version fixture", 1);
    let object = spell(vec![target(permanent.id)]);
    for (zone, current) in [
        (LogZone::Battlefield, true),
        (LogZone::Battlefield, false),
        (LogZone::Graveyard, true),
        (LogZone::Exile, true),
        (LogZone::Stack, true),
        (LogZone::Hand, true),
    ] {
        let mut view = ViewBuilder::new(2)
            .with_battlefield(0, vec![permanent.clone()])
            .with_stack(vec![object.clone()])
            .build();
        let mut snapshot = target_snapshot(&permanent, zone);
        snapshot.is_current = current;
        view.target_objects = vec![snapshot];
        let expected = Some(current && zone == LogZone::Battlefield);
        assert_eq!(matches(&view, &object, &TARGETS_MINE), expected);
        assert_eq!(
            matches(&view, &object, &Filter::Not(&TARGETS_MINE)),
            expected.map(|b| !b)
        );
    }
}

#[test]
fn a_returned_permanent_cannot_replace_the_original_target() {
    let permanent = printed(1, 0, "Returned fixture", 1);
    let mut old = target_snapshot(&permanent, LogZone::Battlefield);
    old.is_current = false;
    let mut new = old.clone();
    new.source.version += 1;
    new.is_current = true;
    let object = spell(vec![TargetRef::Object(old.source)]);
    let mut view = ViewBuilder::new(2)
        .with_battlefield(0, vec![permanent])
        .with_stack(vec![object.clone()])
        .build();
    view.target_objects = vec![new.clone(), old];
    assert_eq!(matches(&view, &object, &TARGETS_MINE), Some(false));
    view.target_objects = vec![new.clone()];
    assert_eq!(
        matches(&view, &object, &TARGETS_MINE),
        None,
        "a missing exact row is unknown, even when its handle is live"
    );
    assert_eq!(matches(&view, &object, &Filter::Not(&TARGETS_MINE)), None);
    let current = spell(vec![TargetRef::Object(new.source)]);
    assert_eq!(matches(&view, &current, &TARGETS_MINE), Some(true));
}

#[test]
fn hidden_target_descriptions_preserve_unknown_under_negation() {
    let permanent = printed(1, 0, "Hidden target fixture", 1);
    let object = spell(vec![target(permanent.id)]);
    let mut view = ViewBuilder::new(2)
        .with_battlefield(0, vec![permanent.clone()])
        .with_stack(vec![object.clone()])
        .build();
    let mut snapshot = target_snapshot(&permanent, LogZone::Battlefield);
    snapshot.card = None;
    snapshot.rules = None;
    snapshot.token = None;
    view.target_objects = vec![snapshot];
    assert_eq!(matches(&view, &object, &TARGETS_MINE), None);
    assert_eq!(matches(&view, &object, &Filter::Not(&TARGETS_MINE)), None);
    view.target_objects[0].token = Some(0);
    assert_eq!(matches(&view, &object, &TARGETS_MINE), Some(true));
    let mut hidden_candidate = object;
    hidden_candidate.card = None;
    hidden_candidate.rules = None;
    hidden_candidate.token = None;
    assert_eq!(matches(&view, &hidden_candidate, &TARGETS_MINE), None);
}

#[test]
fn an_unannounced_x_stays_unknown_for_every_candidate_mana_value() {
    for mana_value in [0, 1, 5, u32::MAX] {
        let mut object = printed(1, 0, "X fixture", 1);
        object.mana_value = mana_value;
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![object.clone()])
            .build();
        assert_eq!(matches(&view, &object, &Filter::CmcExactlyX), None);
        assert_eq!(
            matches(&view, &object, &Filter::Not(&Filter::CmcExactlyX)),
            None
        );
        assert_eq!(
            legal_targets(&view, &TargetSpec::Object(&Filter::CmcExactlyX)),
            None
        );
        assert!(matches!(
            unreadable_leaf(&view, &object, &Filter::CmcExactlyX),
            Some(Filter::CmcExactlyX)
        ));
    }
}

#[test]
fn ability_origin_stays_unknown_without_authoritative_kind_and_source_version() {
    let mut source = printed(1, 0, "Ability source fixture", 1);
    source.types = TypeSet::ARTIFACT;
    let mut object = spell(vec![]);
    object.stack_item = Some(StackItem::Ability {
        source: source.id,
        ability: None,
        rules: None,
        token: None,
        text: None,
    });
    for live_source in [true, false] {
        let mut view = ViewBuilder::new(2)
            .with_battlefield(
                0,
                if live_source {
                    vec![source.clone()]
                } else {
                    vec![]
                },
            )
            .with_stack(vec![object.clone()])
            .build();
        // Even a source-choice LKI row is not an activated/triggered tag.
        let mut lki = target_snapshot(&source, LogZone::Battlefield);
        lki.source = DamageSourceRef {
            object: source.id,
            version: 1,
        };
        lki.is_current = live_source;
        lki.referenced_by = vec![object.id];
        view.damage_sources = vec![lki];
        assert_eq!(matches(&view, &object, &ACTIVATED_FROM_ARTIFACT), None);
        assert_eq!(
            matches(&view, &object, &Filter::Not(&ACTIVATED_FROM_ARTIFACT)),
            None
        );
        assert_eq!(
            legal_targets(&view, &TargetSpec::AbilityOnStack(&ACTIVATED_FROM_ARTIFACT)),
            None
        );
        assert!(matches!(
            unreadable_leaf(&view, &object, &ACTIVATED_FROM_ARTIFACT),
            Some(Filter::ActivatedAbilityFrom(_))
        ));
    }
}
