//! The `?` overlay: every shell shortcut with its current key, searchable,
//! and runnable from the list (`KEYBOARD.md` §4.1, §7.10).
//!
//! A sheet in the sheet band over a scrim, `modal` in its [`TabOrder`]
//! (search → list → Edit keys… → Close), so Tab cycles inside it. It opens
//! on `?` or `Ctrl/Cmd+/` (and `Ctrl/Cmd+K` natively), focus lands in its
//! search, and the key that opened it is not typed there (the resolver
//! fires after the field editor has read the frame's keys, and the search
//! takes focus only once it exists). `↑↓` move the highlight from the
//! search or the list, Enter runs the highlighted action in the layer
//! beneath (the overlay closes first), Esc clears the search and then
//! closes, `Ctrl/Cmd+/` toggles it. Focus returns to whatever had it.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::shellkeys::{ShellAction, ShellKeymap};

use super::controls::{self, Kit, Live, Weight};
use super::focus::{Activated, ShellField, Stop, TabOrder};
use super::keys::{LearntKeys, ShellFired, mac};
use super::metrics::px_fixed;
use super::size::{InputClass, Platform, TextSize, Viewport};
use super::surfaces::{self, SheetWidth};
use super::{ShellMetrics, tokens};
use crate::hud::UiFonts;
use crate::settings::ClientSettings;

/// The overlay's focus order.
pub const OVERLAY_ORDER: TabOrder = TabOrder {
    name: "overlay",
    stops: &["search", "list", "edit-keys", "close"],
    modal: true,
};

/// Whether the overlay is up, and what it highlights.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Overlay {
    /// Shown.
    pub open: bool,
    /// The highlighted row among those the search leaves.
    pub cursor: usize,
    /// The search's text, mirrored from the field.
    pub query: String,
    /// What had focus when it opened; it gets it back.
    opener: Option<Entity>,
}

/// The overlay's root (the scrim).
#[derive(Component)]
pub struct OverlayRoot;

#[derive(Component)]
struct OverlaySearch;

#[derive(Component)]
struct OverlayList;

/// One row: the action it runs.
#[derive(Component, Clone, Copy)]
struct OverlayRow(ShellAction);

pub(super) fn install(app: &mut App) {
    app.init_resource::<Overlay>().add_systems(
        Update,
        (
            overlay_keys.before(super::focus::FocusSystems),
            (
                toggle,
                draw,
                focus_the_search,
                follow_the_search,
                draw_rows,
                clicks,
            )
                .chain()
                .after(super::keys::StackSystems)
                .after(super::focus::FocusSystems),
        ),
    );
}

/// The actions the overlay lists: those a key can reach here (Quick switch
/// is native only).
fn listed() -> impl Iterator<Item = ShellAction> {
    ShellAction::ALL
        .into_iter()
        .filter(|a| cfg!(not(target_arch = "wasm32")) || *a != ShellAction::QuickSwitch)
}

/// The rows the search leaves, in the overlay's order: an action matches
/// when its name or its key contains the search, case ignored.
#[must_use]
pub fn matching(
    query: &str,
    keymap: &ShellKeymap,
    lang: Lang,
    learnt: &LearntKeys,
) -> Vec<(ShellAction, String, String)> {
    let query = query.trim().to_lowercase();
    listed()
        .map(|a| {
            let keys = keymap
                .chords(a)
                .iter()
                .filter(|c| !c.mac || mac())
                .map(|c| c.display(mac(), &learnt.0))
                .collect::<Vec<_>>()
                .join(" · ");
            (a, a.label().text(lang).to_string(), keys)
        })
        .filter(|(_, name, keys)| {
            query.is_empty()
                || name.to_lowercase().contains(&query)
                || keys.to_lowercase().contains(&query)
        })
        .collect()
}

/// Opens and closes on the overlay's own actions.
fn toggle(
    mut fired: MessageReader<ShellFired>,
    mut overlay: ResMut<Overlay>,
    focus: Res<InputFocus>,
) {
    for ShellFired(action) in fired.read() {
        match action {
            ShellAction::Overlay if overlay.open => close(&mut overlay),
            ShellAction::Overlay | ShellAction::QuickSwitch if !overlay.open => {
                overlay.open = true;
                overlay.cursor = 0;
                overlay.query.clear();
                overlay.opener = focus.get();
            }
            _ => {}
        }
    }
}

fn close(overlay: &mut Overlay) {
    overlay.open = false;
    overlay.query.clear();
    overlay.cursor = 0;
}

