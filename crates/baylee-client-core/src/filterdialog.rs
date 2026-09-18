//! A filter string, taken apart into the controls a dialog draws — and put
//! back together again.
//!
//! The owner asked for both directions at once: a gear button inside the
//! search box opens a panel where the query can be set *"peniebelst fein"*,
//! and *"aber auch in die andere Richtung, der bestehende Filter String wird
//! in die UI Elemente zerlegt"*. Those are one problem, not two. A dialog
//! that decomposes a string and recomposes it differently is a dialog that
//! silently rewrites what the player typed the moment they open it — and the
//! first thing anyone does with a builder is open it to look.
//!
//! So the whole module is built around one property, and
//! `tests::a_form_puts_back_exactly_what_it_took_apart` asserts it over every
//! query in the language's own corpus — written as a name and not as a link,
//! because the module it names only exists in a test build:
//!
//! ```text
//! FilterForm::of(&query).query() == query
//! ```
//!
//! Exactly equal, for **every** query and not only the ones the controls
//! understand. That is what [`FilterPart`] buys: a top-level part the dialog
//! has no control for is not dropped, not normalised and not pushed to the
//! end — it keeps its place in the row of parts and is carried whole. A
//! dialog opened on `t:creature (c:r or c:g) mv:3` draws two controls with an
//! untouchable chip between them, and closing it without touching anything
//! gives back the string that was there.
//!
//! What this module is **not** is a second reading of the language. It moves
//! [`Term`]s around; it never looks inside one. Deciding what `c:rg` means is
//! [`crate::cardquery`]'s job and is done once.

use crate::cardquery::{Colors, Flag, Key, Op, Query, Surface, Term, Value, value_of};
use crate::textbuf::{Dir, Step, TextBuffer};

/// One thing standing in the row of controls a dialog draws.
///
/// The two cases are not "understood" and "broken": they are *a term that has
/// an on/off reading* and *a shape that does not*. `t:creature` may be drawn
/// as a control because everything beside it must also hold, so ticking it
/// off means something. `(c:r or c:g)` has no such reading — `a or b` minus
/// `a` is `b`, a different question — so it is carried as it stands and the
/// dialog offers to delete it whole or nothing at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilterPart {
    /// A term the dialog draws a control for, and the minus in front of it.
    Row {
        /// Whether the player wrote a `-` before it.
        negated: bool,
        /// The term itself, untouched.
        term: Term,
    },
    /// A branch no control stands for, carried verbatim and put back in place.
    Opaque(Query),
}

/// What kind of control a key wants.
///
/// A dialog reads this rather than matching on [`Key`] itself, so that adding
/// a key to the language does not mean finding every `match` in the renderer:
/// a new key with no control here is drawn as a plain text box, which is what
/// a player would have typed into anyway.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    /// A text box: a name, a word, a type, a line of rules text.
    Text,
    /// Five colour pips, and the three rules a colon, an `=` and a `<=` mean.
    Colors,
    /// A comparison and a number.
    Number,
    /// A tri-state tick: on, off, or not asked about.
    Flag,
    /// A mana cost, typed in `{1}{W}` notation.
    Cost,
}

impl Control {
    /// The control a key is drawn with.
    #[must_use]
    pub const fn of(key: &Key) -> Self {
        match key {
            Key::Color | Key::Identity => Self::Colors,
            Key::ManaValue | Key::Power | Key::Toughness | Key::Loyalty => Self::Number,
            Key::Is => Self::Flag,
            Key::Mana => Self::Cost,
            Key::Loose | Key::Name | Key::ExactName | Key::Oracle | Key::Type | Key::Unknown(_) => {
                Self::Text
            }
        }
    }

