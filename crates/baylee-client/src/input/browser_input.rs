//! The zone browser's keyboard and soft keys.

#[allow(clippy::wildcard_imports)] // the input module's shared vocabulary
use super::*;

/// Browsing starts in search; a small complete target offer starts on its first row.
///
/// Every application with a search field does this, and §6's reading of the
/// dialog is the same one — *„ein Ort, an dem man arbeitet"*. It was the
/// other way round until the owner paid for it: the box took the keyboard
/// only on a click in it, so a search term typed into a panel that had just
/// been opened was fifteen bound letters fired at the table instead. One of
/// them (`T`) latched the text view on and *persisted* it, which is how every
/// card in a duel came to be drawn as its own rules text; `K`/`B` and `Y`/`N`
/// reach the engine, and there is no undo there.
///
/// The player takes the keyboard back with `Esc` or `Enter` and the sheet
/// goes on standing — that half of the bargain is untouched, and it is what
/// lets a graveyard stay open through a turn.
///
/// Two things it will not do. **Not on a platform that owns the typing**,
/// because there `is_typing` is what raises a phone's keyboard
/// ([`browser_softkeys`]) and a tap on a pile would put it over the graveyard
/// the player meant to read. And **not for an ordering**, which draws no
/// filter box at all (`hud::tray`: the row says what to do instead) — a
/// keyboard handed to a field that is not on screen is a keyboard nobody can
/// get back.
pub fn browser_takes_the_keyboard(mut duel: ResMut<Duel>, mut was_open: Local<bool>) {
    let open = duel.browser.is_open();
    let opening = open && !*was_open;
    *was_open = open;
    if open
        && duel.view.as_ref().is_some_and(|view| {
            duel.browser
                .compact_targets(view, duel.interaction.as_ref())
        })
    {
        duel.browser.stop_typing();
        return;
    }
    if !opening || crate::softkeys::SoftKeyboard::owns_typing() {
        return;
    }
    if duel
        .interaction
        .as_ref()
        .is_some_and(Interaction::is_ordering)
    {
        return;
    }
    duel.browser.start_typing();
}

/// The platform's own text input, pointed at the browser's filter box.
///
/// Only the browser has one, and it is the only thing that raises a phone's
/// keyboard — a canvas never does. The lobby's form does this for its fields;
/// the table has exactly one field, and without this the pile a player wanted
/// to search was searchable on a desktop and not on the device the tray's
/// scrolling and 44-pixel targets were sized for.
///
/// Focus is an *edge*: `typing_epoch` counts how many times the box has been
/// given the keyboard, because pointing the input at the field on every frame
/// would fight the player for the caret.
pub fn browser_softkeys(
    mut keys: ResMut<crate::softkeys::SoftKeyboard>,
    mut duel: ResMut<Duel>,
    mut epoch: Local<u64>,
    mut had_focus: Local<bool>,
) {
    if !crate::softkeys::SoftKeyboard::owns_typing() {
        return;
    }
    let typing = duel.browser.is_typing();
    if typing && *epoch != duel.browser.typing_epoch() {
        *epoch = duel.browser.typing_epoch();
        keys.open(
            baylee_client_core::lobby::FieldKind::Name,
            duel.browser.filter(),
        );
    }
    if !typing {
        if *had_focus {
            keys.close();
        }
        *had_focus = false;
        return;
    }
    *had_focus = true;
    for key in keys.drain() {
        match key {
            // Not a keystroke: autofill and paste arrive as a whole value,
            // and with the caret and selection the element is holding — the
            // `<input>` owns both, and the box draws both now.
            crate::softkeys::SoftKey::Text {
                value,
                cursor,
                anchor,
            } => duel.browser.set_filter_state(&value, cursor, anchor),
            // A caret that moved inside unchanged text is exactly what the
            // arrow keys do on a page, and it used to change nothing here
            // because there was no caret to move.
            crate::softkeys::SoftKey::Caret { cursor, anchor } => {
                duel.browser.place_filter_caret(cursor, anchor);
            }
            // Nothing to submit — the rows are already narrowed, so the
            // action key means "done".
            crate::softkeys::SoftKey::Submit => {
                duel.browser.stop_typing();
                keys.close();
            }
            // Escape, arriving the long way round because the `<input>` has
            // the focus and the canvas never sees the key. Same two steps as
            // the native path in `browser_keys`: empty the box first, let go
            // of it second — `clear_filter` bumps the epoch, so the next
            // frame points the field at the emptied value rather than
            // leaving the old letters on screen.
            crate::softkeys::SoftKey::Dismiss => {
                if duel.browser.filter().is_empty() {
                    duel.browser.stop_typing();
                    keys.close();
                } else {
                    duel.browser.clear_filter();
                }
            }
        }
    }
}

