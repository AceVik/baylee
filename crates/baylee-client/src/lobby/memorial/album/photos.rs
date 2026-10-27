//! The family's photographs retain their original ratios beside the story.
use super::{ALBUM_ORDER, MemorialPress};
use crate::lobby::{LobbyState, Press};
use crate::shellkit::controls::{self, Kit, Live, Weight};
use crate::shellkit::focus::Stop;
use crate::shellkit::{metrics::px_fixed, surfaces};
use baylee_client_core::i18n::Phrase;
use bevy::prelude::*;

const PHOTOS: [(&str, f32); 5] = [
    ("memorial/baylee-1.jpg", 9.0 / 16.0),
    ("memorial/baylee-2.jpg", 16.0 / 9.0),
    ("memorial/baylee-3.jpg", 16.0 / 9.0),
    ("memorial/baylee-4.jpg", 9.0 / 16.0),
    ("memorial/baylee-5.jpg", 9.0 / 16.0),
];
pub(super) fn draw(
    commands: &mut Commands,
    parent: Entity,
    state: &LobbyState,
    kit: Kit,
    assets: Option<&AssetServer>,
) {
    let lang = state.lobby.lang();
    let side = commands
        .spawn((
            Node {
                width: Val::Percent(30.0),
                flex_shrink: 0.0,
                min_height: px_fixed(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let (path, aspect) = PHOTOS[state.memorial.photo % PHOTOS.len()];
    let image = assets.map_or_else(Handle::default, |a| a.load(path));
    let photo = commands
        .spawn((
            ImageNode::new(image),
            Node {
                height: Val::Percent(68.0),
                max_width: Val::Percent(100.0),
                aspect_ratio: Some(aspect),
                flex_shrink: 1.0,
                min_height: px_fixed(0.0),
                border_radius: BorderRadius::all(px_fixed(14.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let label = surfaces::prose(commands, kit, Phrase::MemorialPhotos.text(lang), true);
    let navigation = commands
        .spawn((
            Node {
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let prev = controls::button(
        commands,
        kit,
        "←",
        Weight::Ghost,
        Live::Yes,
        None,
        (
            Press::Memorial(MemorialPress::Previous),
            Stop::new(ALBUM_ORDER.name, "previous"),
        ),
    );
    let count = surfaces::prose(
        commands,
        kit,
        &format!("{} / 5", state.memorial.photo + 1),
        true,
    );
    let next = controls::button(
        commands,
        kit,
        "→",
        Weight::Ghost,
        Live::Yes,
        None,
        (
            Press::Memorial(MemorialPress::Next),
            Stop::new(ALBUM_ORDER.name, "next"),
        ),
    );
    for (entity, phrase) in [
        (prev, Phrase::MemorialPrevious),
        (next, Phrase::MemorialNext),
    ] {
        let mut access = accesskit::Node::new(accesskit::Role::Button);
        access.set_label(phrase.text(lang));
        commands
            .entity(entity)
            .insert(bevy::a11y::AccessibilityNode(access));
    }
    commands
        .entity(navigation)
        .add_children(&[prev, count, next]);
    commands
        .entity(side)
        .add_children(&[photo, label, navigation]);
    commands.entity(parent).add_child(side);
}