    /// The control a row is drawn with, which is a question about the value
    /// and not only about the key.
    ///
    /// [`Self::of`] answers what a *fresh* row of a key opens on; this
    /// answers what the row in front of you holds. The two disagree because
    /// one key spells more than one kind of value: `c:r` is a set of colours
    /// and `c>=2` is a count of them, and `is:token` is a flag where
    /// `is:permanent` is a word this language does not know. Drawn by its key
    /// alone, `c>=2` was five unlit pips and `is:permanent` three unlit
    /// choices — a row saying nothing about what it held, whose next button
    /// press would have overwritten it.
    ///
    /// Found by photographing the dialog rather than by a test, which is why
    /// the rule is written here rather than in the renderer: a second reader
    /// deciding this would be a second place for the two to disagree.
    #[must_use]
    pub fn for_term(term: &Term) -> Self {
        match (Self::of(&term.key), &term.value) {
            (Self::Colors, Value::Colors(_)) => Self::Colors,
            // One arm, because a count of colours *is* a comparison and a
            // number: `c>=2` and `mv>=2` are the same control asking about
            // different things, and the key beside it is what says which.
            (Self::Colors, Value::ColorCount(_))
            | (Self::Number, Value::Number(_) | Value::Parity(_)) => Self::Number,
            (Self::Flag, Value::Flag(_)) => Self::Flag,
            (Self::Cost, Value::Cost(_)) => Self::Cost,
            // Everything left over is a value its own control cannot draw: a
            // colour nickname, `c:m`, a word where a flag was expected, a
            // number that is not one. A text box draws all of them, because a
            // text box draws whatever was written.
            (Self::Colors | Self::Number | Self::Flag | Self::Cost | Self::Text, _) => Self::Text,
        }
    }
}

/// A filter string as a dialog holds it.
///
/// A list and not a struct of named fields, which is the decision the whole
/// module rests on. Named fields would mean one `name`, one `mv`, one colour
/// rule — and a player may write `t:creature t:goblin`, or `mv>=2 mv<=4`,
/// both of which are ordinary searches. Collapsing those into a single field
/// would lose one of them on the way in and re-emit a different query on the
/// way out, which is exactly the silent rewrite this module exists to avoid.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilterForm {
    /// The top-level parts, in the order they were written.
    pub parts: Vec<FilterPart>,
}

impl FilterForm {
    /// Takes a query apart into what a dialog draws.
    #[must_use]
    pub fn of(query: &Query) -> Self {
        let parts = match query {
            // An empty box opens an empty dialog, not a dialog with one blank
            // row in it: a row stands for a term, and there is no term.
            Query::Anything => Vec::new(),
            Query::All(parts) => parts.iter().map(Self::part_of).collect(),
            other => vec![Self::part_of(other)],
        };
        Self { parts }
    }

    fn part_of(query: &Query) -> FilterPart {
        match query {
            Query::Term(term) => FilterPart::Row {
                negated: false,
                term: term.clone(),
            },
            Query::Not(inner) => match &**inner {
                Query::Term(term) => FilterPart::Row {
                    negated: true,
                    term: term.clone(),
                },
                _ => FilterPart::Opaque(query.clone()),
            },
            _ => FilterPart::Opaque(query.clone()),
        }
    }

    /// The query these controls stand for.
    ///
    /// Built through the same `All` the parser builds, so the result is in the
    /// language's one canonical shape and compares equal to the query it came
    /// from. A form with nothing in it is [`Query::Anything`], which is the
    /// empty box — not an `All` of nothing, which would render as a stray
    /// pair of brackets.
    #[must_use]
    pub fn query(&self) -> Query {
        let parts: Vec<Query> = self.parts.iter().map(FilterPart::query).collect();
        crate::cardquery::all_of(parts)
    }

    /// Where the first row for this key sits, if a row for it is drawn.
    ///
    /// A dialog binding a dedicated control — the five colour pips, the mana
    /// value spinner — asks this to find out whether it is showing an
    /// existing term or offering a new one.
    #[must_use]
    pub fn row_of(&self, key: &Key) -> Option<usize> {
        self.parts.iter().position(|part| match part {
            FilterPart::Row { term, .. } => &term.key == key,
            FilterPart::Opaque(_) => false,
        })
    }

    /// Writes a term for this key: over the first row that has one, or a new
    /// row at the end.
    ///
    /// The first and not every one, for the same reason `t:creature t:goblin`
    /// is two rows: a second term with the same key is a second condition the
    /// player wrote on purpose, and a control that overwrote both would be
    /// deleting one of them.
    pub fn set(&mut self, key: &Key, op: Op, value: Value, negated: bool) {
        let row = FilterPart::Row {
            negated,
            term: Term {
                key: key.clone(),
                op,
                value,
            },
        };
        match self.row_of(key) {
            Some(at) => self.parts[at] = row,
            None => self.parts.push(row),
        }
    }

