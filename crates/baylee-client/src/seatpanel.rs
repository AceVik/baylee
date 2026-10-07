//! The language-model seat's settings panel, on the settings screen
//! (`docs/llm-seat.md`): the profiles a seat bridge (`baylee-seat`) plays,
//! the default among them, the caps across games and what the spend book
//! says was spent today and this month.
//!
//! Everything that decides is `baylee_client_core::llmseat::panel` and
//! tested there; the file and the book are its `desk`. This is the shell
//! round it: when the desk is opened and read again, what it is told the
//! time is, the keys typed into its boxes, and the drawing.
//!
//! On a desktop only, where the bridge runs. A browser and a phone have no
//! bridge beside them, and the settings screen says so in one line.
//!
//! A key never goes in the file. A key typed into any of the file's boxes
//! is refused beside it and never saved; the file holds the *name* of the
//! key's environment variable. Beside it is the key box: what is typed
//! there is drawn masked, with no eye to show it, and handed to the bridge
//! on its stdin (`baylee-seat key set`), which keeps it in the OS
//! credential store under that variable and the host the key goes to
//! (`docs/llm-seat.md` §"Where a key is kept"). Of a kept key the panel
//! knows only that it is kept: it is never read back.

use crate::hud::{UiFonts, palette, tf};
use crate::lobby::{
    FieldLook, Metrics, Press, button, chip, heading, note, panel, row, text_field,
};
use baylee_client_core::i18n::{Lang, Phrase};
#[cfg(not(target_arch = "wasm32"))]
use baylee_client_core::llmseat::keys::KeyEntry;
use baylee_client_core::llmseat::keys::{KeyDesk, KeyState};
use baylee_client_core::llmseat::panel::{Act, Disk, PanelFault, Saved, SeatPanel, Slot, Spot};
use baylee_client_core::llmseat::seating::{Preset, protocol_label};
use baylee_client_core::llmseat::{AnswerMode, CapField, Provider};
use bevy::prelude::*;
use bevy::ui::{percent, px};

/// Whether this build sets up the seat: a desktop, where the bridge runs.
pub(crate) const DESKTOP: bool = !cfg!(any(
    target_arch = "wasm32",
    target_os = "android",
    target_os = "ios"
));

/// How often, while the settings screen is up, the file and the book are
/// looked at again (their length and time only; a quiet look reads
/// neither).
const POLL_SECS: f32 = 1.0;

/// The panel, once the settings screen has opened it.
#[derive(Default)]
pub(crate) struct SeatDesk {
    #[cfg(not(target_arch = "wasm32"))]
    opened: Opened,
    /// The key boxes, and what the store was last heard to keep.
    keys: KeyDesk,
    /// The `baylee-seat key` jobs out.
    #[cfg(not(target_arch = "wasm32"))]
    runner: crate::seatbin::KeyRunner,
    /// A paste into the key box the clipboard has not answered.
    #[cfg(not(target_arch = "wasm32"))]
    key_paste: Option<bevy::clipboard::ClipboardRead>,
}

/// A press on a profile's key box.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum KeyPress {
    /// Put the caret in the key box of the profile at this place.
    Focus(usize),
    /// Hand the typed key to the store.
    Submit,
    /// Forget the key kept for the profile at this place.
    Delete(usize),
}

/// The provider's address variable, read from this program's environment
/// for a key's entry (the bridge it starts inherits it).
fn env_var(name: &str) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var(name).ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = name;
        None
    }
}

/// Whether the desk is open, and on what.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
enum Opened {
    /// Not yet, or never: a test, whose process may not touch the
    /// player's files ([`crate::settings::store_is_open`]).
    #[default]
    Not,
    /// This system names no configuration directory.
    Nowhere,
    /// The panel over the file, and a paste the clipboard has not answered.
    Desk(Box<Desk>),
}

#[cfg(not(target_arch = "wasm32"))]
struct Desk {
    desk: baylee_client_core::llmseat::desk::Desk,
    paste: Option<(Spot, bevy::clipboard::ClipboardRead)>,
}

