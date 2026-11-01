//! A parchment remembrance book; photographs and writing keep independent places.
mod chrome;
mod pages;
mod photos;
mod style;
mod viewer;
use super::{ALBUM_ORDER, CloudArt, MemorialPress};
use crate::lobby::{LobbyState, Press, Scrolled};
use crate::shellkit::controls::Kit;
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::{role::Role, surfaces};
use bevy::prelude::*;
pub(super) use pages::reading_keys;
pub(super) use viewer::{PhotoPan, input as viewer_input, layout as viewer_layout};

#[allow(clippy::too_many_arguments)] // The book shares the shell's assets and scroll memory.
pub(super) fn draw(
    c: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    assets: Option<&AssetServer>,
    art: Option<&CloudArt>,
    scrolled: &Scrolled,
) {
    if state.memorial.viewing {
        viewer::draw(c, root, state, kit, assets);
        return;
    }
    let surface = c
        .spawn((
            Role::Opaque,
            style::elevation(24.0),
            Node {
                width: kit.m.px(1140.0),
                max_width: Val::Percent(94.0),
                height: Val::Percent(92.0),
                max_height: kit.m.px(880.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(kit.m.px(24.0)),
                row_gap: kit.m.px(10.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(10.0)),
                ..default()
            },
            BackgroundColor(style::PAPER),
            BorderColor::all(style::GOLD),
            Press::Memorial(MemorialPress::Nothing),
        ))
        .id();
    super::paper::attach(c, surface, false);
    super::motion::attach(c, surface, false);
    let header = chrome::header(c, state, kit, art);
    let rule = style::rule(c, kit);
    let body = c
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_height: px_fixed(0.0),
                column_gap: kit.m.px(20.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    photos::draw(c, body, state, kit, assets);
    let writing = c
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                min_height: px_fixed(0.0),
                row_gap: kit.m.px(10.0),
                border: UiRect::left(px_fixed(1.0)),
                padding: UiRect::left(kit.m.px(20.0)),
                ..default()
            },
            BorderColor::all(style::GOLD.with_alpha(0.3)),
            Pickable::IGNORE,
        ))
        .id();
    let chapters = chrome::chapters(c, state, kit);
    let reading = c
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_height: px_fixed(0.0),
                width: Val::Percent(100.0),
                overflow: Overflow::clip(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    pages::draw(c, reading, state, kit, scrolled);
    pages::leaf(c, reading, state, kit);
    let rule = [rule, style::rule(c, kit)];
    let footer = chrome::footer(c, state, kit);
    c.entity(writing)
        .add_children(&[chapters, reading, rule[1], footer]);
    c.entity(body).add_child(writing);
    c.entity(surface).add_children(&[header, rule[0], body]);
    let scrim = surfaces::sheet(c, surface);
    c.entity(scrim).insert((
        GlobalZIndex(crate::shellkit::tokens::z::TOAST + 1),
        Press::Memorial(MemorialPress::Close),
    ));
    c.entity(root).add_child(scrim);
}