    /// Takes the first row for this key out, if there is one.
    ///
    /// Returns whether anything was removed, so a caller can tell "the tick
    /// was already off" from "the tick has just been cleared".
    pub fn clear(&mut self, key: &Key) -> bool {
        match self.row_of(key) {
            Some(at) => {
                self.parts.remove(at);
                true
            }
            None => false,
        }
    }

    /// The parts this surface cannot answer, by index.
    ///
    /// Not a filter and not a refusal: the row is still drawn and the term is
    /// still carried, because a player may be editing a line they will paste
    /// somewhere else. What it is for is the warning beside the control —
    /// a term this surface cannot answer hides every row, which reads exactly
    /// like a search that found nothing, and this is the one place the dialog
    /// can say so before it happens.
    #[must_use]
    pub fn unanswerable(&self, surface: Surface) -> Vec<usize> {
        self.parts
            .iter()
            .enumerate()
            .filter(|(_, part)| match part {
                FilterPart::Row { term, .. } => !surface.answers(&term.key),
                // A branch is unanswerable only if nothing in it can be
                // answered, which is `Match::Unknown`'s own rule: one side of
                // an `or` is enough.
                FilterPart::Opaque(query) => {
                    !query.terms().iter().any(|term| surface.answers(&term.key))
                }
            })
            .map(|(at, _)| at)
            .collect()
    }
}

impl FilterPart {
    /// The query this one part stands for.
    #[must_use]
    pub fn query(&self) -> Query {
        match self {
            Self::Row { negated, term } => {
                let term = Query::Term(term.clone());
                if *negated {
                    Query::Not(Box::new(term))
                } else {
                    term
                }
            }
            Self::Opaque(query) => query.clone(),
        }
    }
}

#[cfg(test)]
mod tests;

/// Which way a colour term is read, in the words a player thinks in.
///
/// The three are not three operators: the bare colon means *at least* on
/// `c:` and *at most* on `id:` (CR 903.4 is what makes the second one the
/// useful default — `id:c t:land` is the lands a colourless commander may
/// play). A player choosing between them is choosing the sentence, so the
/// control offers the sentence and this maps it to the operator each key
/// spells it with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorRule {
    /// The card has these colours and may have others.
    AtLeast,
    /// The card has exactly these.
    Exactly,
    /// The card has no colour outside these.
    AtMost,
}

impl ColorRule {
    /// How a written term reads.
    ///
    /// An operator with no reading here — `c!=w` — answers `None`, and the
    /// control then lights nothing and leaves the term exactly as typed.
    /// Lighting the nearest one would be a control claiming to stand for a
    /// term it does not stand for.
    #[must_use]
    pub fn of(key: &Key, op: Op) -> Option<Self> {
        let colon = if matches!(key, Key::Identity) {
            Self::AtMost
        } else {
            Self::AtLeast
        };
        match op {
            Op::Colon => Some(colon),
            Op::Eq => Some(Self::Exactly),
            Op::Ge => Some(Self::AtLeast),
            Op::Le => Some(Self::AtMost),
            Op::Ne | Op::Lt | Op::Gt => None,
        }
    }

    /// The operator this reading is written with for that key.
    ///
    /// The colon where the key already means this, so touching the control a
    /// player's own term already agrees with writes their spelling back and
    /// not a longer synonym.
    #[must_use]
    pub fn op(self, key: &Key) -> Op {
        let colon = if matches!(key, Key::Identity) {
            Self::AtMost
        } else {
            Self::AtLeast
        };
        if self == colon {
            return Op::Colon;
        }
        match self {
            Self::AtLeast => Op::Ge,
            Self::Exactly => Op::Eq,
            Self::AtMost => Op::Le,
        }
    }
}

/// What the "add a condition" button has unfolded.
///
/// Two steps and not one list of thirteen keys: the first offers the five
/// [`Control`]s, the second the keys of that one. It is the same taxonomy the
/// rows are drawn from rather than a second one invented for the menu, so a
/// key that grows a control cannot go missing from the menu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Adding {
    /// The button is a button.
    #[default]
    Closed,
    /// The five kinds are showing.
    Kinds,
    /// The keys of one kind are showing.
    Keys(Control),
}

