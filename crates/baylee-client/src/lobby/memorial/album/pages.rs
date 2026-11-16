//! Smoked obsidian-vellum reading well, `OnlyWhenNeeded` scrollbar, and keyboard reading.
use super::{ALBUM_ORDER, style};
use crate::lobby::{List, LobbyState, Scrollable, Scrolled, scrollbars};
use crate::shellkit::{
    Frame, controls::Kit, focus::Stop, metrics::px_fixed, role::Role, surfaces, tokens,
};
use baylee_client_core::{i18n::Phrase, memorial};
use bevy::prelude::*;

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
            crate::hud::tf_serif_italic(
                kit.fonts,
                surfaces::whole_pixels(size),
                if title { 600 } else { 500 },
            ),
            TextColor(if title {
                style::TITLE_INK
            } else {
                style::STORY_INK
            }),
            bevy::text::LineHeight::RelativeToFont(1.52),
            Node {
                width: Val::Percent(100.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

pub(super) fn draw(
    commands: &mut Commands,
    state: &LobbyState,
    kit: Kit,
    scrolled: &Scrolled,
) -> Entity {
    let lang = state.lobby.lang();
    let phone = kit.m.frame == Frame::Phone;
    let (title, body) = memorial::STORY[state.memorial.chapter.min(memorial::CHAPTERS - 1)];

    let well = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_height: px_fixed(0.0),
                width: Val::Percent(100.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(style::READING_WELL),
            BorderColor::all(style::BRASS_BORDER.with_alpha(0.55)),
            Pickable::IGNORE,
        ))
        .id();
    super::super::paper::attach(commands, well, false);

    let reading = commands
        .spawn((
            Reading,
            Role::Scroll,
            Scrollable(List::Memorial),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::Memorial))),
            Stop::new(ALBUM_ORDER.name, "reading"),
            Pickable::default(),
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                min_height: px_fixed(0.0),
                height: Val::Percent(100.0),
                overflow: Overflow::scroll_y(),
                padding: UiRect::axes(kit.m.px(22.0), kit.m.px(18.0)),
                row_gap: kit.m.px(14.0),
                ..default()
            },
        ))
        .id();
    let mut access = accesskit::Node::new(accesskit::Role::Document);
    access.set_label(title.text(lang));
    commands.entity(reading).insert((
        bevy::a11y::AccessibilityNode(access),
        Name::new(Phrase::MemorialRead.text(lang).to_owned()),
    ));

    let heading = serif(commands, kit, title.text(lang), kit.m.head * 1.18, true);
    commands.entity(reading).add_child(heading);
    for paragraph in body.text(lang).split("\n\n") {
        let text = serif(commands, kit, paragraph, kit.m.text, false);
        commands.entity(reading).add_child(text);
    }
    let ornament = ornament(commands, kit);
    commands.entity(reading).add_child(ornament);

    let band = scrollbars::band_when_needed(commands, reading, phone);
    commands
        .entity(band)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.height = Val::Percent(100.0);
            node.flex_grow = 1.0;
        });
    commands.entity(well).add_child(band);
    well
}

pub(in crate::lobby::memorial) fn reading_keys(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<LobbyState>,
    focus: Res<bevy::input_focus::InputFocus>,
    mut texts: Query<(Entity, &mut ScrollPosition, &ComputedNode), With<Reading>>,
    mut scrolled: ResMut<Scrolled>,
) {
    if !state.memorial.open || state.memorial.viewing {
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

/// The previous chapter is a visual-only leaf above the new reading viewport.
/// Its text keeps the old scroll while the leaf pivots towards its binding.
pub(super) fn leaf(c: &mut Commands, parent: Entity, state: &LobbyState, kit: Kit) {
    if state.memorial.revision == 0 {
        return;
    }
    let page = c
        .spawn((
            super::super::paper::Leaf,
            Node {
                position_type: PositionType::Absolute,
                left: px_fixed(0.0),
                top: px_fixed(0.0),
                right: px_fixed(0.0),
                height: Val::Percent(100.0),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(style::READING_WELL),
            Visibility::Hidden,
            ZIndex(2),
            Pickable::IGNORE,
        ))
        .id();
    super::super::paper::attach(c, page, true);
    let mut access = accesskit::Node::new(accesskit::Role::GenericContainer);
    access.set_hidden();
    c.entity(page).insert(bevy::a11y::AccessibilityNode(access));
    let words = c
        .spawn((
            super::super::paper::LeafScroll,
            ScrollPosition::default(),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                padding: UiRect::axes(kit.m.px(22.0), kit.m.px(18.0)),
                row_gap: kit.m.px(14.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let (title, body) = memorial::STORY[state.memorial.previous_chapter];
    let heading = serif(
        c,
        kit,
        title.text(state.lobby.lang()),
        kit.m.head * 1.18,
        true,
    );
    c.entity(words).add_child(heading);
    for paragraph in body.text(state.lobby.lang()).split("\n\n") {
        let text = serif(c, kit, paragraph, kit.m.text, false);
        c.entity(words).add_child(text);
    }
    let ornament = ornament(c, kit);
    c.entity(words).add_child(ornament);
    super::super::paper::restore_scroll(c, words);
    c.entity(page).add_child(words);
    c.entity(parent).add_child(page);
}

fn ornament(commands: &mut Commands, kit: Kit) -> Entity {
    let ornament = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: kit.m.px(24.0),
                column_gap: kit.m.px(10.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
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
                BackgroundColor(style::BRASS),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(ornament).add_child(diamond);
    }
    ornament
}