impl SeatDesk {
    /// Opens the desk on the player's settings file, the first time the
    /// settings screen is up, and afterwards reads again whatever changed
    /// on disk. Whether anything drawn changed.
    pub(crate) fn poll(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            match &mut self.opened {
                Opened::Not if DESKTOP && crate::settings::store_is_open() => {
                    use baylee_client_core::llmseat::store::configured_path;
                    use baylee_client_core::userdirs::{Os, real_env};
                    // The file the bridges this client seats read: the one
                    // `BAYLEE_SEAT_CONFIG` names, else the config directory's.
                    self.opened = match configured_path(Os::current(), &real_env) {
                        Some(path) => Opened::Desk(Box::new(Desk {
                            desk: baylee_client_core::llmseat::desk::Desk::open(path, now()),
                            paste: None,
                        })),
                        None => Opened::Nowhere,
                    };
                    true
                }
                Opened::Desk(open) => open.desk.poll(now()),
                Opened::Not | Opened::Nowhere => false,
            }
        }
        #[cfg(target_arch = "wasm32")]
        false
    }

    /// Opens the desk on the settings file at `path`, as a test's scratch
    /// directory has it: a test never opens the player's own.
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) fn open_at(&mut self, path: std::path::PathBuf) {
        self.opened = Opened::Desk(Box::new(Desk {
            desk: baylee_client_core::llmseat::desk::Desk::open(path, now()),
            paste: None,
        }));
    }

    /// Whether the desk is open, so a poll is a look at the files rather
    /// than the opening of them.
    fn is_open(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            !matches!(self.opened, Opened::Not)
        }
        #[cfg(target_arch = "wasm32")]
        true
    }

    /// The panel, when there is one.
    #[must_use]
    pub(crate) fn panel(&self) -> Option<&SeatPanel> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Opened::Desk(open) = &self.opened {
            return Some(open.desk.panel());
        }
        None
    }

    /// Whether a box of the panel has the caret.
    #[must_use]
    pub(crate) fn typing(&self) -> bool {
        self.panel().is_some_and(SeatPanel::typing) || self.keys.typing().is_some()
    }

    /// The entry the shown profile keeps its key under, as its boxes say.
    #[cfg(not(target_arch = "wasm32"))]
    fn shown_entry(&self) -> Option<(usize, KeyEntry)> {
        let panel = self.panel()?;
        let at = panel.selected()?;
        Some((at, panel.key_entry(at, &env_var)?))
    }

    /// Takes what the key jobs came to, asks the store about the shown
    /// profile's entry once, and starts the jobs waiting. Whether anything
    /// drawn changed. Runs every frame the settings screen is up and costs
    /// a lock when nothing is out; no job runs in a test, which may not
    /// touch the player's store ([`crate::settings::store_is_open`]).
    pub(crate) fn pump(&mut self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut changed = false;
            for (entry, came) in self.runner.take() {
                self.keys.answered(&entry, came);
                changed = true;
            }
            if let Some((_, entry)) = self.shown_entry() {
                self.keys.look(&entry);
            }
            while let Some(job) = self.keys.next_job() {
                if crate::settings::store_is_open() {
                    self.runner.run(job);
                } else {
                    let entry = job.entry().clone();
                    self.keys
                        .answered(&entry, Ok(KeyState::Unavailable("not in a test".into())));
                    changed = true;
                }
            }
            changed
        }
        #[cfg(target_arch = "wasm32")]
        false
    }

    /// The key boxes, for a test to tell what the store answered.
    #[cfg(test)]
    pub(crate) fn keys_mut(&mut self) -> &mut KeyDesk {
        &mut self.keys
    }

    /// Does what a press on a key box asks.
    pub(crate) fn key_press(&mut self, press: KeyPress) {
        if let Some(panel) = self.panel_mut() {
            panel.blur();
        }
        match press {
            KeyPress::Focus(at) | KeyPress::Delete(at) => {
                let entry = self.panel().and_then(|panel| panel.key_entry(at, &env_var));
                if let Some(entry) = entry {
                    if matches!(press, KeyPress::Focus(_)) {
                        self.keys.focus(&entry);
                    } else {
                        self.keys.delete(&entry);
                    }
                }
            }
            KeyPress::Submit => self.keys.submit(),
        }
    }

    /// The panel, to change.
    fn panel_mut(&mut self) -> Option<&mut SeatPanel> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Opened::Desk(open) = &mut self.opened {
            return Some(open.desk.panel_mut());
        }
        None
    }

    /// Does what a press on the panel asks. [`Act::Save`] writes the file.
    pub(crate) fn act(&mut self, act: Act) {
        self.keys.blur();
        #[cfg(not(target_arch = "wasm32"))]
        if let Opened::Desk(open) = &mut self.opened {
            open.desk.act(act);
        }
        #[cfg(target_arch = "wasm32")]
        let _ = act;
    }

    /// Takes the caret out of the panel, when anything else is pressed.
    pub(crate) fn blur(&mut self) {
        self.keys.blur();
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.key_paste = None;
            if let Opened::Desk(open) = &mut self.opened {
                open.desk.panel_mut().blur();
                open.paste = None;
            }
        }
    }

    /// What the file is, to name it on the panel.
    fn path(&self) -> Option<String> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Opened::Desk(open) = &self.opened {
            return Some(open.desk.path().display().to_string());
        }
        None
    }

    /// Whether the system names no configuration directory.
    fn nowhere(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            matches!(self.opened, Opened::Nowhere)
        }
        #[cfg(target_arch = "wasm32")]
        false
    }
}