/// The builder, as a mode of one search box.
///
/// It is a *mode* and not a window: the box keeps showing the string, the
/// gear inside it stays lit while this is open, and a tap in the box closes
/// the builder and takes the caret. Two editors of one value is the silent
/// rewrite this whole module exists to prevent, and two carets on a phone is
/// not an interface.
#[derive(Clone, Debug, Default)]
pub struct FilterPanel {
    /// The parts being edited.
    form: FilterForm,
    /// What the add button has unfolded.
    adding: Adding,
    /// The row whose value is being typed into, and the text as it stands.
    typing: Option<Typing>,
    /// Whether any row has been changed since the builder opened.
    touched: bool,
}

/// A row being typed into.
///
/// **One caret in the whole builder**, which is the module header's rule seen
/// from the other side: the box has none while the builder is open, and the
/// builder has at most one. It lives here and not in the renderer for the
/// same reason [`Act`] does — a test that cannot type cannot tell a key that
/// is wired from one that is not.
///
/// Every control has one, not only the text keys. A stepper is no way to
/// reach `mv:12`, a hybrid symbol is on no keyboard of six, and what gets
/// typed is put through [`value_of`] — the parser's own reader — so a row
/// arrives at exactly the value the same letters typed into the box would
/// have made.
#[derive(Clone, Debug)]
pub struct Typing {
    /// Which row.
    row: usize,
    /// The text, the caret and the selection.
    field: TextBuffer,
}

impl Typing {
    /// Which row the caret is in.
    #[must_use]
    pub const fn row(&self) -> usize {
        self.row
    }

    /// The text, the caret and the selection, to draw.
    #[must_use]
    pub const fn field(&self) -> &TextBuffer {
        &self.field
    }
}

impl FilterPanel {
    /// Opens the builder on what is in the box.
    #[must_use]
    pub fn open(query: &Query) -> Self {
        Self {
            form: FilterForm::of(query),
            adding: Adding::Closed,
            typing: None,
            touched: false,
        }
    }

    /// The parts to draw.
    #[must_use]
    pub fn parts(&self) -> &[FilterPart] {
        &self.form.parts
    }

    /// What the add button has unfolded.
    #[must_use]
    pub const fn adding(&self) -> Adding {
        self.adding
    }

    /// The rows this surface will answer nothing for.
    #[must_use]
    pub fn unanswerable(&self, surface: Surface) -> Vec<usize> {
        self.form.unanswerable(surface)
    }

    /// What to write into the search box, or `None` to leave it alone.
    ///
    /// `None` until a row has actually been changed, and that is the sharp
    /// edge of the whole design. The guarantee this module proves is
    /// `FilterForm::of(q).query() == q` — about *queries*, not about strings.
    /// `Key::render` picks one spelling out of several a player may have
    /// typed, so `color:red` comes back as `c:red`: correct, equal as a
    /// query, and a box rewritten by nothing but being looked at.
    #[must_use]
    pub fn written(&self) -> Option<Query> {
        self.touched.then(|| self.form.query())
    }

    /// The row being typed into, if any.
    #[must_use]
    pub const fn typing(&self) -> Option<&Typing> {
        self.typing.as_ref()
    }

    /// The control a row is drawn with, or `None` for a row that is not
    /// there.
    ///
    /// [`Control::for_term`] with the caret taken into account, and that is
    /// the whole reason the panel answers this rather than the renderer
    /// asking the term directly. Typing goes through
    /// [`value_of`](crate::cardquery::value_of) on every keystroke, so the
    /// value's *kind* changes under the caret — `toke` is a word and `token`
    /// is a flag — and a row that swapped its control mid-word would take the
    /// box away from under the letters while [`Self::typing`] still named it,
    /// with every further keystroke going into a field nothing drew.
    ///
    /// So a row being typed into is a text box until the caret is given back.
    #[must_use]
    pub fn control(&self, row: usize) -> Option<Control> {
        let Some(FilterPart::Row { term, .. }) = self.form.parts.get(row) else {
            return None;
        };
        if self.typing.as_ref().is_some_and(|t| t.row == row) {
            return Some(Control::Text);
        }
        Some(Control::for_term(term))
    }

    /// Writes a number into a row in the kind of value its own key reads.
    ///
    /// Through [`value_of`](crate::cardquery::value_of), the parser's own
    /// reader, because a number under `mv` is a [`Value::Number`] and the
    /// same number under `c` is a [`Value::ColorCount`]. Writing the first
    /// into a colour row renders the same string and parses back as the
    /// second, so the box was right and the form was holding something the
    /// box would never have produced.
    fn set_number(&mut self, row: usize, n: i32) {
        let Some(FilterPart::Row { term, .. }) = self.form.parts.get(row) else {
            return;
        };
        let value = value_of(&term.key, &n.to_string());
        self.set_value(row, value);
    }