/// Esc, the arrows and Enter while the overlay is up. Runs before the
/// field editor, so an Esc that finds text in the search leaves the
/// clearing to the field and closes nothing.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
fn overlay_keys(
    mut keys: MessageReader<KeyboardInput>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    settings: Option<Res<ClientSettings>>,
    learnt: Res<LearntKeys>,
    focus: Res<InputFocus>,
    stops: Query<&Stop>,
    search: Query<&ShellField, With<OverlaySearch>>,
    mut overlay: ResMut<Overlay>,
    mut fired: MessageWriter<ShellFired>,
) {
    if !overlay.open {
        keys.clear();
        return;
    }
    let lang = settings.as_deref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let standard = ShellKeymap::standard();
    let keymap = prefs.as_deref().map_or(&standard, |p| &p.all().shell_keys);
    let on = focus.get().and_then(|f| stops.get(f).ok()).map(|s| s.id);
    let in_search_or_list = matches!(on, Some("search" | "list"));
    let typed = search
        .iter()
        .next()
        .map(|f| f.value.clone())
        .unwrap_or_default();
    for key in keys.read() {
        if key.state != ButtonState::Pressed {
            continue;
        }
        let rows = matching(&typed, keymap, lang, &learnt);
        match &key.logical_key {
            Key::Escape if key.repeat => {}
            Key::Escape if on == Some("search") && !typed.is_empty() => {}
            Key::Escape => close(&mut overlay),
            Key::ArrowDown if in_search_or_list && !rows.is_empty() => {
                overlay.cursor = (overlay.cursor + 1).min(rows.len() - 1);
            }
            Key::ArrowUp if in_search_or_list => {
                overlay.cursor = overlay.cursor.saturating_sub(1);
            }
            Key::Home if on == Some("list") => overlay.cursor = 0,
            Key::End if on == Some("list") && !rows.is_empty() => overlay.cursor = rows.len() - 1,
            Key::Enter if in_search_or_list && !key.repeat => {
                if let Some((action, _, _)) = rows.get(overlay.cursor) {
                    let action = *action;
                    close(&mut overlay);
                    if !matches!(action, ShellAction::Overlay | ShellAction::QuickSwitch) {
                        fired.write(ShellFired(action));
                    }
                }
            }
            _ => {}
        }
    }
}

/// Builds the sheet when it opens and takes it down when it closes, giving
/// focus back to the opener.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
fn draw(
    mut commands: Commands,
    overlay: Res<Overlay>,
    settings: Option<Res<ClientSettings>>,
    input: Res<InputClass>,
    fonts: Option<Res<UiFonts>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    roots: Query<Entity, With<OverlayRoot>>,
    alive: Query<()>,
    mut focus: ResMut<InputFocus>,
) {
    if !overlay.open {
        if !roots.is_empty() {
            for root in &roots {
                commands.entity(root).despawn();
            }
            match overlay.opener.filter(|e| alive.contains(*e)) {
                Some(opener) => focus.set(opener, FocusCause::Navigated),
                None => focus.clear(),
            }
        }
        return;
    }
    if !roots.is_empty() {
        return;
    }
    let Some(fonts) = fonts else {
        return;
    };
    let (width, height) = windows
        .single()
        .map_or((1280.0, 800.0), |w| (w.width(), w.height()));
    let lang = settings.as_deref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let step = settings.as_deref().map_or(TextSize::L, |s| s.text_size);
    let kit = Kit {
        fonts: &fonts,
        m: ShellMetrics::of(
            Viewport {
                width,
                height,
                platform: Platform::current(),
                input: *input,
            },
            step,
        ),
        german: lang == Lang::De,
    };
    let search = controls::search(
        &mut commands,
        kit,
        "",
        Phrase::ShellOverlaySearch.text(lang),
        (
            OverlaySearch,
            Stop::new(OVERLAY_ORDER.name, "search"),
            ShellField::new("", Phrase::ShellOverlaySearch.text(lang)),
        ),
    );
    let list = commands
        .spawn((
            OverlayList,
            Stop::new(OVERLAY_ORDER.name, "list"),
            Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                row_gap: px_fixed(2.0),
                ..default()
            },
        ))
        .id();
    let edit = controls::button(
        &mut commands,
        kit,
        Phrase::ShellOverlayEditKeys.text(lang),
        Weight::Ghost,
        Live::Yes,
        None,
        Stop::new(OVERLAY_ORDER.name, "edit-keys"),
    );
    let close = controls::button(
        &mut commands,
        kit,
        Phrase::ShellClose.text(lang),
        Weight::Secondary,
        Live::Yes,
        None,
        Stop::new(OVERLAY_ORDER.name, "close"),
    );
    let surface = surfaces::sheet_box(
        &mut commands,
        kit,
        SheetWidth::Medium,
        Phrase::ShellKeyOverlay.text(lang),
        &[search, list],
        &[edit, close],
    );
    let root = surfaces::sheet(&mut commands, surface);
    commands.entity(root).insert(OverlayRoot);
}

/// The search takes focus once it exists.
fn focus_the_search(added: Query<Entity, Added<OverlaySearch>>, mut focus: ResMut<InputFocus>) {
    if let Some(search) = added.iter().next() {
        focus.set(search, FocusCause::Navigated);
    }
}