/// The moment the spend is read at: this machine's clock, and its offset
/// from UTC then, where the platform says (else the days are UTC's, and the
/// panel says so).
#[cfg(not(target_arch = "wasm32"))]
fn now() -> baylee_client_core::llmseat::ledger::Moment {
    let unix = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        });
    let offset = time::OffsetDateTime::from_unix_timestamp(unix)
        .ok()
        .and_then(|utc| time::UtcOffset::local_offset_at(utc).ok())
        .map(time::UtcOffset::whole_seconds);
    baylee_client_core::llmseat::ledger::Moment { unix, offset }
}

/// Opens the desk when the settings screen is up, and looks at the files
/// again once a second while it stays up.
///
/// Through `bypass_change_detection`, so a quiet look marks nothing
/// changed and the screen is not rebuilt for it.
pub(crate) fn poll(
    mut state: ResMut<crate::lobby::LobbyState>,
    time: Res<Time<Real>>,
    mut last: Local<Option<f32>>,
) {
    if !DESKTOP || !state.settings_open() {
        if last.take().is_some() {
            // Opened again later, every key is asked about anew: one may
            // have been kept from a terminal meanwhile.
            state.bypass_change_detection().seat.keys.forget();
        }
        return;
    }
    if state.bypass_change_detection().seat.pump() {
        state.set_changed();
    }
    let at = time.elapsed_secs();
    if state.seat.is_open() && last.is_some_and(|last| at - last < POLL_SECS) {
        return;
    }
    *last = Some(at);
    if state.bypass_change_detection().seat.poll() {
        state.set_changed();
    }
}

/// Types into the panel's box with the caret: the keys the lobby's other
/// boxes answer, `Tab` to the next box, `Enter` to save and `Escape` to
/// put the caret away. Touches the lobby only when a key was pressed or a
/// paste landed, so a quiet frame rebuilds nothing.
pub(crate) fn keys(
    keys: &mut MessageReader<bevy::input::keyboard::KeyboardInput>,
    codes: &ButtonInput<KeyCode>,
    state: &mut ResMut<crate::lobby::LobbyState>,
    clipboard: Option<&mut bevy::clipboard::Clipboard>,
) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        land_paste(state);
        if keys.is_empty() {
            return;
        }
        if !state.seat.typing() {
            keys.clear();
            return;
        }
        if state.seat.keys.typing().is_some() {
            type_key(keys, codes, &mut state.seat, clipboard);
            return;
        }
        let Opened::Desk(open) = &mut state.seat.opened else {
            keys.clear();
            return;
        };
        type_keys(keys, codes, open, clipboard);
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (codes, state, clipboard);
        keys.clear();
    }
}

/// Types into the key box: what the other boxes answer, but `Enter` hands
/// the key over and `Tab` and `Escape` drop it. Nothing typed here is
/// logged, and the box draws it masked.
#[cfg(not(target_arch = "wasm32"))]
fn type_key(
    keys: &mut MessageReader<bevy::input::keyboard::KeyboardInput>,
    codes: &ButtonInput<KeyCode>,
    seat: &mut SeatDesk,
    clipboard: Option<&mut bevy::clipboard::Clipboard>,
) {
    use baylee_client_core::textbuf::{Dir, Step, TextBuffer};
    use bevy::input::keyboard::Key;
    let shift = codes.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let command = codes.any_pressed([
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
    ]);
    let mut clipboard = clipboard;
    for key in keys.read() {
        if !key.state.is_pressed() || seat.keys.typing().is_none() {
            continue;
        }
        let desk = &mut seat.keys;
        match &key.logical_key {
            Key::Backspace => desk.edit(TextBuffer::delete_back),
            Key::Delete => desk.edit(TextBuffer::delete_forward),
            Key::ArrowLeft => desk.edit(|b| b.move_caret(Step::Char, Dir::Left, shift)),
            Key::ArrowRight => desk.edit(|b| b.move_caret(Step::Char, Dir::Right, shift)),
            Key::Home => desk.edit(|b| b.move_caret(Step::Line, Dir::Left, shift)),
            Key::End => desk.edit(|b| b.move_caret(Step::Line, Dir::Right, shift)),
            Key::Tab | Key::Escape => desk.blur(),
            Key::Enter => desk.submit(),
            Key::Character(text) if command => {
                if text.eq_ignore_ascii_case("a") {
                    desk.edit(TextBuffer::select_all);
                } else if text.eq_ignore_ascii_case("v")
                    && let Some(cb) = clipboard.as_deref_mut()
                {
                    let mut read = cb.fetch_text();
                    match read.poll_result() {
                        Some(Ok(text)) => desk.edit(|b| b.insert(text.trim())),
                        Some(Err(_)) => {}
                        None => seat.key_paste = Some(read),
                    }
                }
            }
            _ => {
                if let Some(text) = key.text.as_ref() {
                    desk.edit(|b| b.insert(text));
                }
            }
        }
    }
}

