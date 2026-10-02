//! Persistent labels for what a permanent named on entering: a creature type
//! (Cavern of Souls), or a card name (Pithing Needle); and for the doors of a
//! Room still locked (CR 709.5).
use super::{UiFonts, glyph, icon_tf, palette, tf};
use crate::{Duel, settings::ClientSettings, table};
use baylee_client_core::{Lang, i18n::Phrase, type_names};
use baylee_core::{generated::subtypes, ids::ObjectId};
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct ChosenTypeLabel(ObjectId);

#[derive(Component)]
pub(crate) struct ChargeIcon;

fn words(object: &baylee_view::PublicObject, lang: Lang, detail: bool) -> Option<String> {
    let mut parts: Vec<String> = choice_words(object, lang).into_iter().collect();
    for counter in &object.counters {
        if counter.kind == baylee_view::CounterKind::Charge && counter.count > 0 {
            let noun = Phrase::counted(
                usize::from(counter.count),
                Phrase::LogCounterCharge,
                Phrase::LogCountersCharge,
            );
            parts.push(if detail {
                format!("{} {}", counter.count, noun.text(lang))
            } else {
                format!("{} ", counter.count)
            });
        }
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

fn choice_words(object: &baylee_view::PublicObject, lang: Lang) -> Option<String> {
    if let Some(doors) = object.unlocked_doors {
        // The locked halves by name, in the pool's English as a named card
        // is; nothing once every door is open.
        let def = baylee_cards::by_index(object.card?.index)?;
        let locked: Vec<&str> = def
            .faces
            .iter()
            .zip(doors)
            .filter(|(_, open)| !open)
            .map(|(face, _)| face.name)
            .collect();
        return (!locked.is_empty()).then(|| Phrase::LockedDoors.fill(lang, &[&locked.join(", ")]));
    }
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
    Some(name.to_string())
}

/// Follow the actual card pose without taking its pointer events. Only cards
/// carrying a choice or charge counters get a label.
pub(crate) fn sync(
    mut commands: Commands,
    duel: Res<Duel>,
    (settings, fonts): (Res<ClientSettings>, Res<UiFonts>),
    shown: Res<table::ShownRig>,
    (windows, cards): (Query<&Window>, Query<(&table::CardVisual, &Transform)>),
    mut labels: Query<(
        Entity,
        &ChosenTypeLabel,
        &mut Text,
        &mut Node,
        &mut Visibility,
        &Children,
    )>,
    mut icons: Query<&mut TextSpan, With<ChargeIcon>>,
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
        .filter_map(|object| {
            let detail = duel.hovered == Some(object.id);
            let icon = !detail
                && object.counters.iter().any(|counter| {
                    counter.kind == baylee_view::CounterKind::Charge && counter.count > 0
                });
            words(object, lang, detail).map(|label| (object.id, label, icon))
        })
        .collect();
    for (entity, mark, _, _, _, _) in &mut labels {
        if !wanted.iter().any(|(id, _, _)| *id == mark.0) {
            commands.entity(entity).despawn();
        }
    }
    let boxes: Vec<_> = lens
        .into_iter()
        .flat_map(|lens| {
            cards.iter().filter_map(move |(card, pose)| {
                card_rect(&lens, pose).map(|rect| (card.object, rect))
            })
        })
        .collect();
    for (id, label, charge) in wanted {
        let at = boxes
            .iter()
            .find(|(object, _)| *object == id)
            .map(|(_, rect)| {
                // Conservative first-frame width; every letter is at most one em.
                let width = label.chars().count() as f32 * 13.0 * super::UI_SCALE
                    + if charge { 16.0 } else { 0.0 }
                    + 10.0;
                label_anchor(*rect, Vec2::new(width, 22.0), &boxes)
            });
        let visible = if at.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some((_, _, mut text, mut node, mut visibility, children)) =
            labels.iter_mut().find(|(_, mark, _, _, _, _)| mark.0 == id)
        {
            if text.0 != label {
                text.0 = label;
            }
            for child in children {
                if let Ok(mut icon) = icons.get_mut(*child) {
                    let wanted = if charge {
                        glyph::CHARGE.to_string()
                    } else {
                        String::new()
                    };
                    if icon.0 != wanted {
                        icon.0 = wanted;
                    }
                }
            }
            if *visibility != visible {
                *visibility = visible;
            }
            if let Some(at) = at {
                place(&mut node, at);
            }
        } else {
            spawn_label(&mut commands, &fonts, id, label, charge, at);
        }
    }
}

fn spawn_label(
    commands: &mut Commands,
    fonts: &UiFonts,
    id: ObjectId,
    label: String,
    charge: bool,
    at: Option<Vec2>,
) {
    let visible = if at.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
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
        TextLayout::no_wrap(),
        children![(
            ChargeIcon,
            TextSpan::new(if charge {
                glyph::CHARGE.to_string()
            } else {
                String::new()
            }),
            icon_tf(fonts, 12.0),
            TextColor(palette::CANDLE),
            Pickable::IGNORE,
        )],
        tf(fonts, 13.0),
        TextColor(palette::CANDLE),
        BackgroundColor(palette::DIALOG),
        Pickable::IGNORE,
        GlobalZIndex(-1),
        visible,
    ));
}

fn card_rect(lens: &table::Lens, pose: &Transform) -> Option<Rect> {
    let w = baylee_client_core::layout::CARD_WIDTH * 0.5;
    let h = baylee_client_core::layout::CARD_HEIGHT * 0.5;
    let mut rect = Rect {
        min: Vec2::splat(f32::INFINITY),
        max: Vec2::splat(f32::NEG_INFINITY),
    };
    for (x, y) in [(-w, -h), (w, -h), (-w, h), (w, h)] {
        let p = lens.project_world(pose.transform_point(Vec3::new(x, y, 0.0)))?;
        rect.min = rect.min.min(p);
        rect.max = rect.max.max(p);
    }
    Some(rect)
}

