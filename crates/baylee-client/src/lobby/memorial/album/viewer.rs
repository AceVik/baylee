//! A contained photograph viewer: explicit zoom, bounded drag and Escape.
mod geometry;
pub(in crate::lobby::memorial) use geometry::{PhotoPan, input, layout};
use geometry::{drag, restore};

use super::{ALBUM_ORDER, MemorialPress, photos, style};
use crate::lobby::{LobbyState, Press};
use crate::shellkit::{
    controls::{Kit, Live},
    focus::Stop,
    metrics::px_fixed,
    surfaces,
};
use baylee_client_core::i18n::Phrase;
use bevy::prelude::*;

#[derive(Component)]
pub(in crate::lobby::memorial) struct PhotoViewport;
#[derive(Component)]
pub(in crate::lobby::memorial) struct LargePhoto;

pub(super) fn draw(
    c: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    assets: Option<&AssetServer>,
) {
    let lang = state.lobby.lang();
    let surface = c
        .spawn((
            Node {
                width: Val::Percent(94.0),
                height: Val::Percent(94.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(kit.m.px(18.0)),
                row_gap: kit.m.px(10.0),
                ..default()
            },
            BackgroundColor(style::PAPER),
            style::elevation(28.0),
            Press::Memorial(MemorialPress::Nothing),
        ))
        .id();
    super::super::paper::attach(c, surface, false);
    super::super::motion::attach(c, surface, false);
    let header = c
        .spawn((
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title = style::label(c, kit, Phrase::MemorialPhotos.text(lang), false);
    let close = style::button(
        c,
        kit,
        "×",
        Live::Yes,
        false,
        (
            Press::Memorial(MemorialPress::PhotoClose),
            Stop::new(ALBUM_ORDER.name, "photo-close"),
        ),
    );
    let mut access = accesskit::Node::new(accesskit::Role::Button);
    access.set_label(Phrase::SheetClose.text(lang));
    c.entity(close)
        .insert(bevy::a11y::AccessibilityNode(access));
    c.entity(header).add_children(&[title, close]);
    let viewport = c
        .spawn((
            PhotoViewport,
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_height: px_fixed(0.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.14, 0.14)),
        ))
        .observe(drag)
        .id();
    let path = format!("memorial/full/baylee-{}.jpg", state.memorial.photo + 1);
    let image = c
        .spawn((
            LargePhoto,
            Visibility::Hidden,
            ImageNode::new(assets.map_or_else(Handle::default, |a| a.load(path))),
            Node {
                width: Val::Px(1.0),
                height: Val::Px(1.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    restore(c, image);
    c.entity(viewport).add_child(image);
    let toolbar = toolbar(c, state, kit);
    c.entity(surface).add_children(&[header, viewport, toolbar]);
    let scrim = surfaces::sheet(c, surface);
    c.entity(scrim).insert((
        GlobalZIndex(crate::shellkit::tokens::z::TOAST + 1),
        Press::Memorial(MemorialPress::PhotoClose),
    ));
    c.entity(root).add_child(scrim);
}

fn toolbar(c: &mut Commands, state: &LobbyState, kit: Kit) -> Entity {
    let lang = state.lobby.lang();
    let toolbar = c
        .spawn((
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let navigation = photos::navigation(c, state, kit);
    let zoom = c
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let out = style::button(
        c,
        kit,
        "−",
        if state.memorial.zoom > 0 {
            Live::Yes
        } else {
            Live::No(Phrase::MemorialZoomMinimum.text(lang))
        },
        false,
        (
            Press::Memorial(MemorialPress::ZoomOut),
            Stop::new(ALBUM_ORDER.name, "zoom-out"),
        ),
    );
    let percent = style::label(
        c,
        kit,
        &format!("{} %", 100 + u16::from(state.memorial.zoom) * 50),
        true,
    );
    let into = style::button(
        c,
        kit,
        "+",
        if state.memorial.zoom < 6 {
            Live::Yes
        } else {
            Live::No(Phrase::MemorialZoomMaximum.text(lang))
        },
        false,
        (
            Press::Memorial(MemorialPress::ZoomIn),
            Stop::new(ALBUM_ORDER.name, "zoom-in"),
        ),
    );
    let fit = style::button(
        c,
        kit,
        Phrase::MemorialFit.text(lang),
        Live::Yes,
        false,
        (
            Press::Memorial(MemorialPress::Fit),
            Stop::new(ALBUM_ORDER.name, "fit"),
        ),
    );
    for (e, phrase) in [
        (out, Phrase::MemorialZoomOut),
        (into, Phrase::MemorialZoomIn),
    ] {
        let mut access = accesskit::Node::new(accesskit::Role::Button);
        access.set_label(phrase.text(lang));
        c.entity(e).insert(bevy::a11y::AccessibilityNode(access));
    }
    c.entity(zoom).add_children(&[out, percent, into, fit]);
    c.entity(toolbar).add_children(&[navigation, zoom]);
    toolbar
}