/// Lands a paste the clipboard has now answered, in the box it was asked
/// for while that box still has the caret.
#[cfg(not(target_arch = "wasm32"))]
fn land_paste(state: &mut ResMut<crate::lobby::LobbyState>) {
    if state.seat.key_paste.is_some() {
        let seat = &mut state.bypass_change_detection().seat;
        let answer = seat
            .key_paste
            .as_mut()
            .and_then(bevy::clipboard::ClipboardRead::poll_result);
        if let Some(answer) = answer {
            seat.key_paste = None;
            if let Ok(text) = answer
                && seat.keys.typing().is_some()
            {
                seat.keys.edit(|b| b.insert(text.trim()));
                state.set_changed();
            }
        }
    }
    let Opened::Desk(open) = &state.seat.opened else {
        return;
    };
    let Some((spot, _)) = &open.paste else {
        return;
    };
    let spot = *spot;
    let seat = &mut state.bypass_change_detection().seat;
    let Opened::Desk(open) = &mut seat.opened else {
        return;
    };
    let Some(answer) = open.paste.as_mut().and_then(|(_, read)| read.poll_result()) else {
        return;
    };
    open.paste = None;
    if let Ok(text) = answer
        && open.desk.panel().focus() == Some(spot)
    {
        open.desk.panel_mut().edit(|buffer| buffer.insert(&text));
        state.set_changed();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn type_keys(
    keys: &mut MessageReader<bevy::input::keyboard::KeyboardInput>,
    codes: &ButtonInput<KeyCode>,
    open: &mut Desk,
    mut clipboard: Option<&mut bevy::clipboard::Clipboard>,
) {
    use baylee_client_core::textbuf::{Dir, Step, TextBuffer};
    use bevy::input::keyboard::Key;
    // Read once for the batch, as the lobby's boxes do: a key event carries
    // no modifier state of its own.
    let shift = codes.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let word = codes.any_pressed([
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
    ]);
    let line = codes.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]);
    let command = line || codes.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let reach = if line {
        Step::Line
    } else if word {
        Step::Word
    } else {
        Step::Char
    };
    for key in keys.read() {
        if !key.state.is_pressed() || !open.desk.panel().typing() {
            continue;
        }
        let panel = open.desk.panel_mut();
        match &key.logical_key {
            Key::Backspace => panel.edit(TextBuffer::delete_back),
            Key::Delete => panel.edit(TextBuffer::delete_forward),
            Key::ArrowLeft => panel.edit(|b| b.move_caret(reach, Dir::Left, shift)),
            Key::ArrowRight => panel.edit(|b| b.move_caret(reach, Dir::Right, shift)),
            Key::Home => panel.edit(|b| b.move_caret(Step::Line, Dir::Left, shift)),
            Key::End => panel.edit(|b| b.move_caret(Step::Line, Dir::Right, shift)),
            Key::Tab => panel.tab(shift),
            Key::Escape => panel.blur(),
            Key::Enter => open.desk.act(Act::Save),
            Key::Character(text) if command => {
                if text.eq_ignore_ascii_case("a") {
                    panel.edit(TextBuffer::select_all);
                } else if text.eq_ignore_ascii_case("v")
                    && let Some(cb) = clipboard.as_deref_mut()
                    && let Some(spot) = panel.focus()
                {
                    let mut read = cb.fetch_text();
                    match read.poll_result() {
                        // `insert` drops the line breaks a copied value
                        // brings along.
                        Some(Ok(text)) => panel.edit(|b| b.insert(&text)),
                        Some(Err(_)) => {}
                        None => open.paste = Some((spot, read)),
                    }
                }
            }
            _ => {
                if let Some(text) = key.text.as_ref() {
                    panel.edit(|b| b.insert(text));
                }
            }
        }
    }
}

// ------------------------------------------------------------------ drawing

/// Whether the variable named `name` is set, and not blank, in this
/// program's environment — as the bridge's own reading of it
/// (`baylee-seat`'s `Secret::new`) takes it. Its value is never kept.
fn is_set(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| !value.to_string_lossy().trim().is_empty())
}

