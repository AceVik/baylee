use super::*;
use baylee_client_core::test_support::{ViewBuilder, token};
use baylee_core::ids::{DamageSourceRef, PlayerId, SourceChoiceId};
use baylee_engine::choice::Pending;

#[test]
fn opening_a_source_choice_clears_the_previous_card_preview() {
    let source = DamageSourceRef {
        object: ObjectId::new(9, 0),
        version: 1,
    };
    let pending = Pending::ChooseDamageSource {
        player: PlayerId::new(0),
        choice: SourceChoiceId::new(3),
        options: vec![source],
    };
    let mut duel = crate::Duel {
        view: Some(ViewBuilder::new(2).build()),
        hovered: Some(source.object),
        hovered_at: Some(crate::HoverSpot::Point(bevy::prelude::Vec2::ZERO)),
        ..Default::default()
    };
    duel.receive_choice(pending.clone());
    assert_eq!(duel.hovered, None);
    assert!(duel.hovered_at.is_none());
    // A re-sent identical question does not steal a newly requested preview.
    duel.hovered = Some(source.object);
    duel.receive_choice(pending);
    assert_eq!(duel.hovered, Some(source.object));
}

#[test]
fn historical_rows_stay_distinct_and_never_preview_the_current_object() {
    let mut view = ViewBuilder::new(2)
        .with_battlefield(1, vec![token(9, 1, "New secret identity", 8, 8)])
        .build();
    let old = baylee_view::DamageSourceView {
        source: DamageSourceRef {
            object: ObjectId::new(9, 0),
            version: 2,
        },
        name: "Orcish Artillery".into(),
        card: None,
        rules: None,
        token: None,
        controller: PlayerId::new(1),
        zone: baylee_view::LogZone::Battlefield,
        is_current: false,
        referenced_by: vec![ObjectId::new(10, 0)],
        colors: baylee_core::color::ColorSet::default(),
        types: baylee_core::types::TypeSet::default(),
        power: Some(1),
        toughness: Some(3),
        keywords: 0,
    };
    let older = baylee_view::DamageSourceView {
        source: DamageSourceRef {
            version: 1,
            ..old.source
        },
        ..old.clone()
    };
    let refs = vec![old.source, older.source];
    let mut old = old;
    old.referenced_by = vec![ObjectId::new(10, 0)];
    let mut older = older;
    older.referenced_by = vec![ObjectId::new(11, 0)];
    let mut first = token(10, 1, "Ability", 0, 0);
    first.targets = vec![baylee_view::TargetRef::Player(PlayerId::new(0))];
    let mut second = token(11, 1, "Ability", 0, 0);
    second.targets = vec![baylee_client_core::test_support::target(ObjectId::new(
        12, 0,
    ))];
    view.battlefield.push(token(12, 0, "Grizzly Bears", 2, 2));
    view.stack = vec![first, second];
    baylee_client_core::test_support::project_current_targets(&mut view);
    view.damage_sources = vec![old, older];
    let pending = Pending::ChooseDamageSource {
        player: view.seat,
        choice: SourceChoiceId::new(3),
        options: refs,
    };
    let i = Interaction::new(pending, view.seat);
    let rows = options(
        &i.prompt(),
        Lang::De,
        None,
        "",
        FaceNames {
            view: Some(&view),
            texts: None,
        },
    )
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows[0].label.starts_with("1. Orcish Artillery"));
    assert!(
        rows[0].label.contains("früheres Objekt 2") && rows[1].label.contains("früheres Objekt 1")
    );
    assert!(rows.iter().all(|r| !r.label.contains("secret")));
    assert!(rows[0].label.contains(&format!(
        "Stapel → {}",
        seat_name(Lang::De, None, PlayerId::new(0))
    )));
    assert!(rows[1].label.contains("Stapel → Grizzly Bears"));
    assert!(!rows[0].label.contains("Grizzly Bears"));
    let duel = crate::Duel {
        view: Some(view),
        interaction: Some(i),
        ..Default::default()
    };
    assert_eq!(preview_object(&duel, 0), None);
    assert_eq!(preview_object(&duel, 1), None);
}
