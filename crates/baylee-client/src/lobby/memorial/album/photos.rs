//! Framed original photographs with independent navigation.
use super::{ALBUM_ORDER, MemorialPress, style};
use crate::lobby::{LobbyState, Press};
use crate::shellkit::{
    controls::{Kit, Live},
    focus::Stop,
    metrics::px_fixed,
};
use baylee_client_core::i18n::Phrase;
use bevy::prelude::*;

pub(super) const PHOTOS: [(&str, f32); baylee_client_core::memorial::PHOTOS] = [
    ("memorial/baylee-1.jpg", 9.0 / 16.0),
    ("memorial/baylee-2.jpg", 16.0 / 9.0),
    ("memorial/baylee-3.jpg", 16.0 / 9.0),
    ("memorial/baylee-4.jpg", 9.0 / 16.0),
    ("memorial/baylee-5.jpg", 9.0 / 16.0),
    ("memorial/baylee-6.jpg", 16.0 / 9.0),
    ("memorial/baylee-7.jpg", 16.0 / 9.0),
    ("memorial/baylee-8.jpg", 16.0 / 9.0),
    ("memorial/baylee-9.jpg", 9.0 / 16.0),
    ("memorial/baylee-10.jpg", 16.0 / 9.0),
];
pub(super) fn draw(
    c: &mut Commands,
    parent: Entity,
    state: &LobbyState,
    kit: Kit,
    assets: Option<&AssetServer>,
) {
    let lang = state.lobby.lang();
    let side = c
        .spawn((
            Node {
                width: Val::Percent(29.0),
                flex_shrink: 0.0,
                min_height: px_fixed(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(10.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(kit.m.px(14.0), kit.m.px(10.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    let frame = c
        .spawn((
            crate::ui_button::UiButton,
            crate::shellkit::role::Role::Hit,
            Press::Memorial(MemorialPress::PhotoOpen),
            Stop::new(ALBUM_ORDER.name, "photo"),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(72.0),
                min_height: px_fixed(0.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(kit.m.px(9.0)),
                border_radius: BorderRadius::all(kit.m.px(16.0)),
                border: UiRect::all(px_fixed(1.0)),
                ..default()
            },
            style::elevation(6.0),
            BackgroundColor(Color::srgb(0.99, 0.96, 0.89)),
            BorderColor::all(style::GOLD.with_alpha(0.6)),
            crate::ambience::Feel::tinting_to(
                Color::srgb(0.99, 0.96, 0.89),
                Color::srgb(1.0, 0.91, 0.68),
            ),
        ))
        .id();
    let mut access = accesskit::Node::new(accesskit::Role::Button);
    access.set_label(Phrase::MemorialPhotoOpen.text(lang));
    c.entity(frame)
        .insert(bevy::a11y::AccessibilityNode(access));
    let photo = c
        .spawn((
            BackgroundColor(style::PAPER),
            Node {
                height: Val::Percent(100.0),
                width: Val::Percent(100.0),
                border_radius: BorderRadius::all(kit.m.px(12.0)),
                flex_shrink: 1.0,
                min_height: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    super::super::preview::attach(c, photo, state.memorial.photo, assets.is_some());
    super::super::motion::attach(c, photo, true);
    c.entity(frame).add_child(photo);
    let label = style::label(c, kit, Phrase::MemorialPhotos.text(lang), false);
    let hint = style::label(c, kit, Phrase::MemorialPhotoOpen.text(lang), true);
    let navigation = navigation(c, state, kit);
    c.entity(side)
        .add_children(&[frame, label, hint, navigation]);
    c.entity(parent).add_child(side);
}

pub(super) fn navigation(c: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let row = c
        .spawn((
            Node {
                column_gap: kit.m.px(10.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let prev = style::button(
        c,
        kit,
        "←",
        Live::Yes,
        false,
        (
            Press::Memorial(MemorialPress::Previous),
            Stop::new(ALBUM_ORDER.name, "previous"),
        ),
    );
    let count = style::label(
        c,
        kit,
        &format!("{:02} / {:02}", state.memorial.photo + 1, PHOTOS.len()),
        true,
    );
    let next = style::button(
        c,
        kit,
        "→",
        Live::Yes,
        false,
        (
            Press::Memorial(MemorialPress::Next),
            Stop::new(ALBUM_ORDER.name, "next"),
        ),
    );
    for (e, p) in [
        (prev, Phrase::MemorialPrevious),
        (next, Phrase::MemorialNext),
    ] {
        let mut access = accesskit::Node::new(accesskit::Role::Button);
        access.set_label(p.text(state.lobby.lang()));
        c.entity(e).insert(bevy::a11y::AccessibilityNode(access));
    }
    c.entity(row).add_children(&[prev, count, next]);
    row
}