/// Draws the panel into the settings screen's scrolling column `root`.
pub(crate) fn draw(
    commands: &mut Commands,
    root: Entity,
    seat: &SeatDesk,
    lang: Lang,
    fonts: &UiFonts,
    metrics: Metrics,
) {
    if !DESKTOP {
        let line = note(commands, fonts, metrics, Phrase::SeatDesktopOnly.text(lang));
        commands.entity(root).add_child(line);
        return;
    }
    if !seat.is_open() {
        return;
    }
    let column = panel(commands, metrics, percent(100), 0.0);
    commands.entity(column).insert(Node {
        width: percent(100),
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Column,
        row_gap: px(metrics.gap * 0.8),
        padding: UiRect::all(px(metrics.pad * 1.5)),
        border_radius: BorderRadius::all(px(14)),
        border: UiRect::all(px(1)),
        ..default()
    });
    commands.entity(root).add_child(column);
    let title = heading(commands, fonts, metrics, Phrase::SeatPanelTitle.text(lang));
    let about = note(commands, fonts, metrics, Phrase::SeatPanelAbout.text(lang));
    commands.entity(column).add_children(&[title, about]);
    if seat.nowhere() {
        let line = note(commands, fonts, metrics, Phrase::SeatNoConfigDir.text(lang));
        commands.entity(column).add_child(line);
        return;
    }
    let (Some(panel), Some(path)) = (seat.panel(), seat.path()) else {
        return;
    };
    let file = note(
        commands,
        fonts,
        metrics,
        &Phrase::SeatFileAt.fill(lang, &[path.as_str()]),
    );
    commands.entity(column).add_child(file);
    let faults = panel.faults();
    let mut out = Drawn {
        commands,
        fonts,
        metrics,
        lang,
        panel,
        faults: &faults,
        keys: &seat.keys,
    };
    match panel.disk() {
        Disk::Refused(why) => {
            out.line(
                column,
                &Phrase::SeatFileRefused.fill(lang, &[why.as_str()]),
                palette::DANGER,
            );
        }
        Disk::Missing if panel.is_empty() => {
            out.line(column, Phrase::SeatEmpty.text(lang), palette::MUTED);
        }
        Disk::Read(_) | Disk::Missing => {}
    }
    if panel.newer_on_disk() {
        out.line(
            column,
            Phrase::SeatChangedOnDisk.text(lang),
            palette::ACCENT,
        );
    }
    if panel.editable() {
        out.profiles(column);
        if let Some(at) = panel.selected() {
            out.profile(column, at);
        }
        out.caps(column);
    }
    out.spent(column);
    if panel.editable() {
        out.footer(column);
    }
}

/// What one drawing of the panel draws with.
struct Drawn<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    fonts: &'a UiFonts,
    metrics: Metrics,
    lang: Lang,
    panel: &'a SeatPanel,
    faults: &'a [PanelFault],
    keys: &'a KeyDesk,
}