    /// Puts the caret in a row's value, seeded with what the row already says.
    ///
    /// Seeded from [`Value::written`] and not from what the control drew, so
    /// that a colour row typed into starts at `rg` and a cost row at
    /// `{W}{W}`. A row that is not there takes no caret.
    pub fn edit(&mut self, row: usize) {
        let Some(FilterPart::Row { term, .. }) = self.form.parts.get(row) else {
            return;
        };
        let mut field = TextBuffer::new(&term.value.written());
        field.select_all();
        self.typing = Some(Typing { row, field });
    }

    /// Gives the caret back, without changing anything.
    pub fn stop_typing(&mut self) {
        self.typing = None;
    }

    /// Types into the row that holds the caret.
    pub fn type_text(&mut self, text: &str) {
        let mut clean: String = text.to_string();
        clean.retain(|c| !c.is_control());
        if clean.is_empty() {
            return;
        }
        self.typed(|field| field.insert(&clean));
    }

    /// Rubs out what is selected, or the character before the caret, and says
    /// whether there was anything to rub out.
    pub fn pop_typed(&mut self) -> bool {
        let before = self.typing.as_ref().map(|t| t.field.text().len());
        self.typed(TextBuffer::delete_back);
        before.is_some_and(|len| {
            self.typing
                .as_ref()
                .is_some_and(|t| t.field.text().len() != len)
        })
    }

    /// The same forwards — Delete.
    pub fn delete_typed_forward(&mut self) {
        self.typed(TextBuffer::delete_forward);
    }

    /// Moves the caret, extending the selection when `select`.
    pub fn move_typing_caret(&mut self, step: Step, dir: Dir, select: bool) {
        self.typed(|field| field.move_caret(step, dir, select));
    }

    /// Selects the whole of the row being typed into.
    pub fn select_all_typed(&mut self) {
        self.typed(TextBuffer::select_all);
    }

    /// Puts the caret at a byte offset, with `anchor` the other end of a
    /// selection — what a click or a drag in a row's box asks for.
    pub fn place_typing_caret(&mut self, cursor: usize, anchor: Option<usize>) {
        self.typed(|field| field.place(cursor, anchor));
    }

    /// Does one editing gesture and writes the row it was done in.
    ///
    /// Every keystroke writes the value, rather than a `Return` committing
    /// one: the box shows what the builder holds and the list under it is
    /// filtered by that, so a row that only took effect on `Return` would be
    /// a row a player could not see the effect of.
    fn typed(&mut self, edit: impl FnOnce(&mut TextBuffer)) {
        let Some(typing) = self.typing.as_mut() else {
            return;
        };
        edit(&mut typing.field);
        let row = typing.row;
        let text = typing.field.text().to_string();
        let Some(FilterPart::Row { term, .. }) = self.form.parts.get_mut(row) else {
            // The row went while the caret was in it, which a stale button
            // can do; the caret goes with it rather than writing into
            // whatever moved up into its place.
            self.typing = None;
            return;
        };
        term.value = value_of(&term.key, &text);
        self.touched = true;
    }

    /// Turns the minus in front of a row on or off.
    pub fn negate(&mut self, row: usize, negated: bool) {
        if let Some(FilterPart::Row { negated: at, .. }) = self.form.parts.get_mut(row) {
            *at = negated;
            self.touched = true;
        }
    }

    /// Takes a row out. The only way to change a row's key, and the way a
    /// flag control says "don't ask": a row standing for no term would be a
    /// row that writes nothing.
    pub fn remove(&mut self, row: usize) {
        if row < self.form.parts.len() {
            self.form.parts.remove(row);
            // The parts below move up, so a caret below this one is in a
            // different row now and a caret *in* it has nowhere to be.
            match self.typing.as_mut() {
                Some(typing) if typing.row == row => self.typing = None,
                Some(typing) if typing.row > row => typing.row -= 1,
                _ => {}
            }
            self.touched = true;
        }
    }

