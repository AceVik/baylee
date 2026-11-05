//! A quiet place for Baylee: the interactive cloud and a local photograph album.
mod album;
mod cloud;
mod motion;
mod paper;
mod preview;
pub(super) use cloud::Cloud;
pub(crate) use cloud::{CloudArt, CloudPlugin};
pub(super) use paper::BookPlugin;

/// The hub leaves this strip of sky free at the bottom for the unified footer pills.
pub(super) const CLOUD_FOOTER: f32 = 44.0;

use super::press::Cx;
use super::{LobbyState, Screen};
use crate::shellkit::controls::Kit;
use crate::shellkit::focus::{Stop, TabOrder};
use bevy::prelude::*;

/// The invitation's caption under the cloud, which the lobby's chat pill
/// stands clear of (`social::window`).
#[derive(Component)]
pub(crate) struct MemorialCaption;

/// Tab stays inside the album; Escape and the close button return to its opener.
pub(crate) const ALBUM_ORDER: TabOrder = TabOrder {
    name: "memorial",
    stops: &[
        "close",
        "photo-close",
        "chapters",
        "photo",
        "previous",
        "next",
        "zoom-out",
        "zoom-in",
        "fit",
        "reading",
        "back",
        "forward",
    ],
    modal: true,
};

/// Every way of opening or moving through the local album.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MemorialPress {
    Open,
    Close,
    PhotoOpen,
    PhotoClose,
    ZoomIn,
    ZoomOut,
    Fit,
    Previous,
    Next,
    Chapter(u8),
    Back,
    Forward,
    Nothing,
}

impl MemorialPress {
    pub(super) fn handle(self, cx: Cx<'_, '_, '_, '_, '_>) {
        if self == Self::Nothing {
            return;
        }
        let album = &mut cx.state.memorial;
        match self {
            Self::Open => {
                album.open = true;
                cx.state.lobby.park_caret();
            }
            Self::Close => {
                album.open = false;
                album.viewing = false;
            }
            Self::PhotoOpen => {
                album.viewing = true;
                album.zoom = 0;
            }
            Self::PhotoClose => {
                album.viewing = false;
                album.zoom = 0;
            }
            Self::ZoomIn => album.magnify(true),
            Self::ZoomOut => album.magnify(false),
            Self::Fit => album.zoom = 0,
            Self::Previous => album.turn(false),
            Self::Next => album.turn(true),
            Self::Chapter(i) if usize::from(i) < baylee_client_core::memorial::CHAPTERS => {
                if album.chapter != usize::from(i) {
                    album.select(usize::from(i));
                    cx.scrolled.set(super::List::Memorial, 0.0);
                }
            }
            Self::Back | Self::Forward => {
                let before = album.chapter;
                album.read(self == Self::Forward);
                if album.chapter != before {
                    cx.scrolled.set(super::List::Memorial, 0.0);
                }
            }
            Self::Chapter(_) | Self::Nothing => {}
        }
    }
}

/// Build the cloud alongside the screen, then its modal above it.
pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    kit: Kit,
    assets: Option<&AssetServer>,
    art: Option<&CloudArt>,
    scrolled: &super::Scrolled,
) {
    if state.settings_open()
        || state.terms.up()
        || !matches!(state.lobby.screen(), Screen::SignIn { .. } | Screen::Table)
    {
        return;
    }
    if state.memorial.open {
        album::draw(commands, root, state, kit, assets, art, scrolled);
    }
}

