//! Persistent labels for what a permanent named on entering: a creature type
//! (Cavern of Souls), or a card name (Pithing Needle); for the doors of a
//! Room still locked (CR 709.5); for a leveler's level (report 30), and for a
//! permanent that has lost all its abilities (report 21).
use super::{UiFonts, glyph, icon_tf, palette, tf};
use crate::{Duel, settings::ClientSettings, table};
use baylee_client_core::{
    Lang,
    annotations::{self, Bounds},
    i18n::Phrase,
    type_names,
};
use baylee_core::{generated::subtypes, ids::ObjectId};
use bevy::prelude::*;

/// Age counter (`baylee_cards_dsl::counters::AGE`) projected onto `baylee_view`.
const AGE_COUNTER: baylee_view::CounterKind = baylee_view::CounterKind::Custom(10);

#[derive(Component)]
pub(crate) struct ChosenTypeLabel {
    object: ObjectId,
    measured: [(String, Option<char>, Vec2); 2],
}

#[derive(Component)]
pub(crate) struct ChargeIcon;

fn counter_icon(object: &baylee_view::PublicObject) -> Option<char> {
    for counter in &object.counters {
        if counter.count == 0 {
            continue;
        }
        match counter.kind {
            baylee_view::CounterKind::Charge => return Some(glyph::CHARGE),
            AGE_COUNTER => return Some(glyph::HOURGLASS),
            _ => {}
        }
    }
    None
}