    /// Writes a row's operator.
    pub fn set_op(&mut self, row: usize, op: Op) {
        if let Some(FilterPart::Row { term, .. }) = self.form.parts.get_mut(row) {
            term.op = op;
            self.touched = true;
        }
    }

    /// Writes a row's value.
    pub fn set_value(&mut self, row: usize, value: Value) {
        if let Some(FilterPart::Row { term, .. }) = self.form.parts.get_mut(row) {
            term.value = value;
            self.touched = true;
            self.reseed(row);
        }
    }

    /// Puts the row's own spelling back into the caret, if the caret is in it.
    ///
    /// A pip tapped while the row was being typed into leaves a box holding
    /// the text from before the tap, and the next letter typed would write
    /// that stale text back over the pip — the control and the box would be
    /// two editors of one value, which is the thing this module exists to
    /// prevent, at the width of one row.
    fn reseed(&mut self, row: usize) {
        let Some(typing) = self.typing.as_mut() else {
            return;
        };
        if typing.row != row {
            return;
        }
        let Some(FilterPart::Row { term, .. }) = self.form.parts.get(row) else {
            return;
        };
        typing.field = TextBuffer::new(&term.value.written());
    }

    /// Writes the operator a colour reading is spelled with for that row's key.
    pub fn set_rule(&mut self, row: usize, rule: ColorRule) {
        if let Some(FilterPart::Row { term, .. }) = self.form.parts.get_mut(row) {
            term.op = rule.op(&term.key);
            self.touched = true;
        }
    }

    /// Turns one colour of a colour row on or off.
    ///
    /// Colourless is exclusive with the five, in both directions: a card is
    /// colourless or it is not, so `c:wc` is not a question.
    pub fn toggle_color(&mut self, row: usize, letter: char) {
        let Some(FilterPart::Row { term, .. }) = self.form.parts.get_mut(row) else {
            return;
        };
        let held = match &term.value {
            Value::Colors(colors) => *colors,
            _ => Colors::default(),
        };
        let one = Colors::of_letters(&letter.to_string());
        let next = if letter.eq_ignore_ascii_case(&'c') {
            // Tapping colourless clears the five; tapping it again clears it,
            // which is the same empty set — so it stays chosen until a colour
            // is.
            Colors::default()
        } else if held.holds(one) {
            Colors(held.0 & !one.0)
        } else {
            Colors(held.0 | one.0)
        };
        term.value = Value::Colors(next);
        self.touched = true;
        self.reseed(row);
    }

    /// Unfolds the add button one step, or folds it away.
    pub fn add_step(&mut self, step: Adding) {
        self.adding = step;
    }

    /// Adds a row for this key, with the value a fresh control starts at.
    ///
    /// The menu closes with it: a player who has just added a condition is
    /// looking at the condition, not at the menu they added it from.
    ///
    /// Every key goes through here, a mana cost included. That was a special
    /// case for one revision, on a reading of `Value::Cost(vec![])` that
    /// turned out to be wrong in all three halves — it renders as `m:""`, it
    /// reads back as the same empty cost, and the evaluator answers `Yes` to
    /// it, because a cost containing no named symbol is every cost. It is an
    /// ordinary empty opening like `t:""`, and the row carries the same ✕ as
    /// its neighbours instead of being deleted by its own last symbol.
    pub fn add(&mut self, key: &Key) {
        self.form.parts.push(FilterPart::Row {
            negated: false,
            term: Term {
                key: key.clone(),
                op: starting_op(key),
                value: starting_value(key),
            },
        });
        self.adding = Adding::Closed;
        self.touched = true;
    }

    /// Appends one symbol to a cost row.
    ///
    /// The symbol is written as the language writes it, without braces —
    /// `"W"`, `"2"`, `"W/U"` — because that is what [`Value::Cost`] holds and
    /// what `cardquery::render` puts the braces back on.
    pub fn push_symbol(&mut self, row: usize, symbol: &str) {
        if let Some(FilterPart::Row { term, .. }) = self.form.parts.get_mut(row)
            && let Value::Cost(symbols) = &mut term.value
        {
            symbols.push(symbol.to_string());
            self.touched = true;
            self.reseed(row);
        }
    }