impl Drawn<'_, '_, '_> {
    /// A line of text in `ink`, under `parent`.
    fn line(&mut self, parent: Entity, words: &str, ink: Color) -> Entity {
        let id = self
            .commands
            .spawn((
                Text::new(words),
                tf(self.fonts, self.metrics.small),
                TextColor(ink),
                Pickable::IGNORE,
            ))
            .id();
        self.commands.entity(parent).add_child(id);
        id
    }

    /// A section's caption, as the keymap's groups are captioned.
    fn caption(&mut self, parent: Entity, words: &str) {
        self.line(parent, words, palette::ACCENT);
    }

    /// A wrapping row of cells.
    fn cells(&mut self, parent: Entity) -> Entity {
        let id = self
            .commands
            .spawn((
                Node {
                    width: percent(100),
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: px(self.metrics.gap),
                    row_gap: px(self.metrics.gap * 0.6),
                    align_items: AlignItems::FlexStart,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        self.commands.entity(parent).add_child(id);
        id
    }

    /// A cell of a row of cells: wide enough for a model id, and the whole
    /// width on a phone.
    fn cell(&mut self, parent: Entity) -> Entity {
        let phone = self.metrics.frame == crate::lobby::Frame::Compact;
        let id = self
            .commands
            .spawn((
                Node {
                    width: if phone { percent(100) } else { px(300) },
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        self.commands.entity(parent).add_child(id);
        id
    }

    /// Every fault shown at `spot`, in the fault's own ink, under `parent`.
    fn faults_at(&mut self, parent: Entity, spot: Spot) {
        let said: Vec<String> = self
            .faults
            .iter()
            .filter(|fault| fault.spot == spot)
            .map(|fault| self.panel.say(fault, self.lang))
            .collect();
        for words in said {
            self.line(parent, &words, palette::DANGER);
        }
    }

    /// The box at `spot`, its caption, what an empty one means, and every
    /// fault beside it.
    fn field(&mut self, parent: Entity, spot: Spot, label: &str) -> Entity {
        let cell = self.cell(parent);
        let Some(buffer) = self.panel.buffer(spot) else {
            return cell;
        };
        let hint = self.panel.hint(spot, self.lang);
        let look = FieldLook {
            buffer,
            focused: self.panel.focus() == Some(spot),
            mask: None,
            press: Press::Seat(Act::Focus(spot)),
            tail: None,
            lead: None,
            hint: hint.as_deref(),
        };
        let boxed = text_field(self.commands, self.fonts, self.metrics, label, &look);
        self.commands.entity(cell).add_child(boxed);
        self.faults_at(cell, spot);
        cell
    }

    /// A caption over a row of chips, one of them lit.
    fn choice(&mut self, parent: Entity, label: &str, chips: &[(&str, Press, bool)]) -> Entity {
        let cell = self.cell(parent);
        let caption = self
            .commands
            .spawn((
                Text::new(label),
                tf(self.fonts, self.metrics.small * 0.8),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        let line = row(self.commands, self.metrics, true);
        self.commands
            .entity(line)
            .entry::<Node>()
            .and_modify(|mut node| node.flex_wrap = FlexWrap::Wrap);
        for (words, press, on) in chips {
            let chip = chip(self.commands, self.fonts, self.metrics, words, *press, *on);
            self.commands.entity(line).add_child(chip);
        }
        self.commands.entity(cell).add_children(&[caption, line]);
        cell
    }

    /// The profiles as chips, the one shown lit, and Add.
    fn profiles(&mut self, parent: Entity) {
        let line = row(self.commands, self.metrics, true);
        self.commands
            .entity(line)
            .entry::<Node>()
            .and_modify(|mut node| node.flex_wrap = FlexWrap::Wrap);
        self.commands.entity(parent).add_child(line);
        for at in 0..self.panel.len() {
            let name = self.panel.name(at).unwrap_or_default();
            let words = if self.panel.default() == Some(at) {
                Phrase::SeatChipDefault.fill(self.lang, &[name])
            } else {
                name.to_string()
            };
            let on = self.panel.selected() == Some(at);
            let id = chip(
                self.commands,
                self.fonts,
                self.metrics,
                &words,
                Press::Seat(Act::Select(at)),
                on,
            );
            // A profile with something wrong in it says so from the list,
            // so a fault in a profile not shown is not a Save that is dead
            // for no reason on screen.
            if self
                .faults
                .iter()
                .any(|fault| matches!(fault.spot, Spot::Profile(p, _) if p == at))
            {
                self.commands
                    .entity(id)
                    .insert(BorderColor::all(palette::DANGER));
            }
            self.commands.entity(line).add_child(id);
        }
        let add = button(
            self.commands,
            self.fonts,
            self.metrics,
            Phrase::SeatAdd.text(self.lang),
            Press::Seat(Act::Add),
            palette::PANEL_LIT,
            true,
        );
        self.commands.entity(line).add_child(add);
        let presets: Vec<(&str, Press, bool)> = Preset::ALL
            .iter()
            .map(|preset| (preset.label(), Press::Seat(Act::AddPreset(*preset)), false))
            .collect();
        // A row of its own across the panel, not a cell of a form's row.
        let row_of_presets = self.choice(parent, Phrase::SeatPresets.text(self.lang), &presets);
        self.commands
            .entity(row_of_presets)
            .entry::<Node>()
            .and_modify(|mut node| node.width = percent(100));
        if self.panel.is_empty() && matches!(self.panel.disk(), Disk::Read(_)) {
            self.line(
                parent,
                Phrase::SeatNoProfiles.text(self.lang),
                palette::MUTED,
            );
        } else if !self.panel.is_empty() {
            let words = if self.panel.default().is_some() {
                Phrase::SeatDefaultAbout
            } else {
                Phrase::SeatDefaultNone
            };
            self.line(parent, words.text(self.lang), palette::MUTED);
        }
        self.faults_at(parent, Spot::Default);
    }

    /// The shown profile's boxes and choices.
    #[allow(clippy::too_many_lines)] // one form, read top to bottom
    fn profile(&mut self, parent: Entity, at: usize) {
        let lang = self.lang;
        let doors = row(self.commands, self.metrics, true);
        self.commands.entity(parent).add_child(doors);
        let is_default = self.panel.default() == Some(at);
        let default = chip(
            self.commands,
            self.fonts,
            self.metrics,
            Phrase::SeatDefault.text(lang),
            Press::Seat(Act::Default(at)),
            is_default,
        );
        let copy = button(
            self.commands,
            self.fonts,
            self.metrics,
            Phrase::SeatDuplicate.text(lang),
            Press::Seat(Act::Duplicate(at)),
            palette::PANEL_LIT,
            true,
        );
        let remove = button(
            self.commands,
            self.fonts,
            self.metrics,
            Phrase::SeatRemove.text(lang),
            Press::Seat(Act::Remove(at)),
            palette::DANGER,
            true,
        );
        self.commands
            .entity(doors)
            .add_children(&[default, copy, remove]);

        let spot = |slot| Spot::Profile(at, slot);
        let provider = self.panel.provider(at).unwrap_or(Provider::Anthropic);
        let first = self.cells(parent);
        self.field(first, spot(Slot::Name), Phrase::SeatName.text(lang));
        let protocols = [Provider::Anthropic, Provider::OpenAi, Provider::Cli].map(|p| {
            (
                protocol_label(p),
                Press::Seat(Act::Provider(at, p)),
                provider == p,
            )
        });
        self.choice(first, Phrase::SeatProvider.text(lang), &protocols);

        let model = self.cells(parent);
        let cell = self.field(model, spot(Slot::Model), Phrase::SeatModel.text(lang));
        let offered = self.panel.suggestions(at);
        if !offered.is_empty() {
            let line = row(self.commands, self.metrics, true);
            self.commands
                .entity(line)
                .entry::<Node>()
                .and_modify(|mut node| node.flex_wrap = FlexWrap::Wrap);
            self.commands.entity(cell).add_child(line);
            let chosen = self
                .panel
                .buffer(spot(Slot::Model))
                .map(|b| b.text().trim().to_string());
            for (index, name, price) in offered {
                let words = SeatPanel::suggestion(name, price, lang);
                let id = chip(
                    self.commands,
                    self.fonts,
                    self.metrics,
                    &words,
                    Press::Seat(Act::Suggest(at, index)),
                    chosen.as_deref() == Some(name),
                );
                self.commands.entity(line).add_child(id);
            }
            self.line(cell, Phrase::SeatPricedAbout.text(lang), palette::MUTED);
        }
        if let Some(note) = self.panel.model_note(at, lang) {
            self.line(cell, &note, palette::MUTED);
        }
        let answer = self.panel.answer(at);
        self.choice(
            model,
            Phrase::SeatAnswer.text(lang),
            &[
                (
                    Phrase::SeatAnswerBuild.text(lang),
                    Press::Seat(Act::Answer(at, None)),
                    answer.is_none(),
                ),
                (
                    Phrase::SeatAnswerTools.text(lang),
                    Press::Seat(Act::Answer(at, Some(AnswerMode::Tools))),
                    answer == Some(AnswerMode::Tools),
                ),
                (
                    Phrase::SeatAnswerJson.text(lang),
                    Press::Seat(Act::Answer(at, Some(AnswerMode::Json))),
                    answer == Some(AnswerMode::Json),
                ),
                (
                    Phrase::SeatAnswerJsonSchema.text(lang),
                    Press::Seat(Act::Answer(at, Some(AnswerMode::JsonSchema))),
                    answer == Some(AnswerMode::JsonSchema),
                ),
            ],
        );
        self.faults_at(model, spot(Slot::Answer));

        let play = self.cells(parent);
        for slot in [Slot::Effort, Slot::MaxTokens, Slot::ThinkSecs] {
            self.field(play, spot(slot), slot.label().text(lang));
        }

        let money = self.cells(parent);
        for slot in [
            Slot::PriceIn,
            Slot::PriceOut,
            Slot::GameUsd,
            Slot::GameTokens,
            Slot::GameCalls,
        ] {
            if self.panel.shows(spot(slot)) {
                self.field(money, spot(slot), slot.label().text(lang));
            }
        }
        let price = self.panel.price_note(at, lang);
        self.line(parent, &price, palette::MUTED);
        if let Some(warning) = self.panel.warning(at, lang) {
            self.line(parent, &warning, palette::ACCENT);
        }

        let reach = self.cells(parent);
        for slot in [Slot::BaseUrl, Slot::Command] {
            if self.panel.shows(spot(slot)) {
                self.field(reach, spot(slot), slot.label().text(lang));
            }
        }
        if self.panel.shows(spot(Slot::KeyEnv)) {
            let key = self.field(reach, spot(Slot::KeyEnv), Phrase::SeatKeyEnv.text(lang));
            if let Some((words, set)) = self.panel.key_line(at, &is_set, lang) {
                self.line(key, &words, if set { palette::INK } else { palette::MUTED });
            }
            self.key_box(reach, at);
        }
    }

    /// The key box of the profile at `at`: what the store keeps for its
    /// entry (never the key), a masked box to type one into, and Keep and
    /// Forget. Unavailable, with the reason, where this machine has no
    /// store the bridge can open; the variable keeps working everywhere.
    fn key_box(&mut self, parent: Entity, at: usize) {
        let lang = self.lang;
        let cell = self.cell(parent);
        let Some(entry) = self.panel.key_entry(at, &env_var) else {
            self.line(cell, Phrase::SeatKeyNoEntry.text(lang), palette::MUTED);
            return;
        };
        let state = self.keys.state(&entry).cloned();
        let busy = self.keys.busy(&entry);
        let host = entry.host().to_string();
        let (words, ink) = match (&state, busy) {
            (_, true) | (None, _) => (Phrase::SeatKeyAsking.text(lang).to_string(), palette::MUTED),
            (Some(KeyState::Set), _) => (Phrase::SeatKeyKept.fill(lang, &[&host]), palette::INK),
            (Some(KeyState::Absent), _) => {
                (Phrase::SeatKeyNoneKept.fill(lang, &[&host]), palette::MUTED)
            }
            (Some(KeyState::Unavailable(why)), _) => (
                Phrase::SeatKeyStoreUnavailable.fill(lang, &[why.as_str()]),
                palette::MUTED,
            ),
        };
        if matches!(state, Some(KeyState::Unavailable(_))) {
            self.line(cell, &words, ink);
            return;
        }
        let typing = self.keys.typing().filter(|(e, _)| **e == entry);
        let blank = baylee_client_core::textbuf::TextBuffer::default();
        let look = FieldLook {
            buffer: typing.map_or(&blank, |(_, buffer)| buffer),
            focused: typing.is_some(),
            mask: Some(crate::lobby::Masked::sealed()),
            press: Press::SeatKey(KeyPress::Focus(at)),
            tail: None,
            lead: None,
            hint: Some(if matches!(state, Some(KeyState::Set)) {
                Phrase::SeatKeyReplaceHint.text(lang)
            } else {
                Phrase::SeatKeyHint.text(lang)
            }),
        };
        let boxed = text_field(
            self.commands,
            self.fonts,
            self.metrics,
            Phrase::SeatKeyBox.text(lang),
            &look,
        );
        self.commands.entity(cell).add_child(boxed);
        let doors = row(self.commands, self.metrics, true);
        self.commands.entity(cell).add_child(doors);
        let keep = button(
            self.commands,
            self.fonts,
            self.metrics,
            Phrase::SeatKeyKeep.text(lang),
            Press::SeatKey(KeyPress::Submit),
            palette::ACCENT,
            typing.is_some_and(|(_, b)| !b.is_empty()) && !busy,
        );
        let forget = button(
            self.commands,
            self.fonts,
            self.metrics,
            Phrase::SeatKeyForget.text(lang),
            Press::SeatKey(KeyPress::Delete(at)),
            palette::DANGER,
            matches!(state, Some(KeyState::Set)) && !busy,
        );
        self.commands.entity(doors).add_children(&[keep, forget]);
        self.line(cell, &words, ink);
        if let Some(why) = self.keys.said(&entry) {
            let why = why.to_string();
            self.line(cell, &why, palette::DANGER);
        }
    }

    /// The caps across games.
    fn caps(&mut self, parent: Entity) {
        let lang = self.lang;
        self.caption(parent, Phrase::SeatCaps.text(lang));
        self.line(parent, Phrase::SeatCapsAbout.text(lang), palette::MUTED);
        let cells = self.cells(parent);
        for cap in CapField::ALL {
            let label = baylee_client_core::llmseat::panel::cap_label(cap).text(lang);
            self.field(cells, Spot::Cap(cap), label);
        }
    }

    /// What the book says was spent, and in which zone the days are
    /// counted.
    fn spent(&mut self, parent: Entity) {
        let lines = self.panel.spent_lines(self.lang);
        if lines.is_empty() {
            return;
        }
        self.caption(parent, Phrase::SeatSpent.text(self.lang));
        let unreadable = matches!(self.panel.spent(), Some(Err(_)));
        for words in lines {
            let ink = if unreadable {
                palette::DANGER
            } else {
                palette::INK
            };
            self.line(parent, &words, ink);
        }
    }

    /// Save, Discard, and how the last save went or why none can be made.
    fn footer(&mut self, parent: Entity) {
        let lang = self.lang;
        self.faults_at(parent, Spot::File);
        let line = row(self.commands, self.metrics, true);
        self.commands.entity(parent).add_child(line);
        let can_save = self.panel.to_save().is_some();
        let save = button(
            self.commands,
            self.fonts,
            self.metrics,
            Phrase::SeatSave.text(lang),
            Press::Seat(Act::Save),
            palette::ACCENT,
            can_save,
        );
        let can_revert = self.panel.changed() || self.panel.newer_on_disk();
        let revert = button(
            self.commands,
            self.fonts,
            self.metrics,
            Phrase::SeatRevert.text(lang),
            Press::Seat(Act::Revert),
            palette::PANEL_LIT,
            can_revert,
        );
        self.commands.entity(line).add_children(&[save, revert]);
        let status = match self.panel.last_save() {
            Some(Saved::Written) => {
                Some((Phrase::SeatSavedNote.text(lang).to_string(), palette::MUTED))
            }
            Some(Saved::Failed(why)) => Some((
                Phrase::SeatNotSaved.fill(lang, &[why.as_str()]),
                palette::DANGER,
            )),
            None if self.panel.changed() && !self.faults.is_empty() => {
                Some((Phrase::SeatFixFirst.text(lang).to_string(), palette::DANGER))
            }
            None if self.panel.changed() => {
                Some((Phrase::SeatUnsaved.text(lang).to_string(), palette::MUTED))
            }
            None => None,
        };
        if let Some((words, ink)) = status {
            self.line(line, &words, ink);
        }
    }
}
