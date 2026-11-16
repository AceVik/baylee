//! Baylee's symmetric gallery-grade memorial dialog (`Über Baylee`).
mod chrome;
mod pages;
mod photos;
mod style;
mod viewer;
use super::{ALBUM_ORDER, CloudArt, MemorialPress};
use crate::lobby::{LobbyState, Press, Scrolled};
use crate::shellkit::controls::Kit;
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
    let surface = surfaces::dialog(
        c,
        kit,
        Val::Percent(92.0),
        style::BRASS_BORDER,
        Press::Memorial(MemorialPress::Nothing),
    );
    c.entity(surface)
        .insert((
            BackgroundColor(Color::NONE),
            style::dialog_gradient(),
            style::dialog_border(),
        ))
        .entry::<Node>()
        .and_modify(move |mut node| {
            if phone {
                node.width = Val::Percent(100.0);
                node.max_width = Val::Percent(100.0);
                node.height = Val::Percent(100.0);
                node.max_height = Val::Percent(100.0);
            } else {
                node.width = Val::Percent(92.0);
                node.max_width = kit.m.px(820.0);
                node.height = Val::Vh(86.0);
                node.max_height = kit.m.px(780.0);
            }
        });
    super::motion::attach(c, surface, false);

    let header = chrome::header(c, state, kit, art);
    let chapters = chrome::chapters(c, state, kit);
    let body = pages::draw(c, state, kit, assets, scrolled);
    let footer = chrome::footer(c, state, kit);
    c.entity(surface)
        .add_children(&[header, chapters, body, footer]);

    let scrim = surfaces::scrim(
        c,
        kit,
        tokens::z::TOAST + 1,
        Press::Memorial(MemorialPress::Close),
        surface,
    );
    c.entity(root).add_child(scrim);
}
