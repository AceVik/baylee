//! Generic attachment-filter matching and structural readability regressions.

use super::*;
use baylee_core::ids::ObjectId;

static CREATURE_OR_LAND: Filter = Filter::Or(&[Filter::CREATURE, Filter::LAND]);
static ATTACHED_TO_CREATURE_OR_LAND: Filter = Filter::AttachedTo(&CREATURE_OR_LAND);

fn attached_pair(host_types: TypeSet) -> (PublicObject, PublicObject) {
    let mut host = printed(2, 1, "Host fixture", 2);
    host.types = host_types;
    let mut attachment = printed(1, 0, "Attachment fixture", 1);
    attachment.types = TypeSet::ENCHANTMENT;
    attachment.attached_to = Some(host.id);
    (attachment, host)
}

#[test]
fn attachment_matches_either_creature_or_land_host() {
    for host_types in [TypeSet::CREATURE, TypeSet::LAND] {
        let (attachment, host) = attached_pair(host_types);
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![attachment.clone()])
            .with_battlefield(1, vec![host])
            .build();
        assert_eq!(
            matches(&view, &attachment, &ATTACHED_TO_CREATURE_OR_LAND),
            Some(true),
            "the host's {host_types:?} type matches, although the attachment's does not"
        );
        assert_eq!(
            matches(
                &view,
                &attachment,
                &Filter::Not(&ATTACHED_TO_CREATURE_OR_LAND)
            ),
            Some(false)
        );
    }
}

#[test]
fn attachment_evaluates_nested_predicates_on_the_host_with_the_same_viewer() {
    static INNER: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::ControlledByOpponent,
        Filter::Not(&Filter::LAND),
    ]);
    let (attachment, host) = attached_pair(TypeSet::CREATURE);
    for controller in [0, 1] {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![attachment.clone()])
            .with_battlefield(controller, vec![host.clone()])
            .build();
        assert_eq!(
            matches(&view, &attachment, &Filter::AttachedTo(&INNER)),
            Some(controller == 1),
            "the host changes sides; the attachment and the viewer do not"
        );
    }
}

#[test]
fn unrelated_host_is_not_a_match_even_when_other_objects_match() {
    let (mut attachment, host) = attached_pair(TypeSet::ARTIFACT);
    attachment.types = TypeSet::CREATURE;
    let mut decoy = printed(3, 1, "Unattached host fixture", 3);
    decoy.types = TypeSet::LAND;
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![attachment.clone()])
        .with_battlefield(1, vec![host, decoy])
        .build();
    assert_eq!(
        matches(&view, &attachment, &ATTACHED_TO_CREATURE_OR_LAND),
        Some(false),
        "only the attached host counts, not the candidate or another permanent"
    );
    assert_eq!(
        matches(
            &view,
            &attachment,
            &Filter::Not(&ATTACHED_TO_CREATURE_OR_LAND)
        ),
        Some(true)
    );
}

#[test]
fn an_unattached_object_does_not_match_even_an_unknown_inner_filter() {
    static UNKNOWN: Filter = Filter::AttachedTo(&Filter::HasKeyword(KeywordSet::FLYING));
    let (mut attachment, host) = attached_pair(TypeSet::CREATURE);
    attachment.attached_to = None;
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![attachment.clone()])
        .with_battlefield(1, vec![host])
        .build();
    for filter in [&ATTACHED_TO_CREATURE_OR_LAND, &UNKNOWN] {
        assert_eq!(matches(&view, &attachment, filter), Some(false));
    }
    assert_eq!(
        matches(
            &view,
            &attachment,
            &Filter::Not(&ATTACHED_TO_CREATURE_OR_LAND)
        ),
        Some(true)
    );
}