    /// Takes the last symbol off a cost row.
    ///
    /// A row emptied this way stays: `m:""` is a written term asking for a
    /// cost with no named symbol in it, which every card answers, and the ✕
    /// beside the row is how a row goes. Backspacing past the first symbol
    /// therefore does nothing at all rather than deleting the row under the
    /// finger that was editing it.
    pub fn pop_symbol(&mut self, row: usize) {
        let Some(FilterPart::Row { term, .. }) = self.form.parts.get_mut(row) else {
            return;
        };
        let Value::Cost(symbols) = &mut term.value else {
            return;
        };
        if symbols.pop().is_some() {
            self.touched = true;
            self.reseed(row);
        }
    }

    /// Empties the builder, and with it the box.
    pub fn clear(&mut self) {
        self.form.parts.clear();
        self.adding = Adding::Closed;
        self.typing = None;
        self.touched = true;
    }
}

/// The operator a fresh row of this key opens with.
///
/// It has to be the operator the *written* form reads back as, or a fresh row
/// fails the round trip the moment it is added. `!word` is the case that is
/// not a matter of taste: it carries no operator on the page at all, so the
/// parser supplies `=` and a row opening on a colon would come back as a
/// different term than the one it wrote.
fn starting_op(key: &Key) -> Op {
    match key {
        Key::ExactName => Op::Eq,
        _ => match Control::of(key) {
            // A number opens at `=`, which is what a player means by typing
            // one.
            Control::Number => Op::Eq,
            _ => Op::Colon,
        },
    }
}

/// The value a fresh row of this key opens with.
///
/// Every one of them is a term that is *legal to write* and narrows nothing:
/// no colours chosen is `c:` with the empty set, which asks for colourless —
/// so a colour row opens on colourless rather than on a term that cannot be
/// rendered, and every other opening is an empty value.
///
/// "Narrows nothing" is asserted over the whole menu in
/// `every_kind_of_row_opens_on_something_writable`, and over the language in
/// `cardquery::tests::a_value_nobody_has_typed_into_narrows_nothing` — where
/// it was found to be false for two of the five word keys and, worse, for two
/// of them to be no term at all: `""` and `!""` were read back as
/// `Query::Anything`, so a row a player had just added vanished out of the
/// box that was holding it.
fn starting_value(key: &Key) -> Value {
    match Control::of(key) {
        Control::Colors => Value::Colors(Colors::default()),
        Control::Number => Value::Number(0),
        // `is:` is the only flag key there is, and a fresh one opens on the
        // flag every surface knows something about rather than on nothing:
        // a row with no flag chosen could not be written at all.
        Control::Flag => Value::Flag(Flag::Playable),
        // No symbol named is every cost, the same way an empty word is every
        // word — `m:""` is written, read back and answered `Yes` to.
        Control::Cost => Value::Cost(Vec::new()),
        Control::Text => Value::Word(String::new()),
    }
}

/// The keys a dialog may offer to add, in the order it offers them.
///
/// One list, read both for the menu and for the [`Press`](crate) a button
/// carries — a button holds an *index into this* rather than a `Key`, because
/// a `Key` owns a `String` for the one variant nothing offers and a button
/// that has to be cloned is a button that cannot be `Copy`. The grouping in
/// the menu is [`Control::of`] over this list and is not a second list, so a
/// key that grows a control cannot go missing from the menu.
///
/// `Key::Unknown` is deliberately absent: a dialog offers the language's own
/// keys, and an unknown one exists only because a player typed it.
pub const OFFERED: &[Key] = &[
    Key::Loose,
    Key::Name,
    Key::ExactName,
    Key::Type,
    Key::Oracle,
    Key::Color,
    Key::Identity,
    Key::ManaValue,
    Key::Power,
    Key::Toughness,
    Key::Loyalty,
    Key::Is,
    Key::Mana,
];

/// The mana symbols a cost row's keyboard offers, as [`Value::Cost`] holds
/// them — without braces, which `cardquery::render` puts back.
///
/// Six and not the whole of Magic's notation: hybrid, Phyrexian and `{X}`
/// exist and are typed into the field beside the keyboard. A keyboard with
/// every symbol on it would be a page, and the five colours plus one generic
/// are what a cost is mostly made of.
pub const SYMBOLS: &[&str] = &["W", "U", "B", "R", "G", "1"];

/// The flags a dialog may offer for an `is:` row, in the order it offers them.
///
/// Filtered by the surface at the point of drawing: the pool knows six of
/// them and a zone three, and a flag a surface cannot answer is left out of
/// the menu rather than greyed in it — a button that lights under the pointer
/// and then refuses the click is worse than no button.
pub const FLAGS: &[Flag] = &Flag::ALL;

