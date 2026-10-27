//! A local remembrance book with independent chapters and photographs.
mod chrome;
mod pages;
mod photos;
use super::{ALBUM_ORDER, CloudArt, MemorialPress};
use crate::lobby::{LobbyState, Press, Scrolled};
use crate::shellkit::controls::Kit;
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::{role::Role, surfaces};
use bevy::prelude::*;
pub(super) use pages::reading_keys;

#[allow(clippy::too_many_arguments)] // The book shares the shell's assets and scroll memory.
pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    assets: Option<&AssetServer>,
    art: Option<&CloudArt>,
    scrolled: &Scrolled,
) {
    let surface = commands
        .spawn((
            Role::Opaque,
            Node {
                width: kit.m.px(1140.0),
                max_width: Val::Percent(94.0),
                height: Val::Percent(92.0),
                max_height: kit.m.px(880.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(kit.m.px(22.0)),
                row_gap: kit.m.px(12.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(24.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.055, 0.085, 0.105)),
            BorderColor::all(Color::srgb(0.78, 0.65, 0.39)),
            Press::Memorial(MemorialPress::Nothing),
        ))
        .id();
    let header = chrome::header(commands, state, kit, art);
    let chapters = chrome::chapters(commands, state, kit);
    let body = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_height: px_fixed(0.0),
                column_gap: kit.m.px(24.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    photos::draw(commands, body, state, kit, assets);
    pages::draw(commands, body, state, kit, scrolled);
    let footer = chrome::footer(commands, state, kit);
    commands
        .entity(surface)
        .add_children(&[header, chapters, body, footer]);
    let scrim = surfaces::sheet(commands, surface);
    commands
        .entity(scrim)
        .insert(Press::Memorial(MemorialPress::Close));
    commands.entity(root).add_child(scrim);
}
