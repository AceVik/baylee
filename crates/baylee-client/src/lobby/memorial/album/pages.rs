//! Readable paragraphs, a real scrollbar and keyboard scrolling in the modal.
use super::ALBUM_ORDER;
use crate::lobby::{List, LobbyState, Scrollable, Scrolled};
use crate::shellkit::{controls::Kit, focus::Stop, metrics::px_fixed, surfaces, tokens};
use baylee_client_core::{i18n::Phrase, memorial};
use bevy::prelude::*;
use bevy::ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb};

#[derive(Component)]
pub(in crate::lobby::memorial) struct Reading;

pub(super) fn serif(
    commands: &mut Commands,
    kit: Kit,
    text: &str,
    size: f32,
    title: bool,
) -> Entity {
    commands
        .spawn((
            Text::new(text),
            crate::hud::tf_serif(
                kit.fonts,
                surfaces::whole_pixels(size),
                if title { 500 } else { 400 },
            ),
            TextColor(if title {
                Color::srgb(0.93, 0.80, 0.55)
            } else {
                tokens::INK
            }),
            bevy::text::LineHeight::RelativeToFont(1.45),
            Node {
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

pub(super) fn draw(
    commands: &mut Commands,
    parent: Entity,
    state: &LobbyState,
    kit: Kit,
    scrolled: &Scrolled,
) {
    let lang = state.lobby.lang();
    let (title, body) = memorial::STORY[state.memorial.chapter.min(memorial::CHAPTERS - 1)];
    let reading = commands
        .spawn((
            Reading,
            crate::shellkit::role::Role::Scroll,
            Scrollable(List::Memorial),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::Memorial))),
            Stop::new(ALBUM_ORDER.name, "reading"),
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                min_height: px_fixed(0.0),
                height: Val::Percent(100.0),
                overflow: Overflow::scroll_y(),
                padding: UiRect::axes(kit.m.px(16.0), kit.m.px(8.0)),
                row_gap: kit.m.px(15.0),
                ..default()
            },
        ))
        .id();
    let mut access = accesskit::Node::new(accesskit::Role::Document);
    access.set_label(title.text(lang));
    commands
        .entity(reading)
        .insert(bevy::a11y::AccessibilityNode(access));
    let heading = serif(commands, kit, title.text(lang), kit.m.head * 1.15, true);
    commands.entity(reading).add_child(heading);
    for paragraph in body.text(lang).split("\n\n") {
        let text = serif(commands, kit, paragraph, kit.m.text, false);
        commands.entity(reading).add_child(text);
    }
    // Geometry keeps the gold ornament independent of font glyph coverage.
    let ornament = commands
        .spawn((
            Node {
                height: kit.m.px(24.0),
                column_gap: kit.m.px(10.0),
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for size in [4.0, 6.0, 4.0] {
        let diamond = commands
            .spawn((
                Node {
                    width: kit.m.px(size),
                    height: kit.m.px(size),
                    ..default()
                },
                UiTransform::from_rotation(Rot2::radians(std::f32::consts::FRAC_PI_4)),
                BackgroundColor(Color::srgb(0.78, 0.65, 0.39)),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(ornament).add_child(diamond);
    }
    commands.entity(reading).add_child(ornament);
    let track = commands
        .spawn((
            Node {
                width: kit.m.px(12.0),
                height: Val::Percent(100.0),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px_fixed(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.14, 0.17)),
            Scrollbar::new(reading, ControlOrientation::Vertical, 30.0),
        ))
        .id();
    let thumb = commands
        .spawn((
            ScrollbarThumb {
                border_radius: BorderRadius::all(px_fixed(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.68, 0.56, 0.34)),
            Pickable::default(),
        ))
        .id();
    commands.entity(track).add_child(thumb);
    commands.entity(parent).add_children(&[reading, track]);
    // Name the keyboard-scroll destination in the accessibility tree.
    commands
        .entity(reading)
        .insert(Name::new(Phrase::MemorialRead.text(lang).to_owned()));
}

pub(in crate::lobby::memorial) fn reading_keys(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<LobbyState>,
    focus: Res<bevy::input_focus::InputFocus>,
    mut texts: Query<(Entity, &mut ScrollPosition, &ComputedNode), With<Reading>>,
    mut scrolled: ResMut<Scrolled>,
) {
    if !state.memorial.open {
        return;
    }
    let Ok((entity, mut position, node)) = texts.single_mut() else {
        return;
    };
    // Home/End and arrows belong to a focused chapter button's roving group.
    // Page keys can turn the reading viewport from anywhere inside this modal.
    if focus.get() != Some(entity) && !keys.any_just_pressed([KeyCode::PageDown, KeyCode::PageUp]) {
        return;
    }
    let height = node.size().y * node.inverse_scale_factor();
    let end = (node.content_size().y * node.inverse_scale_factor() - height).max(0.0);
    let to = if keys.just_pressed(KeyCode::PageDown) {
        position.y + height * 0.85
    } else if keys.just_pressed(KeyCode::PageUp) {
        position.y - height * 0.85
    } else if keys.just_pressed(KeyCode::Home) {
        0.0
    } else if keys.just_pressed(KeyCode::End) {
        end
    } else if keys.just_pressed(KeyCode::ArrowDown) {
        position.y + 40.0
    } else if keys.just_pressed(KeyCode::ArrowUp) {
        position.y - 40.0
    } else {
        return;
    }
    .clamp(0.0, end);
    if (position.y - to).abs() > f32::EPSILON {
        position.y = to;
        scrolled.set(List::Memorial, to);
    }
}
