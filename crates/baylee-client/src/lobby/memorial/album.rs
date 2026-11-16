//! Baylee's memorial dialog, standardized on `shellkit` out-of-table surfaces.
mod chrome;
mod pages;
mod photos;
mod style;
mod viewer;
use super::{ALBUM_ORDER, CloudArt, MemorialPress};
use crate::lobby::{LobbyState, Press, Scrolled};
use crate::shellkit::controls::Kit;
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::{Frame, surfaces, tokens};
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
    let phone = kit.m.frame == Frame::Phone;
    let stacked = matches!(kit.m.frame, Frame::Phone | Frame::Compact);
    let surface = surfaces::dialog(
        c,
        kit,
        kit.m.px(1120.0),
        style::BRASS_BORDER,
        Press::Memorial(MemorialPress::Nothing),
    );
    c.entity(surface)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.height = if phone {
                Val::Percent(100.0)
            } else {
                Val::Percent(90.0)
            };
            node.max_height = if phone {
                Val::Percent(100.0)
            } else {
                kit.m.px(860.0)
            };
        });
    super::motion::attach(c, surface, false);

    let header = chrome::header(c, state, kit, art);
    let body = c
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: if stacked {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_height: px_fixed(0.0),
                padding: UiRect::all(px_fixed(kit.m.pad)),
                column_gap: kit.m.px(20.0),
                row_gap: kit.m.px(14.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    photos::draw(c, body, state, kit, assets, stacked);

    let writing = c
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                min_height: px_fixed(0.0),
                row_gap: kit.m.px(12.0),
                border: if stacked {
                    UiRect::top(px_fixed(1.0))
                } else {
                    UiRect::left(px_fixed(1.0))
                },
                padding: if stacked {
                    UiRect::top(kit.m.px(12.0))
                } else {
                    UiRect::left(kit.m.px(20.0))
                },
                ..default()
            },
            BorderColor::all(tokens::BORDER),
            Pickable::IGNORE,
        ))
        .id();
    let chapters = chrome::chapters(c, state, kit);
    let reading = pages::draw(c, state, kit, scrolled);
    pages::leaf(c, reading, state, kit);
    let footer = chrome::footer(c, state, kit);
    c.entity(writing).add_children(&[chapters, reading, footer]);
    c.entity(body).add_child(writing);
    c.entity(surface).add_children(&[header, body]);

    let scrim = surfaces::scrim(
        c,
        kit,
        tokens::z::TOAST + 1,
        Press::Memorial(MemorialPress::Close),
        surface,
    );
    c.entity(root).add_child(scrim);
}
