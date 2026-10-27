//! A quiet place for Baylee: the interactive cloud and a local photograph album.
mod album;
mod cloud;
pub(crate) use cloud::{CloudArt, CloudPlugin};

use super::press::Cx;
use super::{LobbyState, Press, Screen};
use crate::shellkit::controls::Kit;
use crate::shellkit::focus::{Stop, TabOrder};
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::role::Role;
use crate::ui_button::UiButton;
use baylee_client_core::i18n::Phrase;
use bevy::prelude::*;

/// Tab stays inside the album; Escape and the close button return to its opener.
pub(crate) const ALBUM_ORDER: TabOrder = TabOrder {
    name: "memorial",
    stops: &[
        "chapters", "previous", "next", "reading", "back", "forward", "close",
    ],
    modal: true,
};

/// Every way of opening or moving through the local album.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MemorialPress {
    Open,
    Close,
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
            Self::Open => album.open = true,
            Self::Close => album.open = false,
            Self::Previous => album.turn(false),
            Self::Next => album.turn(true),
            Self::Chapter(i) if usize::from(i) < baylee_client_core::memorial::CHAPTERS => {
                album.chapter = usize::from(i);
                cx.scrolled.set(super::List::Memorial, 0.0);
            }
            Self::Back | Self::Forward => {
                album.read(self == Self::Forward);
                cx.scrolled.set(super::List::Memorial, 0.0);
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
    let front = matches!(state.lobby.screen(), Screen::SignIn { .. });
    let table = if front {
        super::front::keys::table_of(state).name
    } else if state.lobby.awaiting().is_some() && !state.room_away {
        "room"
    } else if state.hub == super::Hub::Decks {
        "decks"
    } else {
        "play"
    };
    let name = Phrase::MemorialOpen.text(state.lobby.lang());
    let mut a11y = accesskit::Node::new(accesskit::Role::Button);
    a11y.set_label(name);
    let cloud = commands
        .spawn((
            cloud::Cloud { front },
            bevy::ui::RelativeCursorPosition::default(),
            UiButton,
            Role::Hit,
            Name::new(name.to_owned()),
            bevy::a11y::AccessibilityNode(a11y),
            Press::Memorial(MemorialPress::Open),
            Stop::new(table, "baylee"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(1.0),
                top: Val::Percent(if front { 37.0 } else { 70.0 }),
                width: Val::Percent(25.0),
                max_width: px_fixed(600.0),
                aspect_ratio: Some(1.5),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                ..default()
            },
            // The front's empty left margin; the lobby draws its controls above it.
            GlobalZIndex(if front { 1 } else { -1 }),
        ))
        .id();
    if let Some(art) = art {
        commands.entity(cloud).insert(MaterialNode(art.0.clone()));
    }
    let caption = commands
        .spawn((
            Text::new(Phrase::MemorialInvitation.text(state.lobby.lang())),
            crate::hud::tf(kit.fonts, kit.m.small),
            TextColor(Color::srgb(0.96, 0.92, 0.82)),
            TextLayout::justify(Justify::Center),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(100.0),
                padding: UiRect::axes(px_fixed(14.0), px_fixed(6.0)),
                border_radius: BorderRadius::all(px_fixed(20.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.07, 0.13, 0.18, 0.87)),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(cloud).add_child(caption);
    commands.entity(root).add_child(cloud);
    if state.memorial.open {
        album::draw(commands, root, state, kit, assets, art, scrolled);
    }
}

/// Every album close path restores its opener, even after the modal's nodes
/// have been rebuilt. Mouse users keep the ring hidden; keyboard users see it.
pub(super) fn return_focus(
    mut state: ResMut<LobbyState>,
    mut was_open: Local<bool>,
    clouds: Query<(Entity, &Stop), With<cloud::Cloud>>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
    mut remembered: ResMut<crate::shellkit::focus::Remembered>,
    mut kept: ResMut<super::focusing::Kept>,
) {
    if state.memorial.open {
        *was_open = true;
        return;
    }
    if !*was_open {
        return;
    }
    *was_open = false;
    if let Some((entity, stop)) = clouds.iter().next() {
        focus.set(entity, bevy::input_focus::FocusCause::Navigated);
        remembered.0 = Some(*stop);
        kept.keep(*stop);
        if matches!(state.lobby.screen(), Screen::SignIn { .. }) && !state.lobby.caret_parked() {
            state.lobby.park_caret();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shellkit::focus::Remembered;
    use bevy::input_focus::InputFocus;

    #[test]
    fn closing_the_album_restores_the_rebuilt_opener_and_parks_the_caret() {
        let mut app = App::new();
        app.init_resource::<LobbyState>()
            .init_resource::<InputFocus>()
            .init_resource::<Remembered>()
            .init_resource::<super::super::focusing::Kept>()
            .add_systems(Update, return_focus);
        app.world_mut().resource_mut::<LobbyState>().memorial.open = true;
        app.update();
        // A redraw gives the opener a fresh entity after the album closes.
        let stop = Stop::new("front-gateway", "baylee");
        let opener = app
            .world_mut()
            .spawn((cloud::Cloud { front: true }, stop))
            .id();
        app.world_mut().resource_mut::<LobbyState>().memorial.open = false;
        app.update();
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(opener));
        assert_eq!(app.world().resource::<Remembered>().0, Some(stop));
        assert!(app.world().resource::<LobbyState>().lobby.caret_parked());
    }
}