/// Typing into the zone browser's filter box.
///
/// The panel could sort and scroll and the one thing the owner asked for by
/// name — a graveyard you can *search* — had no way in: `Browser::set_filter`
/// was written and nothing ever called it.
///
/// The box holds the keyboard only while it has been given it, which is what
/// lets the panel stay open through a turn. `Cancel` is the way out and
/// empties the box first when there is anything in it, so one press undoes
/// the search and the next one lets go — a player who typed `mou` and found
/// nothing should not have to rub out three letters to get back to the pile.
///
/// What it reads is what `lobby::systems::text_field_keys` reads, chord for
/// chord, because the owner named the lobby's boxes as the thing this one
/// should be: a caret that moves by character, word and line, a selection
/// shift extends, Delete beside Backspace, and select-all. It was a string
/// with characters pushed onto the end of it and popped off again, which is
/// none of those. Tab is the one chord the lobby has and this does not —
/// there is no second field on a zone sheet to move to.
pub(super) fn browser_keys(
    fired: Fired,
    codes: &ButtonInput<KeyCode>,
    typed: &mut MessageReader<KeyboardInput>,
    duel: &mut Duel,
) -> bool {
    use baylee_client_core::textbuf::{Dir, Step};
    // Two fields and one chord table. While the builder holds a caret in one
    // of its rows, every key below goes there instead of into the box — the
    // two are editors of one string and only one of them may be typed into,
    // which is the rule `filterdialog`'s own header states. Written as a
    // target rather than as a second copy of the table: a chord added to one
    // field and not the other is how the lobby's boxes and this one drifted
    // apart in the first place.
    let target = if duel
        .browser
        .builder()
        .is_some_and(|it| it.typing().is_some())
    {
        Caret::Row
    } else if duel.browser.is_typing() {
        Caret::Box
    } else {
        return false;
    };
    // `Cancel` is read *before* the platform bail below, and that ordering is
    // the whole of it: where the browser does the typing every raw key
    // belongs to its `<input>`, so returning first would leave Escape doing
    // nothing at all on a page — not emptying the box, not letting go of it,
    // with only the soft keyboard's own action key as a way out. `fired`
    // carries actions rather than raw keys, so reading it here cannot type a
    // character the `<input>` has already taken.
    if fired.has(Action::Cancel) {
        match target {
            // In a row, Escape gives the row back and leaves the builder
            // open: the way out of the *builder* is its own gear, and a key
            // that shut both would make a mistyped letter cost the panel.
            Caret::Row => duel.browser.in_builder(FilterPanel::stop_typing),
            Caret::Box if duel.browser.filter().is_empty() => duel.browser.stop_typing(),
            Caret::Box => duel.browser.clear_filter(),
        }
        return true;
    }
    // The box still owns the keyboard where the platform does the typing —
    // `browser_softkeys` has already read the value — but the client must not
    // read the raw keys as well, or every character is entered twice.
    if crate::softkeys::SoftKeyboard::owns_typing() {
        return true;
    }
    // The three modifiers a text field reads, once for the whole batch
    // because a key event carries no modifier state of its own. Which one
    // means what is the platform's convention rather than a preference, and
    // it is the lobby's: shift extends a selection, and the reaches past a
    // single character are ⌥/Ctrl for a word and ⌘/Home-End for the line.
    let shift = codes.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let word = codes.any_pressed([
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
    ]);
    let line = codes.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]);
    // ⌘A and Ctrl+A, answered before the text arm so the "a" stays out of the
    // box.
    let command = line || codes.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let reach = if line {
        Step::Line
    } else if word {
        Step::Word
    } else {
        Step::Char
    };
    for event in typed.read() {
        if !event.state.is_pressed() {
            continue;
        }
        let gesture = match &event.logical_key {
            Key::Backspace => Gesture::Back,
            Key::Delete => Gesture::Forward,
            Key::ArrowLeft => Gesture::Move(reach, Dir::Left, shift),
            Key::ArrowRight => Gesture::Move(reach, Dir::Right, shift),
            Key::Home => Gesture::Move(Step::Line, Dir::Left, shift),
            Key::End => Gesture::Move(Step::Line, Dir::Right, shift),
            // "Done" rather than "submit": the rows are already narrowed, so
            // the only thing left to do is hand the keyboard back.
            Key::Enter => Gesture::Done,
            Key::Character(s) if command => {
                if s.eq_ignore_ascii_case("a") {
                    Gesture::SelectAll
                } else {
                    continue;
                }
            }
            Key::Character(s) => Gesture::Insert(s.to_string()),
            Key::Space => Gesture::Insert(" ".to_string()),
            _ => continue,
        };
        gesture.done(target, duel);
    }
    true
}

