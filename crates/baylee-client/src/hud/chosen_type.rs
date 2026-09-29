//! Persistent labels for what a permanent named on entering: a creature type
//! (Cavern of Souls), or a card name (Pithing Needle).
use super::{UiFonts, palette, tf};
use crate::{Duel, settings::ClientSettings, table};
use baylee_client_core::{Lang, i18n::Phrase, type_names};
use baylee_core::{generated::subtypes, ids::ObjectId};
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct ChosenTypeLabel(ObjectId);

fn words(object: &baylee_view::PublicObject, lang: Lang) -> Option<String> {
    if let Some(named) = object.chosen_name {
        // The pool's English: the name is the rules identity, and the card
        // it names may be one this seat has never been shown a printing of.
        let face = baylee_cards::by_index(named.card)?
            .faces
            .get(usize::from(named.face))?;
        return Some(Phrase::ChosenName.fill(lang, &[face.name]));
    }
    let subtype = object.chosen_subtype?;
    let english = subtypes::name(subtype)?;
    let name = type_names::name(english, lang);
    Some(Phrase::ChosenType.fill(lang, &[name]))
}

/// Follow the actual card pose without taking its pointer events. Only cards
/// carrying a choice get a label.
pub(crate) fn sync(
    mut commands: Commands,
    duel: Res<Duel>,
    (settings, fonts): (Res<ClientSettings>, Res<UiFonts>),
    shown: Res<table::ShownRig>,
    windows: Query<&Window>,
    cards: Query<(&table::CardVisual, &Transform)>,
    mut labels: Query<(
        Entity,
        &ChosenTypeLabel,
        &mut Text,
        &mut Node,
        &mut Visibility,
    )>,
) {
    let lang = Lang::of(&settings.lang);
    let lens = shown.rig().and_then(|rig| {
        windows
            .single()
            .ok()
            .map(|window| table::Lens::new(rig, Vec2::new(window.width(), window.height())))
    });
    let wanted: Vec<_> = duel
        .view
        .as_ref()
        .into_iter()
        .flat_map(|view| &view.battlefield)
        .filter_map(|object| words(object, lang).map(|label| (object.id, label)))
        .collect();
    for (entity, mark, _, _, _) in &mut labels {
        if !wanted.iter().any(|(id, _)| *id == mark.0) {
            commands.entity(entity).despawn();
        }
    }
    for (id, label) in wanted {
        let at = lens.and_then(|lens| {
            cards
                .iter()
                .find(|(card, _)| card.object == id)
                .and_then(|(_, pose)| {
                    lens.project_world(pose.transform_point(Vec3::new(
                        0.0,
                        -baylee_client_core::layout::CARD_HEIGHT * 0.38,
                        0.0,
                    )))
                })
        });
        let visible = if at.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some((_, _, mut text, mut node, mut visibility)) =
            labels.iter_mut().find(|(_, mark, _, _, _)| mark.0 == id)
        {
            if text.0 != label {
                text.0 = label;
            }
            if *visibility != visible {
                *visibility = visible;
            }
            if let Some(at) = at {
                place(&mut node, at);
            }
        } else {
            let mut node = Node {
                position_type: PositionType::Absolute,
                padding: UiRect::axes(px(5), px(2)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            };
            if let Some(at) = at {
                place(&mut node, at);
            }
            commands.spawn((
                ChosenTypeLabel(id),
                super::DetachedHud,
                node,
                Text::new(label),
                tf(&fonts, 13.0),
                TextColor(palette::CANDLE),
                BackgroundColor(palette::DIALOG),
                UiTransform::from_translation(Val2::percent(-50.0, -50.0)),
                Pickable::IGNORE,
                GlobalZIndex(-1),
                visible,
            ));
        }
    }
}

fn place(node: &mut Node, at: Vec2) {
    let (left, top) = (px(at.x), px(at.y));
    if node.left != left {
        node.left = left;
    }
    if node.top != top {
        node.top = top;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chosen_type_label_is_localized_and_does_not_invent_a_choice() {
        let mut object = crate::registry_printed(1, 0, "Reflections of Littjara");
        assert_eq!(words(&object, Lang::De), None);
        object.chosen_subtype = Some(subtypes::creature::ALLY);
        assert_eq!(
            words(&object, Lang::De).as_deref(),
            Some("Gewählt: Verbündeter")
        );
        assert_eq!(words(&object, Lang::En).as_deref(), Some("Chosen: Ally"));
    }

    #[test]
    fn a_needle_s_label_names_the_card_it_was_given() {
        let mut object = crate::registry_printed(1, 0, "Pithing Needle");
        assert_eq!(
            words(&object, Lang::En),
            None,
            "no name before one is chosen"
        );
        object.chosen_name = Some(baylee_view::NamedFace {
            card: baylee_cards::decks::by_name("Malakir Rebirth").expect("in the pool"),
            face: 1,
        });
        assert_eq!(
            words(&object, Lang::En).as_deref(),
            Some("Named: Malakir Mire"),
            "the face that was named, the back one here"
        );
        assert_eq!(
            words(&object, Lang::De).as_deref(),
            Some("Genannt: Malakir Mire")
        );
    }
    #[test]
    fn persistent_label_updates_its_language_and_leaves_with_its_permanent() {
        let mut app = App::new();
        let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
        let mut object = crate::registry_printed(1, 0, "Reflections of Littjara");
        object.chosen_subtype = Some(subtypes::creature::ALLY);
        view.battlefield.push(object);
        let fonts = UiFonts {
            text: default(),
            medium: default(),
            bold: default(),
            italic: default(),
            medium_italic: default(),
            serif: default(),
            serif_italic: default(),
            icons: default(),
            mana: default(),
        };
        app.insert_resource(Duel {
            view: Some(view),
            ..default()
        })
        .insert_resource(ClientSettings::default())
        .insert_resource(fonts)
        .init_resource::<table::ShownRig>()
        .add_systems(Update, sync);
        app.update();
        let (entity, text) = app
            .world_mut()
            .query_filtered::<(Entity, &Text), With<ChosenTypeLabel>>()
            .single(app.world())
            .map(|(entity, text)| (entity, text.0.clone()))
            .unwrap();
        assert_eq!(text, "Chosen: Ally");
        app.world_mut().resource_mut::<ClientSettings>().lang = "de".into();
        app.update();
        assert_eq!(
            app.world().get::<Text>(entity).unwrap().0,
            "Gewählt: Verbündeter"
        );
        app.world_mut()
            .resource_mut::<Duel>()
            .view
            .as_mut()
            .unwrap()
            .battlefield
            .clear();
        app.update();
        assert!(app.world().get_entity(entity).is_err());
    }
}
