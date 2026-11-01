//! The settings over a table (report of 30.10.2026: the settings should be
//! reachable at the table, not only from the lobby).
//!
//! The lobby's own tree is not drawn while a table stands, so the screen the
//! lobby stands over itself (`settingsui::screen`) is drawn here in a root of
//! its own over the table, as the report form is (`report::form`): the game
//! menu's row asks for it (`Duel::settings_asked`, [`open_asked`]), and its
//! Back control or `Esc` ([`close_on_escape`]) puts it away. While it stands
//! it has the pointer (the root blocks what is under it) and the keys (the
//! table's `KeyClaims` asks [`LobbyState::settings_over_table`]), and the
//! lobby's clicks, typing, wheel and the screen's own key systems run for it
//! ([`lobby_or_over_table`]). The lobby's own screens stay undrawn.
//!
//! Only the sections that leave the table where it is are offered
//! (`Section::at_table`); the screen is put down with the table
//! ([`put_away`]).

use super::scrolling::{List, Scrolled};
#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use baylee_client_core::settings_map::Section;
use bevy::ui::FocusPolicy;

/// The root the settings stand in over a table.
#[derive(Component)]
pub(crate) struct OverTable;

/// Where the root stands: over the table's HUD, whose roots stand at the
/// page and a few steps above it (the spectators' line, a finished game's
/// preview), and under the screen's own popovers and sheets
/// (`tokens::z::POPOVER` and up), which stand at the global level wherever
/// they are drawn: a root any higher would cover its own dropdowns.
const ROOT_Z: i32 = crate::shellkit::tokens::z::POPOVER - 2;

/// Whether the settings stand over a table now.
pub(crate) fn over_table(state: Option<Res<LobbyState>>) -> bool {
    state.is_some_and(|s| s.settings_over_table())
}

/// The run condition of the lobby systems the settings screen needs: the
/// lobby is up, or the settings stand over a table.
pub(crate) fn lobby_or_over_table(
    state: Option<Res<LobbyState>>,
    phase: Option<Res<State<DuelPhase>>>,
) -> bool {
    phase.is_some_and(|p| *p.get() == DuelPhase::Closed) || over_table(state)
}

/// The game menu's row asked for the settings: open them over the table,
/// and shut the menu under them, as the report form does with its row.
pub(crate) fn open_asked(duel: Option<ResMut<crate::Duel>>, mut state: ResMut<LobbyState>) {
    // Read before anything is borrowed mutably: a quiet frame writes nothing.
    let Some(mut duel) = duel.filter(|d| d.settings_asked) else {
        return;
    };
    duel.settings_asked = false;
    duel.game_menu = false;
    stand_over_table(&mut state);
}

/// Opens the settings over a table, on a section offered there: the one
/// last shown, or the first the table keeps.
pub(crate) fn stand_over_table(state: &mut LobbyState) {
    state.settings_view.over_table = true;
    if !state.settings.is_open() {
        state.settings = SettingsPane::Open;
    }
    if !state.settings_section().at_table() {
        let first = Section::ALL
            .into_iter()
            .find(|s| s.at_table())
            .unwrap_or_default();
        state.set_settings_section(first);
    }
}

/// `Esc` puts the settings over a table away, where nothing inside them
/// answers it first: a key being captured, a profile's sheet and a search
/// with words in it each take their own `Esc` in the lobby's keyboard,
/// which runs after this.
///
/// The key is taken off both of its inputs, so whichever side of this the
/// table's keyboard runs on, it never hears it: before, the settings still
/// held the keys; after, there is no `Esc` left to hear.
pub(crate) fn close_on_escape(
    mut codes: ResMut<ButtonInput<KeyCode>>,
    logical: Option<ResMut<ButtonInput<Key>>>,
    mut state: ResMut<LobbyState>,
) {
    if !codes.just_pressed(KeyCode::Escape) || !state.settings_over_table() {
        return;
    }
    let inner = state.settings.capturing().is_some()
        || state.settings.capturing_shell().is_some()
        || state.settings_view.profile_sheet
        || (state.lobby.focus() == Field::SettingsSearch && !state.settings_query().is_empty());
    if inner {
        return;
    }
    codes.clear_just_pressed(KeyCode::Escape);
    if let Some(mut logical) = logical {
        logical.clear_just_pressed(Key::Escape);
    }
    close(&mut state);
}