fn label_anchor(card: Rect, size: Vec2, cards: &[(ObjectId, Rect)]) -> Vec2 {
    // Prefer the same row, so a label cannot appear to belong to a land
    // below the permanent whose counters it describes.
    for x in [card.max.x + 4.0, card.min.x - size.x - 4.0] {
        let at = Vec2::new(x, card.center().y - size.y * 0.5);
        let label = Rect::from_corners(at, at + size);
        if x >= 0.0 && cards.iter().all(|(_, r)| !overlaps(label, *r)) {
            return at;
        }
    }
    let mut at = Vec2::new(card.center().x - size.x * 0.5, card.max.y + 3.0);
    // Move down only when the label would cover another printing. Each
    // collision passes at least one card bottom, so this is bounded.
    for _ in 0..cards.len() {
        let label = Rect::from_corners(at, at + size);
        let bottom = cards
            .iter()
            .filter(|(_, r)| overlaps(label, *r))
            .map(|(_, r)| r.max.y)
            .max_by(f32::total_cmp);
        let Some(bottom) = bottom else { break };
        at.y = bottom + 3.0;
    }
    at
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.min.x < b.max.x && a.max.x > b.min.x && a.min.y < b.max.y && a.max.y > b.min.y
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
    fn counter_label_stays_clear_of_the_next_rows_print() {
        let card = Rect::from_corners(Vec2::new(100.0, 100.0), Vec2::new(140.0, 160.0));
        let below = Rect::from_corners(Vec2::new(70.0, 170.0), Vec2::new(110.0, 230.0));
        let at = label_anchor(
            card,
            Vec2::new(130.0, 22.0),
            &[(ObjectId::new(2, 0), below)],
        );
        assert_eq!(at, Vec2::new(144.0, 119.0));
        assert_eq!(
            label_anchor(card, Vec2::new(10.0, 22.0), &[]),
            Vec2::new(144.0, 119.0)
        );
    }

    #[test]
    fn charge_counters_are_visible_and_localized_without_a_chosen_type() {
        let mut object = crate::registry_printed(1, 0, "Inspirit, Flagship Vessel");
        object.counters.push(baylee_view::CounterEntry {
            kind: baylee_view::CounterKind::Charge,
            count: 4,
        });
        assert_eq!(words(&object, Lang::De, false).as_deref(), Some("4 "));
        assert_eq!(words(&object, Lang::En, false).as_deref(), Some("4 "));
        assert_eq!(
            words(&object, Lang::De, true).as_deref(),
            Some("4 Ladungsmarken")
        );
        assert_eq!(
            words(&object, Lang::En, true).as_deref(),
            Some("4 charge counters")
        );
        object.counters[0].count = 1;
        assert_eq!(words(&object, Lang::De, false).as_deref(), Some("1 "));
        object.counters.clear();
        assert_eq!(words(&object, Lang::De, false), None);
    }

    #[test]
    fn chosen_type_label_is_localized_and_does_not_invent_a_choice() {
        let mut object = crate::registry_printed(1, 0, "Reflections of Littjara");
        assert_eq!(words(&object, Lang::De, false), None);
        object.chosen_subtype = Some(subtypes::creature::ALLY);
        assert_eq!(
            words(&object, Lang::De, false).as_deref(),
            Some("Verbündeter")
        );
        assert_eq!(words(&object, Lang::En, false).as_deref(), Some("Ally"));
    }

    #[test]
    fn a_needle_s_label_names_the_card_it_was_given() {
        let mut object = crate::registry_printed(1, 0, "Pithing Needle");
        assert_eq!(
            words(&object, Lang::En, false),
            None,
            "no name before one is chosen"
        );
        object.chosen_name = Some(baylee_view::NamedFace {
            card: baylee_cards::decks::by_name("Malakir Rebirth").expect("in the pool"),
            face: 1,
        });
        assert_eq!(
            words(&object, Lang::En, false).as_deref(),
            Some("Named: Malakir Mire"),
            "the face that was named, the back one here"
        );
        assert_eq!(
            words(&object, Lang::De, false).as_deref(),
            Some("Genannt: Malakir Mire")
        );
    }
    #[test]
    fn a_rooms_label_names_the_doors_still_locked() {
        let mut object = crate::registry_printed(1, 0, "Walk-In Closet");
        assert_eq!(
            words(&object, Lang::En, false),
            None,
            "not a Room permanent yet"
        );
        object.unlocked_doors = Some([true, false]);
        assert_eq!(
            words(&object, Lang::En, false).as_deref(),
            Some("Locked: Forgotten Cellar")
        );
        assert_eq!(
            words(&object, Lang::De, false).as_deref(),
            Some("Verschlossen: Forgotten Cellar")
        );
        object.unlocked_doors = Some([false, false]);
        assert_eq!(
            words(&object, Lang::En, false).as_deref(),
            Some("Locked: Walk-In Closet, Forgotten Cellar"),
            "entered uncast, both doors shut"
        );
        object.unlocked_doors = Some([true, true]);
        assert_eq!(words(&object, Lang::En, false), None, "every door open");
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
        assert_eq!(text, "Ally");
        app.world_mut().resource_mut::<ClientSettings>().lang = "de".into();
        app.update();
        assert_eq!(app.world().get::<Text>(entity).unwrap().0, "Verbündeter");
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