/// The highlight goes back to the top when the search changes.
fn follow_the_search(
    search: Query<&ShellField, (With<OverlaySearch>, Changed<ShellField>)>,
    mut overlay: ResMut<Overlay>,
) {
    if let Some(field) = search.iter().next()
        && field.value != overlay.query
    {
        overlay.query.clone_from(&field.value);
        overlay.cursor = 0;
    }
}

/// The rows: rebuilt when the search, the highlight, the keys or the
/// language change, and when the list first appears.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
fn draw_rows(
    mut commands: Commands,
    overlay: Res<Overlay>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    settings: Option<Res<ClientSettings>>,
    learnt: Res<LearntKeys>,
    input: Res<InputClass>,
    fonts: Option<Res<UiFonts>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    lists: Query<(Entity, Option<&Children>), With<OverlayList>>,
    added: Query<(), Added<OverlayList>>,
) {
    let Ok((list, children)) = lists.single() else {
        return;
    };
    let stale = !added.is_empty()
        || overlay.is_changed()
        || learnt.is_changed()
        || prefs.as_ref().is_some_and(DetectChanges::is_changed)
        || settings.as_ref().is_some_and(DetectChanges::is_changed);
    if !stale {
        return;
    }
    let Some(fonts) = fonts else {
        return;
    };
    if let Some(children) = children {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }
    let (width, height) = windows
        .single()
        .map_or((1280.0, 800.0), |w| (w.width(), w.height()));
    let lang = settings.as_deref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let step = settings.as_deref().map_or(TextSize::L, |s| s.text_size);
    let kit = Kit {
        fonts: &fonts,
        m: ShellMetrics::of(
            Viewport {
                width,
                height,
                platform: Platform::current(),
                input: *input,
            },
            step,
        ),
        german: lang == Lang::De,
    };
    let standard = ShellKeymap::standard();
    let keymap = prefs.as_deref().map_or(&standard, |p| &p.all().shell_keys);
    let rows = matching(&overlay.query, keymap, lang, &learnt);
    if rows.is_empty() {
        let none = surfaces::prose(
            &mut commands,
            kit,
            Phrase::ShellOverlayNoMatch.text(lang),
            true,
        );
        commands.entity(list).add_child(none);
        return;
    }
    for (i, (action, name, keys)) in rows.iter().enumerate() {
        let row = surfaces::list_row(&mut commands, kit, name, keys, &[], OverlayRow(*action));
        if i == overlay.cursor {
            commands
                .entity(row)
                .insert(BackgroundColor(tokens::SELECTED));
        }
        commands.entity(list).add_child(row);
    }
}

/// A click on a row runs it; Edit keys… opens Settings; Close closes.
fn clicks(
    mut activated: MessageReader<Activated>,
    mut pointer: MessageReader<Pointer<Click>>,
    rows: Query<&OverlayRow>,
    parents: Query<&ChildOf>,
    mut overlay: ResMut<Overlay>,
    mut fired: MessageWriter<ShellFired>,
) {
    for Activated { stop, .. } in activated.read() {
        if stop.table != OVERLAY_ORDER.name {
            continue;
        }
        match stop.id {
            "close" => close(&mut overlay),
            "edit-keys" => {
                close(&mut overlay);
                fired.write(ShellFired(ShellAction::OpenSettings));
            }
            _ => {}
        }
    }
    if !overlay.open {
        pointer.clear();
        return;
    }
    for click in pointer.read() {
        let mut at = Some(click.entity);
        while let Some(e) = at {
            if let Ok(OverlayRow(action)) = rows.get(e) {
                let action = *action;
                close(&mut overlay);
                if !matches!(action, ShellAction::Overlay | ShellAction::QuickSwitch) {
                    fired.write(ShellFired(action));
                }
                return;
            }
            at = parents.get(e).ok().map(ChildOf::parent);
        }
    }
}

/// Whether the overlay holds the keyboard (the lobby's own key handling
/// stands aside while it is up).
#[must_use]
pub fn holds(overlay: &Overlay) -> bool {
    overlay.open
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_search_narrows_by_name_or_by_key() {
        let map = ShellKeymap::standard();
        let learnt = LearntKeys::default();
        let all = matching("", &map, Lang::En, &learnt);
        assert!(all.len() >= 20, "{}", all.len());
        let text = matching("text", &map, Lang::En, &learnt);
        assert_eq!(
            text.iter().map(|r| r.0).collect::<Vec<_>>(),
            [
                ShellAction::TextLarger,
                ShellAction::TextSmaller,
                ShellAction::TextReset
            ]
        );
        let german = matching("deck", &map, Lang::De, &learnt);
        assert!(german.iter().any(|r| r.0 == ShellAction::SaveDeck));
        assert!(matching("zzz-nothing", &map, Lang::En, &learnt).is_empty());
    }
}