fn words(object: &baylee_view::PublicObject, lang: Lang, detail: bool) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    // Lost every ability (report 21: Oko's Elk): its printed text, which
    // the preview still shows, no longer applies.
    if object.status.has_no_abilities() {
        parts.push(Phrase::NoAbilities.text(lang).to_string());
    }
    parts.extend(choice_words(object, lang));
    if object.types.contains(baylee_core::types::TypeSet::DUNGEON)
        && let Some((dungeon, room_idx)) =
            baylee_core::dungeon::Dungeon::from_public_name(&object.name)
    {
        let total = dungeon.rooms().len();
        let room_name = dungeon.rooms()[room_idx];
        parts.push(if detail {
            format!(
                "{}/{} {room_name} — {}",
                room_idx + 1,
                total,
                dungeon.room_effects()[room_idx]
            )
        } else {
            format!("{}/{} {room_name}", room_idx + 1, total)
        });
    }
    if object.types.contains(baylee_core::types::TypeSet::BATTLE) {
        parts.push(
            Phrase::BattleDefense.fill(
                lang,
                &[&object
                    .counter_count(baylee_view::CounterKind::Defense)
                    .to_string()],
            ),
        );
    }
    // A leveler's level (report 30: Hexdrinker), which nothing on the card
    // says; the plate shows the body it bought.
    for counter in &object.counters {
        if counter.kind == baylee_view::CounterKind::Level && counter.count > 0 {
            let level = if detail {
                Phrase::LevelLong
            } else {
                Phrase::LevelShort
            };
            parts.push(level.fill(lang, &[&counter.count.to_string()]));
        }
    }
    for counter in &object.counters {
        if counter.count == 0 {
            continue;
        }
        let noun = match counter.kind {
            baylee_view::CounterKind::Charge => Phrase::LogCountersCharge,
            AGE_COUNTER => Phrase::LogCountersAge,
            _ => continue,
        };
        parts.push(if detail {
            let noun = noun.fill_count(lang, usize::from(counter.count), &[]);
            format!("{} {noun}", counter.count)
        } else {
            format!("{} ", counter.count)
        });
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

type Geometry<'w, 's> = (
    Query<'w, 's, &'static Window>,
    Option<Res<'w, UiScale>>,
    Query<'w, 's, (&'static table::CardVisual, &'static Transform)>,
    Query<
        'w,
        's,
        (&'static ComputedNode, &'static UiGlobalTransform),
        With<super::overlay::PreviewBounds>,
    >,
);

/// Follow the actual card pose without taking its pointer events. Only cards
/// carrying a choice, a level, charge/age counters or no abilities get a label.
#[allow(clippy::too_many_lines)]
pub(crate) fn sync(
    mut commands: Commands,
    duel: Res<Duel>,
    (settings, fonts, mode): (
        Res<ClientSettings>,
        Res<UiFonts>,
        Option<Res<crate::face::FaceMode>>,
    ),
    shown: Res<table::ShownRig>,
    (windows, ui, cards, previews): Geometry<'_, '_>,
    mut labels: Query<(
        Entity,
        &mut ChosenTypeLabel,
        &mut Text,
        &mut Node,
        &mut Visibility,
        &Children,
        &ComputedNode,
    )>,
    mut icons: Query<&mut TextSpan, With<ChargeIcon>>,
) {
    let lang = Lang::of(&settings.lang);
    let detail = mode.as_ref().is_some_and(|m| m.held);
    let lens = shown.rig().and_then(|rig| {
        windows
            .single()
            .ok()
            .map(|window| table::Lens::new(rig, crate::hud::scale::space(window, ui.as_deref())))
    });
    let wanted = wanted_labels(&duel, lang, detail);
    for (entity, mark, _, _, _, _, _) in &mut labels {
        if !wanted.iter().any(|(id, _, _)| *id == mark.object) {
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
    let mut occupied: Vec<_> = boxes.iter().map(|(_, r)| bounds(*r)).collect();
    occupied.extend(previews.iter().map(|(node, pose)| {
        let scale = node.inverse_scale_factor;
        bounds(Rect::from_center_size(
            pose.translation * scale,
            node.size() * scale,
        ))
    }));
    let window = windows
        .single()
        .ok()
        .map(|w| crate::hud::scale::space(w, ui.as_deref()));
    let slot = usize::from(detail);
    for (id, label, icon_glyph) in wanted {
        let mut size = None;
        for (_, mut mark, text, _, _, children, node) in &mut labels {
            if mark.object != id {
                continue;
            }
            let current_icon = children
                .iter()
                .find_map(|child| icons.get(child).ok())
                .and_then(|span| span.0.chars().next());
            if text.0 == label && current_icon == icon_glyph && node.size().x > 0.0 {
                let actual = node.size() * node.inverse_scale_factor;
                if mark.measured[slot].0 != label
                    || mark.measured[slot].1 != icon_glyph
                    || mark.measured[slot].2 != actual
                {
                    mark.measured[slot] = (label.clone(), icon_glyph, actual);
                }
                size = Some(actual);
            } else if mark.measured[slot].0 == label
                && mark.measured[slot].1 == icon_glyph
                && mark.measured[slot].2.x > 0.0
            {
                size = Some(mark.measured[slot].2);
            }
            break;
        }
        let size = size.unwrap_or_else(|| estimated_size(&label, icon_glyph.is_some()));
        let at = boxes
            .iter()
            .find(|(object, _)| *object == id)
            .and_then(|(_, rect)| {
                window
                    .and_then(|window| annotations::anchor(bounds(*rect), size, window, &occupied))
            });
        if let Some(at) = at {
            occupied.push(Bounds {
                min: at,
                max: at + size,
            });
        }
        let visible = if at.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some((_, _, mut text, mut node, mut visibility, children, _)) = labels
            .iter_mut()
            .find(|(_, mark, _, _, _, _, _)| mark.object == id)
        {
            if text.0 != label {
                text.0 = label;
            }
            for child in children {
                if let Ok(mut icon) = icons.get_mut(*child) {
                    let wanted = icon_glyph.map_or_else(String::new, |c| c.to_string());
                    if icon.0 != wanted {
                        icon.0 = wanted;
                    }
                }
            }
            if *visibility != visible {
                *visibility = visible;
            }
            // Compared here, not only inside `place`: handing it `&mut node`
            // dereferences the `Mut` and marks the node changed, and a
            // changed `Node` lays the whole interface out again, every frame.
            if let Some(at) = at
                && (node.left != px(at.x) || node.top != px(at.y))
            {
                place(&mut node, at);
            }
        } else {
            spawn_label(&mut commands, &fonts, id, label, icon_glyph, at);
        }
    }
}

fn estimated_size(label: &str, has_icon: bool) -> Vec2 {
    Vec2::new(
        label.chars().count() as f32 * 7.2 * super::UI_SCALE
            + if has_icon {
                14.0 * super::UI_SCALE
            } else {
                0.0
            }
            + 10.0,
        21.0 * super::UI_SCALE,
    )
}

fn wanted_labels(duel: &Duel, lang: Lang, detail: bool) -> Vec<(ObjectId, String, Option<char>)> {
    let Some(view) = duel.view.as_ref() else {
        return Vec::new();
    };
    view.battlefield
        .iter()
        .chain(
            view.command
                .iter()
                .flatten()
                .filter(|o| o.types.contains(baylee_core::types::TypeSet::DUNGEON)),
        )
        .filter_map(|object| {
            let icon = (!detail).then(|| counter_icon(object)).flatten();
            let mut parts: Vec<_> = words(object, lang, detail).into_iter().collect();
            if let Some(protector) = object.battle_protector {
                let called = super::seatbar::called(
                    lang,
                    view,
                    duel.statics.as_ref(),
                    protector,
                    super::seatbar::role_of(duel, protector),
                );
                parts.push(Phrase::BattleProtectedBy.fill(lang, &[&called]));
            }
            if let Some(opponent) = object.chosen_opponent {
                parts.insert(
                    0,
                    super::seatbar::called(
                        lang,
                        view,
                        duel.statics.as_ref(),
                        opponent,
                        super::seatbar::role_of(duel, opponent),
                    ),
                );
            }
            (!parts.is_empty()).then(|| (object.id, parts.join(" · "), icon))
        })
        .collect()
}

fn spawn_label(
    commands: &mut Commands,
    fonts: &UiFonts,
    id: ObjectId,
    label: String,
    icon_glyph: Option<char>,
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
        ChosenTypeLabel {
            object: id,
            measured: [
                (String::new(), None, Vec2::ZERO),
                (String::new(), None, Vec2::ZERO),
            ],
        },
        super::DetachedHud,
        node,
        Text::new(label),
        TextLayout::no_wrap(),
        children![(
            ChargeIcon,
            TextSpan::new(icon_glyph.map_or_else(String::new, |c| c.to_string())),
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

fn bounds(rect: Rect) -> Bounds {
    Bounds {
        min: rect.min,
        max: rect.max,
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
    fn battle_label_distinguishes_defense_and_protector_from_controller() {
        use baylee_client_core::test_support::{ViewBuilder, token};
        let mut object = token(1, 0, "Battle", 0, 0);
        object.types = baylee_core::types::TypeSet::BATTLE;
        object.power = None;
        object.toughness = None;
        object.battle_protector = Some(baylee_core::ids::PlayerId::new(1));
        object.counters = vec![baylee_view::CounterEntry {
            kind: baylee_view::CounterKind::Defense,
            count: 4,
        }];
        let mut duel = Duel::default();
        duel.receive_view(ViewBuilder::new(2).with_battlefield(0, [object]).build());
        let english = wanted_labels(&duel, Lang::En, false);
        assert!(english[0].1.contains("Defense 4"));
        assert!(english[0].1.contains("Protected by"));
        assert!(english[0].1.contains('1'));
        let german = wanted_labels(&duel, Lang::De, false);
        assert!(german[0].1.contains("Verteidigung 4"));
        assert!(german[0].1.contains("Beschützt von"));
    }

    #[test]
    fn the_chosen_opponent_label_uses_the_actual_seat_name() {
        use baylee_client_core::test_support::ViewBuilder;
        let mut object = crate::registry_printed(1, 0, "Black Vise");
        object.chosen_opponent = Some(baylee_core::ids::PlayerId::new(1));
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![object])
            .build();
        let mut duel = Duel::default();
        duel.receive_view(view);
        let labels = wanted_labels(&duel, Lang::En, false);
        assert_eq!(labels.len(), 1);
        assert!(
            labels[0].1.contains('1'),
            "fallback still identifies the chosen seat"
        );
        // The named table, as sent with a real game, replaces the numbered fallback.
        let mut statics = baylee_client_core::test_support::statics(1);
        let mut opponent = statics.seats[0].clone();
        opponent.player = baylee_core::ids::PlayerId::new(1);
        opponent.display_name = "Mira".into();
        statics.seats.push(opponent);
        duel.statics = Some(statics);
        assert_eq!(wanted_labels(&duel, Lang::En, false)[0].1, "Mira");
    }

    #[test]
    fn charge_and_age_counters_are_visible_and_localized_with_alt_detail() {
        let mut object = crate::registry_printed(1, 0, "Inspirit, Flagship Vessel");
        object.counters.push(baylee_view::CounterEntry {
            kind: baylee_view::CounterKind::Charge,
            count: 4,
        });
        assert_eq!(words(&object, Lang::De, false).as_deref(), Some("4 "));
        assert_eq!(words(&object, Lang::En, false).as_deref(), Some("4 "));
        assert_eq!(counter_icon(&object), Some(glyph::CHARGE));
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
        assert_eq!(
            words(&object, Lang::De, true).as_deref(),
            Some("1 Ladungsmarke")
        );

        let mut remora = crate::registry_printed(2, 0, "Mystic Remora");
        remora.counters.push(baylee_view::CounterEntry {
            kind: AGE_COUNTER,
            count: 2,
        });
        assert_eq!(words(&remora, Lang::De, false).as_deref(), Some("2 "));
        assert_eq!(counter_icon(&remora), Some(glyph::HOURGLASS));
        assert_eq!(
            words(&remora, Lang::De, true).as_deref(),
            Some("2 Altersmarken")
        );
        assert_eq!(
            words(&remora, Lang::En, true).as_deref(),
            Some("2 age counters")
        );
        remora.counters[0].count = 1;
        assert_eq!(
            words(&remora, Lang::De, true).as_deref(),
            Some("1 Altersmarke")
        );
        object.counters.clear();
        assert_eq!(words(&object, Lang::De, false), None);
    }

    /// Reports 21 and 30: a permanent that lost all its abilities is labelled
    /// "No abilities", first; a leveler says its level, short ("LV 5") and,
    /// with detail, long ("Level 5"); in German "Keine Fähigkeiten", "St. 5"
    /// and "Stufe 5"; and an object with neither has no label. On the old
    /// code `words` knew neither, so every assertion but the last read
    /// `None`.
    #[test]
    fn a_permanent_without_abilities_or_with_a_level_says_so() {
        use baylee_view::{CounterEntry, CounterKind, ObjectStatus};
        let mut object = crate::registry_printed(1, 0, "Reflections of Littjara");
        assert_eq!(words(&object, Lang::En, false), None, "neither: no label");
        assert_eq!(words(&object, Lang::De, true), None);

        object.status = ObjectStatus::NO_ABILITIES;
        assert_eq!(
            words(&object, Lang::En, false).as_deref(),
            Some("No abilities")
        );
        assert_eq!(
            words(&object, Lang::De, false).as_deref(),
            Some("Keine Fähigkeiten")
        );

        object.status = ObjectStatus::NONE;
        object.counters.push(CounterEntry {
            kind: CounterKind::Level,
            count: 5,
        });
        assert_eq!(words(&object, Lang::En, false).as_deref(), Some("LV 5"));
        assert_eq!(words(&object, Lang::En, true).as_deref(), Some("Level 5"));
        assert_eq!(words(&object, Lang::De, false).as_deref(), Some("St. 5"));
        assert_eq!(words(&object, Lang::De, true).as_deref(), Some("Stufe 5"));
        object.counters[0].count = 0;
        assert_eq!(
            words(&object, Lang::En, false),
            None,
            "level 0 says nothing"
        );

        // Both at once: "No abilities" comes first.
        object.counters[0].count = 5;
        object.status = ObjectStatus::NO_ABILITIES;
        assert_eq!(
            words(&object, Lang::En, false).as_deref(),
            Some("No abilities \u{b7} LV 5")
        );
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

    #[test]
    fn a_dungeon_in_the_command_zone_carries_its_active_room_badge() {
        use baylee_client_core::test_support::{ViewBuilder, token};
        let mut dungeon = token(7, 0, "Tomb of Annihilation — Oubliette", 0, 0);
        dungeon.types = baylee_core::types::TypeSet::DUNGEON;
        dungeon.power = None;
        dungeon.toughness = None;
        let mut duel = Duel::default();
        duel.receive_view(ViewBuilder::new(2).with_command(0, vec![dungeon]).build());
        let short = wanted_labels(&duel, Lang::En, false);
        assert_eq!(short.len(), 1);
        assert_eq!(short[0].1, "4/5 Oubliette");
        let detailed = wanted_labels(&duel, Lang::En, true);
        assert_eq!(
            detailed[0].1,
            "4/5 Oubliette — Discard a card and sacrifice a creature, an artifact, and a land."
        );
    }
}