#[test]
fn a_missing_host_remains_unknown_under_negation() {
    let (attachment, mut other_incarnation) = attached_pair(TypeSet::CREATURE);
    other_incarnation.id = ObjectId::new(2, 1);
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![attachment.clone()])
        .with_battlefield(1, vec![other_incarnation])
        .build();
    assert!(view.object(attachment.attached_to.unwrap()).is_none());
    assert_eq!(
        matches(&view, &attachment, &ATTACHED_TO_CREATURE_OR_LAND),
        None
    );
    assert_eq!(
        matches(
            &view,
            &attachment,
            &Filter::Not(&ATTACHED_TO_CREATURE_OR_LAND)
        ),
        None,
        "an unresolved attachment handle is not proof of a nonmatch"
    );
    assert_eq!(
        matches(&view, &attachment, &Filter::AttachedTo(&Filter::Any)),
        None
    );
}

#[test]
fn a_hidden_host_remains_unknown_even_when_its_fields_appear_to_match() {
    for host_types in [TypeSet::CREATURE, TypeSet::LAND] {
        let (attachment, mut host) = attached_pair(host_types);
        host.card = None;
        host.rules = None;
        host.token = None;
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![attachment.clone()])
            .with_battlefield(1, vec![host])
            .build();
        assert_eq!(
            matches(&view, &attachment, &ATTACHED_TO_CREATURE_OR_LAND),
            None
        );
        assert_eq!(
            matches(
                &view,
                &attachment,
                &Filter::Not(&ATTACHED_TO_CREATURE_OR_LAND)
            ),
            None,
            "negation must preserve the hidden host's unknown result"
        );
        assert_eq!(
            matches(&view, &attachment, &Filter::AttachedTo(&Filter::Any)),
            None
        );
    }
}

#[test]
fn an_unknown_inner_filter_on_a_visible_host_remains_unknown() {
    static UNKNOWN: Filter = Filter::AttachedTo(&Filter::HasKeyword(KeywordSet::FLYING));
    let (attachment, host) = attached_pair(TypeSet::CREATURE);
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![attachment.clone()])
        .with_battlefield(1, vec![host])
        .build();
    assert_eq!(matches(&view, &attachment, &UNKNOWN), None);
    assert_eq!(matches(&view, &attachment, &Filter::Not(&UNKNOWN)), None);
}

#[test]
fn census_reads_attachment_filters_independently_of_the_probes_attachment() {
    for (case, attached_to, hidden) in [
        ("unattached", None, false),
        ("visible host", Some(ObjectId::new(2, 0)), false),
        ("missing host", Some(ObjectId::new(99, 0)), false),
        ("hidden host", Some(ObjectId::new(2, 0)), true),
    ] {
        let (mut probe, mut host) = attached_pair(TypeSet::CREATURE);
        probe.attached_to = attached_to;
        if hidden {
            host.card = None;
            host.rules = None;
            host.token = None;
        }
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![probe.clone()])
            .with_battlefield(1, vec![host])
            .build();
        assert!(
            unreadable_leaf(&view, &probe, &ATTACHED_TO_CREATURE_OR_LAND).is_none(),
            "{case}: readability depends on the filter tree, not this probe's host"
        );
    }
}

#[test]
fn census_finds_unknown_nested_filters_behind_an_unattached_probe() {
    static BLIND: Filter = Filter::AttachedTo(&Filter::And(&[
        Filter::LAND,
        Filter::HasKeyword(KeywordSet::FLYING),
    ]));
    static NEGATED: Filter = Filter::Not(&BLIND);
    let mut probe = printed(1, 0, "Unattached probe fixture", 1);
    probe.types = TypeSet::CREATURE;
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![probe.clone()])
        .build();
    assert_eq!(matches(&view, &probe, &BLIND), Some(false));
    assert_eq!(matches(&view, &probe, &NEGATED), Some(true));
    for filter in [&BLIND, &NEGATED] {
        assert!(
            matches!(
                unreadable_leaf(&view, &probe, filter),
                Some(Filter::HasKeyword(_))
            ),
            "the census must descend past both the absent attachment and the false type leaf"
        );
    }
}
