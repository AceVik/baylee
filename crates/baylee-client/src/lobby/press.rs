//! What a click on a lobby control means: [`Press`], and the way from a
//! clicked entity to the control it belongs to.

use super::clicks::sign_out;
use super::systems::keep_gateways;
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

/// A component whose click means something.
///
/// One variant per screen, each carrying that screen's own presses, which
/// are defined and handled beside the screen that draws them.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Press {
    /// The front door.
    Front(FrontPress),
    /// The signed-in hub.
    Hub(HubPress),
    /// The deck library.
    Library(LibraryPress),
    /// A waiting room.
    Room(RoomPress),
    /// The deck builder.
    Build(BuildPress),
    /// The settings screen.
    Settings(SettingsPress),
    /// The end screen.
    End(EndPress),
    /// More than one screen.
    Shared(SharedPress),
    /// The shell's header and strips (WP0b-3).
    Header(super::header::HeaderPress),
    /// The Decks screen (WP3).
    Decks(super::decks::DecksPress),
    /// The Play screen and its sheets (WP2).
    Play(super::play::PlayPress),
}

/// A control more than one screen draws.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SharedPress {
    /// Put the caret in this field.
    Focus(Field),
    /// Show a masked field in the clear, or cover it again.
    Reveal(Field),
    /// Speak this language from now on.
    PickLang(Lang),
    ConfirmDestructive,
    CancelDestructive,
    /// Nothing. Carried by the picker's own panel so a tap inside it is
    /// not also a tap on the shade behind it, which would close it.
    PickerNothing,
    /// The front door's text row: the music on or off (this device's).
    ToggleMusic,
    /// Opens a menu from its `⋯` or caret; the open one closes it again.
    OpenMenu(super::menus::ShellMenu),
    /// The scrim behind an open menu: close it.
    CloseMenu,
}

/// What a press's handler may touch: the resources the `clicks` system
/// holds, lent as they are, so that a handler marks one changed only by
/// writing it (`a_press_that_changes_nothing_marks_nothing`).
pub(super) struct Cx<'a, 'l, 'p, 's, 'c> {
    pub(super) state: &'a mut ResMut<'l, LobbyState>,
    pub(super) prefs: &'a mut ResMut<'p, crate::prefs::Prefs>,
    pub(super) scrolled: &'a mut ResMut<'s, Scrolled>,
    pub(super) mailbox: &'a Mailbox,
    /// Absent in a headless test, which has no settings file to write to.
    pub(super) settings: &'a mut Option<ResMut<'c, crate::settings::ClientSettings>>,
}

impl SharedPress {
    /// What a click on this control does.
    pub(super) fn handle(self, cx: Cx<'_, '_, '_, '_, '_>) {
        let Cx {
            state,
            prefs,
            scrolled,
            mailbox,
            settings,
        } = cx;
        match self {
            SharedPress::ToggleMusic => {
                if let Some(settings) = settings.as_mut() {
                    settings.music.toggle();
                    settings.save();
                }
                // The text row says which; the lobby is drawn from its state.
                state.set_changed();
            }
            SharedPress::PickLang(lang) if lang == state.lobby.lang() => {}
            SharedPress::PickLang(lang) => {
                state.lobby.set_lang(lang);
                // One setting, two readers: the interface draws itself in
                // this language and the catalog is asked for card text in
                // it. Remembered at once, because the settings screen has
                // no way out but a click and a language that reverted on
                // the next launch would read as a button that did nothing.
                state.lang = lang.code().to_string();
                if state.lobby.builder().loaded() {
                    dispatch(state, mailbox, Some(LobbyRequest::LoadPool));
                }
                if let Some(settings) = settings.as_mut() {
                    settings.lang = lang.code().to_string();
                    settings.save();
                }
            }
            SharedPress::Focus(field) => state.lobby.focus_on(field),
            SharedPress::Reveal(field) => state.lobby.toggle_reveal(field),
            // An empty part of the artwork dialog dismisses its set
            // autocomplete.
            SharedPress::PickerNothing => state.lobby.builder_mut().picker_close_sets(),
            SharedPress::ConfirmDestructive
                if matches!(state.confirmation, Some(confirm::Destructive::SignOutGuest)) =>
            {
                state.confirmation = None;
                sign_out(state, prefs, scrolled, mailbox, settings);
            }
            SharedPress::ConfirmDestructive => {
                let forgetting = matches!(
                    state.confirmation,
                    Some(confirm::Destructive::ForgetGateway(_))
                );
                let request = confirm::accept(state);
                if forgetting && let Some(settings) = settings.as_mut() {
                    keep_gateways(state, settings);
                }
                dispatch(state, mailbox, request);
            }
            SharedPress::CancelDestructive => state.confirmation = None,
            SharedPress::OpenMenu(menu) => {
                state.menu = if state.menu == Some(menu) {
                    None
                } else {
                    Some(menu)
                };
            }
            SharedPress::CloseMenu => {
                if state.menu.is_some() {
                    state.menu = None;
                }
            }
        }
    }
}

/// Carries out a press, whatever made it: a click (`clicks`), a key on a
/// focused control (`focusing`), a shell key (`shortcuts`).
pub(super) fn run(press: Press, cx: Cx<'_, '_, '_, '_, '_>) {
    match press {
        Press::Front(press) => press.handle(cx),
        Press::Hub(press) => press.handle(cx),
        Press::Library(press) => press.handle(cx),
        Press::Room(press) => press.handle(cx),
        Press::Build(press) => press.handle(cx),
        Press::Settings(press) => press.handle(cx),
        // Game-over actions are handled by `leave_clicks`.
        Press::End(_) => {}
        Press::Shared(press) => press.handle(cx),
        Press::Header(press) => press.handle(cx),
        Press::Decks(press) => press.handle(cx),
        Press::Play(press) => press.handle(cx),
    }
}

/// The entity carrying the nearest [`Press`] at or above `entity`.
pub(super) fn in_lineage_entity(
    entity: Entity,
    presses: &Query<&Press>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    let mut current = Some(entity);
    while let Some(e) = current {
        if presses.contains(e) {
            return Some(e);
        }
        current = parents.get(e).ok().map(ChildOf::parent);
    }
    None
}

/// The nearest [`Press`] at or above an entity, so a click on a button's
/// label counts as a click on the button.
pub(super) fn in_lineage<'a>(
    entity: Entity,
    presses: &'a Query<&Press>,
    parents: &Query<&ChildOf>,
) -> Option<&'a Press> {
    let mut current = Some(entity);
    while let Some(e) = current {
        if let Ok(found) = presses.get(e) {
            return Some(found);
        }
        current = parents.get(e).ok().map(ChildOf::parent);
    }
    None
}