/// Puts the settings over a table away, as its Back control does; the
/// table plays on.
pub(crate) fn close(state: &mut LobbyState) {
    if state.settings.is_open() {
        state.settings = SettingsPane::Closed;
    }
}

/// The table is put down (it finished, or the player left it): settings
/// standing over it close with it, and the lobby comes back on the screen
/// it left rather than on them.
pub(crate) fn put_away(mut state: ResMut<LobbyState>) {
    if !state.settings_view.over_table {
        return;
    }
    state.settings_view.over_table = false;
    close(&mut state);
}

/// What the screen was last drawn for: the window in logical pixels, the
/// UI scale and the text step.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Drawn {
    window: Vec2,
    ui: f32,
    step: crate::shellkit::TextSize,
    input: crate::shellkit::InputClass,
}

/// Draws the settings over the table while they are open there, and takes
/// them away when they close.
///
/// Rebuilt as the lobby rebuilds its own: when the lobby's state, the
/// account's preferences or this device's settings changed, or the window
/// changed its size or scale. Measured as the report form is
/// (`ShellMetrics::under`): the table's HUD already draws `UiScale` times
/// larger, and the lobby's own measure would scale the screen twice.
#[allow(clippy::too_many_arguments)] // a Bevy system: each is what it draws from
pub(crate) fn draw(
    mut commands: Commands,
    state: Res<LobbyState>,
    prefs: Res<crate::prefs::Prefs>,
    scrolled: Res<Scrolled>,
    fonts: Option<Res<UiFonts>>,
    (windows, ui): (Query<&Window>, Option<Res<UiScale>>),
    kit_inputs: shell::KitInputs,
    roots: Query<Entity, With<OverTable>>,
    mut drawn: Local<Option<Drawn>>,
) {
    if !state.settings_over_table() {
        if !roots.is_empty() {
            for root in &roots {
                commands.entity(root).despawn();
            }
        }
        *drawn = None;
        return;
    }
    let Some(fonts) = fonts else {
        return;
    };
    let window = windows.iter().next();
    let size = window.map_or(Vec2::new(1280.0, 800.0), |w| {
        Vec2::new(w.width(), w.height())
    });
    let ui_scale = ui.as_deref().map_or(1.0, |u| u.0);
    let settings = kit_inputs.1.as_deref();
    let step = settings.map_or_else(Default::default, |s| s.text_size);
    let now = Drawn {
        window: size,
        ui: ui_scale,
        step,
        input: *kit_inputs.0,
    };
    let settings_changed = kit_inputs.1.as_ref().is_some_and(DetectChanges::is_changed);
    if !state.is_changed()
        && !prefs.is_changed()
        && !settings_changed
        && !roots.is_empty()
        && *drawn == Some(now)
    {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
    let shell_m = crate::shellkit::ShellMetrics::under(
        crate::shellkit::Viewport {
            width: size.x,
            height: size.y,
            platform: crate::shellkit::Platform::current(),
            input: *kit_inputs.0,
        },
        step,
        ui_scale,
    );
    // The window in the HUD's units, which is what the root spans.
    let space = window.map_or(size, |w| crate::hud::scale::space(w, ui.as_deref()));
    let metrics = Metrics::of(space.x).at_scale(shell_m.scale);
    let kit = crate::shellkit::controls::Kit {
        fonts: &fonts,
        m: shell_m,
        german: state.lobby.lang() == Lang::De,
    };
    let root = commands
        .spawn((
            OverTable,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            // The table stays in sight, dimmed: the panels carry their own
            // ground, and a graphics knob turned here shows on the table.
            BackgroundColor(crate::shellkit::tokens::SCRIM),
            FocusPolicy::Block,
            GlobalZIndex(ROOT_Z),
        ))
        .id();
    let (_, _, in_use, trial, monitors) = &kit_inputs;
    let view = crate::settingsui::View {
        state: &state,
        prefs: prefs.all(),
        settings,
        graphics: crate::settingsui::keys::shown_graphics(settings, in_use.as_deref()),
        trial: trial
            .as_deref()
            .and_then(|t| t.previous.map(|_| t.seconds())),
        monitors: monitors.iter().count(),
        builds: crate::settingsui::builds(),
        metrics,
        scroll: scrolled.get(List::Settings),
        sheet_scroll: scrolled.get(List::ProfileSheet),
        nav_scroll: scrolled.get(List::SettingsNav),
    };
    crate::settingsui::screen(&mut commands, root, &view, kit);
    *drawn = Some(now);
}