/// Which of the two fields the keyboard is reaching.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Caret {
    /// The search box itself.
    Box,
    /// A row of the filter builder.
    Row,
}

/// One editing gesture, before it is aimed at a field.
///
/// The chords are read once and the field is chosen once, which is the point:
/// the box and a builder row take the same keys because they are the same
/// kind of thing, and a table written twice is a table that agrees until
/// somebody adds a chord to one half of it.
enum Gesture {
    /// Backspace.
    Back,
    /// Delete.
    Forward,
    /// An arrow, Home or End.
    Move(
        baylee_client_core::textbuf::Step,
        baylee_client_core::textbuf::Dir,
        bool,
    ),
    /// ⌘A.
    SelectAll,
    /// Enter: hand the keyboard back.
    Done,
    /// A character, or a whole run of them from an IME or a paste.
    Insert(String),
}

impl Gesture {
    /// Does it, to whichever field holds the caret.
    fn done(self, target: Caret, duel: &mut Duel) {
        match (target, self) {
            (Caret::Box, Self::Back) => {
                duel.browser.pop_filter();
            }
            (Caret::Box, Self::Forward) => duel.browser.delete_forward(),
            (Caret::Box, Self::Move(step, dir, select)) => {
                duel.browser.move_filter_caret(step, dir, select);
            }
            (Caret::Box, Self::SelectAll) => duel.browser.select_all_filter(),
            (Caret::Box, Self::Done) => duel.browser.stop_typing(),
            (Caret::Box, Self::Insert(text)) => duel.browser.type_text(&text),
            (Caret::Row, Self::Back) => duel.browser.in_builder(|it| {
                it.pop_typed();
            }),
            (Caret::Row, Self::Forward) => {
                duel.browser.in_builder(FilterPanel::delete_typed_forward);
            }
            (Caret::Row, Self::Move(step, dir, select)) => duel
                .browser
                .in_builder(|it| it.move_typing_caret(step, dir, select)),
            (Caret::Row, Self::SelectAll) => duel.browser.in_builder(FilterPanel::select_all_typed),
            (Caret::Row, Self::Done) => duel.browser.in_builder(FilterPanel::stop_typing),
            (Caret::Row, Self::Insert(text)) => duel.browser.type_into_builder(&text),
        }
    }
}