/// Every album close path restores its opener, even after the modal's nodes
/// have been rebuilt. Mouse users keep the ring hidden; keyboard users see it.
#[allow(clippy::too_many_arguments)] // Both nested close paths restore the shell’s remembered focus.
pub(super) fn return_focus(
    state: Res<LobbyState>,
    mut was_open: Local<bool>,
    mut was_viewing: Local<bool>,
    stops: Query<(Entity, &Stop), Without<crate::shellkit::controls::Disabled>>,
    clouds: Query<(Entity, &Stop), With<cloud::Cloud>>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
    mut remembered: ResMut<crate::shellkit::focus::Remembered>,
    mut kept: ResMut<super::focusing::Kept>,
) {
    if state.memorial.open {
        let target = if !*was_open {
            Some(if state.memorial.viewing {
                "photo-close"
            } else {
                "close"
            })
        } else if !*was_viewing && state.memorial.viewing {
            Some("photo-close")
        } else if *was_viewing && !state.memorial.viewing {
            Some("photo")
        } else {
            None
        };
        let chosen = target.and_then(|id| {
            stops
                .iter()
                .find(|(_, s)| s.table == ALBUM_ORDER.name && s.id == id)
        });
        let chosen = chosen.or_else(|| {
            if target.is_none()
                && focus
                    .get()
                    .and_then(|e| stops.get(e).ok())
                    .is_some_and(|(_, s)| s.table == ALBUM_ORDER.name)
            {
                return None;
            }
            remembered
                .0
                .filter(|s| s.table == ALBUM_ORDER.name)
                .and_then(|saved| stops.iter().find(|(_, s)| **s == saved))
                .or_else(|| {
                    stops.iter().find(|(_, s)| {
                        s.table == ALBUM_ORDER.name
                            && s.id
                                == if state.memorial.viewing {
                                    "fit"
                                } else {
                                    "close"
                                }
                    })
                })
        });
        if let Some((entity, stop)) = chosen {
            focus.set(entity, bevy::input_focus::FocusCause::Navigated);
            if remembered.0 != Some(*stop) {
                remembered.0 = Some(*stop);
            }
            kept.keep(*stop);
        } else if target.is_some() {
            return;
        }
        *was_open = true;
        *was_viewing = state.memorial.viewing;
        return;
    }
    if !*was_open {
        return;
    }
    *was_open = false;
    if let Some((entity, stop)) = clouds
        .iter()
        .next()
        .or_else(|| stops.iter().find(|(_, s)| s.id == "baylee"))
    {
        focus.set(entity, bevy::input_focus::FocusCause::Navigated);
        remembered.0 = Some(*stop);
        kept.keep(*stop);
    }
}

/// Chapter bookmarks own their roving arrows; photo navigation never consumes
/// the same key. Modified arrows keep focus on the newly selected bookmark.
pub(super) fn chapter_focus(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<LobbyState>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
    stops: Query<(Entity, &Stop)>,
    mut remembered: ResMut<crate::shellkit::focus::Remembered>,
    mut scrolled: ResMut<super::Scrolled>,
) {
    if !state.memorial.open
        || state.memorial.viewing
        || !keys.any_pressed([
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
            KeyCode::Home,
            KeyCode::End,
        ])
    {
        return;
    }
    let Some((_, stop)) = focus.get().and_then(|e| stops.get(e).ok()) else {
        return;
    };
    if stop.table != ALBUM_ORDER.name || stop.id != "chapters" {
        return;
    }
    if keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]) {
        if let Some((entity, stop)) = stops.iter().find(|(_, s)| {
            s.table == ALBUM_ORDER.name
                && s.id == "chapters"
                && usize::from(s.item) == state.memorial.chapter
        }) {
            if focus.get() != Some(entity) {
                focus.set(entity, bevy::input_focus::FocusCause::Navigated);
            }
            if remembered.0 != Some(*stop) {
                remembered.0 = Some(*stop);
            }
        }
    } else if !keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]) && state.memorial.chapter != usize::from(stop.item)
    {
        state.memorial.select(usize::from(stop.item));
        scrolled.set(super::List::Memorial, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shellkit::focus::Remembered;
    use bevy::input_focus::InputFocus;

    #[test]
    fn closing_the_album_restores_the_rebuilt_opener_without_mutating_state() {
        let mut app = App::new();
        app.init_resource::<LobbyState>()
            .init_resource::<InputFocus>()
            .init_resource::<Remembered>()
            .init_resource::<super::super::focusing::Kept>()
            .add_systems(Update, return_focus);
        app.world_mut().spawn(Stop::new(ALBUM_ORDER.name, "close"));
        app.world_mut().resource_mut::<LobbyState>().memorial.open = true;
        app.update();
        // A redraw gives the opener a fresh entity after the album closes.
        let stop = Stop::new("front-gateway", "baylee");
        let opener = app
            .world_mut()
            .spawn((cloud::Cloud { front: true }, stop))
            .id();
        app.world_mut().resource_mut::<LobbyState>().memorial.open = false;
        let state_tick = app.world().resource_ref::<LobbyState>().last_changed();
        app.update();
        assert_eq!(
            app.world().resource_ref::<LobbyState>().last_changed(),
            state_tick
        );
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(opener));
        assert_eq!(app.world().resource::<Remembered>().0, Some(stop));
    }
}