/// One thing a button in the builder means.
///
/// The buttons carry *these* and not method calls, which is the same seam
/// every other panel in this client has: a renderer maps a click to an `Act`
/// and knows nothing else, and a test drives the builder through the same
/// list rather than through a private method a button might not be wired to.
/// `crates/baylee-client-core` has shipped a decision function nothing called
/// before — `Interaction::activate` sat written and unreachable — and a test
/// that calls the method cannot tell.
///
/// Every variant is `Copy`, because a Bevy `Component` on a button is, and
/// that is why a key is an index into [`OFFERED`] rather than a [`Key`]: one
/// variant of `Key` owns a `String`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    /// Turn the minus in front of a row on or off.
    Negate(usize, bool),
    /// Take a row out.
    Remove(usize),
    /// Write a row's operator.
    SetOp(usize, Op),
    /// Write the operator a colour reading spells for that row's key.
    SetRule(usize, ColorRule),
    /// Turn one colour of a colour row on or off.
    Colour(usize, char),
    /// Write a number row's value.
    SetNumber(usize, i32),
    /// Add to it, which is what a stepper does.
    Bump(usize, i32),
    /// Ask a mana value for a parity instead of a number, or stop.
    Parity(usize, Option<bool>),
    /// Write which flag a flag row asks about, by its index in [`FLAGS`].
    SetFlag(usize, usize),
    /// Unfold the add button one step, or fold it away.
    AddStep(Adding),
    /// Add a row for the key at that index in [`OFFERED`].
    Add(usize),
    /// Put the caret in that row's value.
    Edit(usize),
    /// Give the caret back.
    StopTyping,
    /// Append that symbol to a cost row.
    PushSymbol(usize, usize),
    /// Take the last symbol off one, and the row with the last of them.
    PopSymbol(usize),
    /// Empty the builder.
    Clear,
}

impl FilterPanel {
    /// Does what a button means.
    ///
    /// An index that names nothing does nothing, quietly: a button drawn a
    /// frame ago may name a row a later frame no longer has, and a panel that
    /// panicked on one would be a panel that crashed on a double tap.
    pub fn act(&mut self, act: Act) {
        match act {
            Act::Negate(row, on) => self.negate(row, on),
            Act::Remove(row) => self.remove(row),
            Act::SetOp(row, op) => self.set_op(row, op),
            Act::SetRule(row, rule) => self.set_rule(row, rule),
            Act::Colour(row, letter) => self.toggle_color(row, letter),
            Act::SetNumber(row, n) => self.set_number(row, n),
            Act::Bump(row, by) => {
                let now = match self.form.parts.get(row) {
                    Some(FilterPart::Row { term, .. }) => match term.value {
                        // One arm for two values, the same pairing
                        // `Value::written` makes: a number and a count of
                        // colours are the same number on the page, and the
                        // stepper is stepping whichever of them is there. It
                        // read only the first, so `+` on a `c>=2` row read
                        // nought and wrote `c>=1`.
                        Value::Number(n) | Value::ColorCount(n) => n,
                        _ => 0,
                    },
                    _ => return,
                };
                // A printed number is never negative and a mana value never
                // is either, so the stepper stops at nought rather than
                // writing a term no card can answer.
                self.set_number(row, (now + by).max(0));
            }
            Act::Parity(row, even) => match even {
                Some(even) => self.set_value(row, Value::Parity(even)),
                None => self.set_number(row, 0),
            },
            Act::SetFlag(row, at) => {
                if let Some(flag) = FLAGS.get(at) {
                    self.set_value(row, Value::Flag(*flag));
                }
            }
            Act::Edit(row) => self.edit(row),
            Act::StopTyping => self.stop_typing(),
            Act::AddStep(step) => self.add_step(step),
            Act::Add(at) => {
                if let Some(key) = OFFERED.get(at) {
                    self.add(key);
                }
            }
            Act::PushSymbol(row, at) => {
                if let Some(symbol) = SYMBOLS.get(at) {
                    self.push_symbol(row, symbol);
                }
            }
            Act::PopSymbol(row) => self.pop_symbol(row),
            Act::Clear => self.clear(),
        }
    }
}
