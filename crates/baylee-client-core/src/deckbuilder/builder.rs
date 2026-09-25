//! What a deck builder *does* with its pool: search, filter, group, and
//! the two lists it maintains.
//!
//! [`DeckBuilder::problems`] is the half worth reading first — it mirrors
//! what `POST \/decks` enforces, split into blocking and advisory, so a live
//! save button means the deck will save.

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;
use crate::cardquery::{Colors, Facts, Flag, Surface};

impl DeckBuilder {
    /// An empty builder with no pool yet.
    #[must_use]
    pub fn new() -> Self {
        // Nothing but `Default`, and deliberately so: this used to be where
        // "playable only" was switched on, and `Lobby` — which derives
        // `Default` and is the only thing that ever holds one — never came
        // through here. See `show_unplayable`.
        Self::default()
    }

    /// Whether the pool has arrived.
    #[must_use]
    pub fn loaded(&self) -> bool {
        !self.pool.is_empty()
    }

    /// Whether the gateway could serve rules text. A builder that knows it
    /// cannot search rules text can say so once instead of looking broken.
    #[must_use]
    pub fn has_text(&self) -> bool {
        self.has_text
    }

    /// The pool, for a shell that needs to draw a row.
    #[must_use]
    pub fn pool(&self) -> &[PoolCard] {
        &self.pool
    }

    /// Changes whenever card metadata is replaced, even at the same pool size.
    #[must_use]
    pub fn pool_revision(&self) -> u64 {
        self.pool_revision
    }

    /// One pool card.
    #[must_use]
    pub fn card(&self, slot: usize) -> Option<&PoolCard> {
        self.pool.get(slot)
    }

    /// The filtered, sorted search results as pool slots.
    #[must_use]
    pub fn results(&self) -> &[usize] {
        &self.results
    }

    /// The current search text.
    #[must_use]
    pub fn text(&self) -> &str {
        self.text.text()
    }

    /// The deck's name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.text()
    }

    /// The id of the deck being edited, if this is not a new one.
    #[must_use]
    pub fn editing(&self) -> Option<&str> {
        self.editing.as_deref()
    }

    /// Whether anything has changed since the deck was loaded or saved.
    #[must_use]
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Which list an "add" goes to.
    #[must_use]
    pub fn zone(&self) -> Zone {
        self.zone
    }

    /// The colors the filter is restricted to, empty for "any".
    #[must_use]
    pub fn colors(&self) -> &[char] {
        &self.colors
    }

    /// The card type the filter is restricted to.
    #[must_use]
    pub fn kind(&self) -> Option<&str> {
        self.kind.as_deref()
    }

    /// The mana value the filter is restricted to.
    #[must_use]
    pub fn cmc(&self) -> Option<u32> {
        self.cmc
    }

    /// Whether cards the engine cannot play are hidden.
    #[must_use]
    pub const fn playable_only(&self) -> bool {
        !self.show_unplayable
    }

    /// The result order.
    #[must_use]
    pub fn sort(&self) -> Sort {
        self.sort
    }

    /// One zone's rows, in the order a deck list prints them.
    #[must_use]
    pub fn entries(&self, zone: Zone) -> &[Entry] {
        match zone {
            Zone::Main => &self.main,
            Zone::Side => &self.side,
        }
    }

    /// Cards a loaded deck named that the pool no longer has.
    #[must_use]
    pub fn missing(&self) -> &[String] {
        &self.missing
    }

    /// How many copies of a pool card the deck holds, across every
    /// printing of it.
    ///
    /// The copy limit is on the card: four Lightning Bolts are four
    /// Lightning Bolts however many different pieces of cardboard they are.
    #[must_use]
    pub fn count_of(&self, slot: usize, zone: Zone) -> u16 {
        self.entries(zone)
            .iter()
            .filter(|e| e.slot == slot)
            .fold(0u16, |sum, e| sum.saturating_add(e.count))
    }

    /// Where a card's first row sits in a zone's list.
    ///
    /// A card with two printings has two rows; this finds the first, which is
    /// what an action aimed at "this card" should act on.
    #[must_use]
    pub fn row_of(&self, slot: usize, zone: Zone) -> Option<usize> {
        self.entries(zone).iter().position(|e| e.slot == slot)
    }

    // ------------------------------------------------------------- the pool

    /// Forgets the pool and everything built on it (the deck being edited,
    /// the filters, the search) and becomes a builder that never had one.
    ///
    /// The lobby's at sign-out (#270): `/pool` answers a session, so a
    /// signed-out lobby holds no pool that a language change would ask for
    /// again without one, and the next sign-in, perhaps to another gateway,
    /// fetches its own. The revision still moves on, so nothing that drew
    /// the old pool takes the next one for it.
    pub fn forget_pool(&mut self) {
        let pool_revision = self.pool_revision.wrapping_add(1);
        *self = Self {
            pool_revision,
            ..Self::default()
        };
    }

    /// Takes the pool and rebuilds the results.
    pub fn set_pool(&mut self, cards: Vec<PoolCard>, has_text: bool) {
        self.pool_revision = self.pool_revision.wrapping_add(1);
        self.keys = cards
            .iter()
            .map(|card| crate::prose::sort_key(&card.name))
            .collect();
        self.pool = cards;
        self.has_text = has_text;
        // A deck may have been loaded before the pool arrived; its rows were
        // held by name and become real entries now.
        self.resolve_pending();
        self.refilter();
    }

    /// Fill missing type translations from the same vocabulary used by the catalog.
    pub fn localize_types(&mut self, lang: crate::i18n::Lang) -> bool {
        let mut changed = false;
        for card in &mut self.pool {
            let translated = super::types::translated(&card.type_line, lang);
            if translated != card.type_line {
                card.type_line = translated;
                changed = true;
            }
        }
        if changed {
            self.pool_revision = self.pool_revision.wrapping_add(1);
        }
        changed
    }

    /// Apply presentation-only catalog text without reordering the working deck.
    pub fn localize(&mut self, entries: &[crate::card_face::CardTextEntry]) -> bool {
        let mut changed = false;
        for entry in entries {
            let Some((slot, card)) = self
                .pool
                .iter_mut()
                .enumerate()
                .find(|(_, c)| c.scryfall_id == entry.scryfall_id)
            else {
                continue;
            };
            let Some(face) = entry.faces.first() else {
                continue;
            };
            if !face.name.is_empty() && card.name != face.name {
                card.name.clone_from(&face.name);
                self.keys[slot] = crate::prose::sort_key(&face.name);
                if !card.alt_names.contains(&face.name) {
                    card.alt_names.push(face.name.clone());
                }
                changed = true;
            }
            if !face.type_line.is_empty() && card.type_line != face.type_line {
                card.type_line.clone_from(&face.type_line);
                changed = true;
            }
            if !face.oracle_text.is_empty() && card.oracle_text != face.oracle_text {
                card.oracle_text.clone_from(&face.oracle_text);
                changed = true;
            }
        }
        if changed {
            self.pool_revision = self.pool_revision.wrapping_add(1);
        }
        changed
    }

    /// The pool slot holding a card, by its English name.
    #[must_use]
    pub fn slot_of(&self, english_name: &str) -> Option<usize> {
        self.pool
            .iter()
            .position(|c| c.english_name == english_name)
    }

    // ----------------------------------------------------------- the filter

    /// Sets the search text.
    pub fn set_text(&mut self, text: &str) {
        self.text = crate::textbuf::TextBuffer::new(text);
        self.retext();
    }

    /// Types one character into the search box.
    pub fn type_char(&mut self, ch: char) {
        self.text.insert(&ch.to_string());
        self.retext();
    }

    /// Deletes the last character of the search box.
    pub fn backspace(&mut self) {
        self.text.delete_back();
        self.retext();
    }

    /// Reads the box again, then filters.
    ///
    /// The one place the string and the query it stands for are written
    /// together. Parsing costs one pass over what a player typed; not doing
    /// it here would cost one pass per pool card per keystroke.
    fn retext(&mut self) {
        self.query = crate::cardquery::parse(self.text.text());
        self.refilter();
    }

    /// The filter-string builder, while the gear is open.
    #[must_use]
    pub const fn panel(&self) -> Option<&crate::filterdialog::FilterPanel> {
        self.panel.as_ref()
    }

    /// Opens the builder on what is in the box, or closes it.
    pub fn toggle_panel(&mut self) {
        self.panel = if self.panel.is_some() {
            None
        } else {
            Some(crate::filterdialog::FilterPanel::open(&self.query))
        };
    }

    /// Shuts the builder.
    pub fn close_panel(&mut self) {
        self.panel = None;
    }

    /// Does what a button in the builder means, and writes the box.
    ///
    /// The box is written from `FilterPanel::written`, which answers `None`
    /// until a row has actually been changed — so opening the builder to look
    /// at a string leaves the string alone, down to its spelling.
    pub fn filter_act(&mut self, act: crate::filterdialog::Act) {
        self.in_panel(|panel| panel.act(act));
    }

    /// Types into whichever row of the builder holds the caret.
    pub fn type_into_panel(&mut self, text: &str) {
        self.in_panel(|panel| panel.type_text(text));
    }

    /// One editing gesture in the builder, and the box written after it.
    ///
    /// A closure rather than a forwarding method per gesture, which is the
    /// same bargain `Browser::in_builder` makes: a gesture added to the panel
    /// would otherwise be one more to remember to forward.
    pub fn in_panel(&mut self, edit: impl FnOnce(&mut crate::filterdialog::FilterPanel)) {
        let Some(panel) = self.panel.as_mut() else {
            return;
        };
        edit(panel);
        let Some(query) = panel.written() else {
            return;
        };
        // Through `set_text` and not by writing the field, so the query
        // beside it is re-read and the results are refiltered — the two are
        // written together in one place, which is what `retext` is for.
        let written = crate::cardquery::render(&query);
        self.set_text(&written);
    }

    /// What is in the search box, read as a query.
    ///
    /// The filter dialog's half of the round trip:
    /// [`crate::filterdialog::FilterForm::of`] takes this apart into the
    /// controls it draws, and writing the form back out gives the same query
    /// again — which is why the dialog may be opened on a line it does not
    /// fully understand.
    #[must_use]
    pub const fn query(&self) -> &crate::cardquery::Query {
        &self.query
    }

    /// Turns one color on or off. No colors means every color.
    pub fn toggle_color(&mut self, color: char) {
        if let Some(at) = self.colors.iter().position(|c| *c == color) {
            self.colors.remove(at);
        } else {
            self.colors.push(color);
        }
        self.refilter();
    }

    /// Restricts to one card type, or clears the restriction.
    pub fn set_kind(&mut self, kind: Option<&str>) {
        self.kind = kind.map(str::to_string);
        self.refilter();
    }

    /// Restricts to one mana value, or clears the restriction. This is what a
    /// click on a curve bar does.
    pub fn set_cmc(&mut self, cmc: Option<u32>) {
        self.cmc = if self.cmc == cmc { None } else { cmc };
        self.refilter();
    }

    /// Shows or hides the cards the engine cannot play.
    pub fn toggle_playable_only(&mut self) {
        self.show_unplayable = !self.show_unplayable;
        self.refilter();
    }

    /// Moves to the next result order.
    pub fn cycle_sort(&mut self) {
        self.sort = self.sort.next();
        self.refilter();
    }

    /// Clears every filter, including the text.
    pub fn clear_filters(&mut self) {
        self.text.clear();
        self.colors.clear();
        self.kind = None;
        self.cmc = None;
        self.retext();
    }

    /// The chips narrowing the list, beside whatever the query says.
    ///
    /// The deck builder filters **twice** — four chips and a query box — and
    /// only one of the two is being edited when the filter panel is open. So
    /// the panel says this in a line, and the rule it keeps is that two
    /// invisible truths never stand beside each other: a player who has typed
    /// a query and cannot find a card is looking at the wrong half.
    ///
    /// It matters most where it is least visible. `buildui`'s `chips_shown`
    /// is `!phone || filters_open`, so on a phone a chip narrows the list
    /// while being drawn nowhere at all.
    ///
    /// The model decides *what* is in force and the renderer names it,
    /// because a colour's and a type's word are in `buildui`'s own tables —
    /// the type's key stays English on purpose, since it is matched against a
    /// printed type line.
    ///
    /// [`Self::playable_only`] is in the list although [`Self::filtered`]
    /// leaves it out, and the two are right about different questions.
    /// `filtered` answers "is there anything for a Clear button to clear",
    /// and a standing preference is not that. This answers "is something
    /// hiding cards", where the switch is the **largest** of the four: it
    /// hides every card the engine cannot play as printed, which is more than
    /// any colour or type chip drops, and it is on by default — so a player
    /// who never touched it is exactly the player who will not think of it.
    #[must_use]
    pub fn chips_in_force(&self) -> Vec<Chip<'_>> {
        let mut out = Vec::new();
        if !self.colors.is_empty() {
            out.push(Chip::Colors(&self.colors));
        }
        if let Some(kind) = &self.kind {
            out.push(Chip::Kind(kind));
        }
        if let Some(cmc) = self.cmc {
            out.push(Chip::Cmc(cmc));
        }
        if !self.show_unplayable {
            out.push(Chip::PlayableOnly);
        }
        out
    }

    /// Whether anything is narrowing the results.
    #[must_use]
    pub fn filtered(&self) -> bool {
        !self.text.is_empty()
            || !self.colors.is_empty()
            || self.kind.is_some()
            || self.cmc.is_some()
    }

    /// Recomputes the result list.
    fn refilter(&mut self) {
        let mut hits: Vec<usize> = (0..self.pool.len())
            .filter(|slot| self.matches(&self.pool[*slot]))
            .collect();
        let sort = self.sort;
        // The name every order ends in is the *folded* one: `str::cmp` is
        // byte order, which files every accented letter above `z`, so a
        // German pool alphabetised on `name` put Ätherfluss after Zombie.
        let key = |slot: &usize| self.keys.get(*slot).map_or("", String::as_str);
        // `sort_by` rather than `sort_unstable_by`: every comparison ends in
        // the name, so the order is total, but a stable sort keeps it obvious
        // that two runs of the same filter cannot disagree.
        hits.sort_by(|a, b| {
            let (x, y) = (&self.pool[*a], &self.pool[*b]);
            match sort {
                Sort::Name => key(a).cmp(key(b)),
                Sort::Cost => x.cmc.cmp(&y.cmc).then_with(|| key(a).cmp(key(b))),
                Sort::Type => x
                    .group()
                    .cmp(&y.group())
                    .then_with(|| x.cmc.cmp(&y.cmc))
                    .then_with(|| key(a).cmp(key(b))),
            }
        });
        self.results = hits;
    }

    /// Whether one card survives the current filter.
    ///
    /// The chips and the box are two mechanisms and stay two: a chip is a
    /// switch with a state a player can see at a glance, and the box is a
    /// sentence. They meet here, with `and` between them.
    ///
    /// **`playable_only` is deliberately not a term.** It is a safety
    /// default rather than a search — it hides the cards the engine does
    /// nothing with — and as `is:playable` in the string it would mean the
    /// box was never empty, the placeholder never shown, and clearing the
    /// box would quietly offer stubs. `is:playable`, `is:partial` and
    /// `is:stub` exist as terms beside it, for asking on purpose.
    fn matches(&self, card: &PoolCard) -> bool {
        if !self.show_unplayable && card.coverage == Coverage::Unimplemented {
            return false;
        }
        if let Some(kind) = &self.kind
            && !card.is(kind)
        {
            return false;
        }
        if let Some(cmc) = self.cmc
            && (card.is("Land") || card.cmc != cmc)
        {
            return false;
        }
        if !self.colors.is_empty() && !self.color_match(card) {
            return false;
        }
        if self.query.is_anything() {
            return true;
        }
        let flags = pool_flags(card);
        crate::cardquery::matches(&self.query, &pool_facts(card, &flags), Surface::POOL).shown()
    }

    /// Whether a card is within the chosen colors.
    ///
    /// Within, not overlapping: picking W and U asks for the cards a
    /// white-blue deck could play, so a card that also needs black is out.
    /// Colorless cards belong to every deck and always survive.
    fn color_match(&self, card: &PoolCard) -> bool {
        card.identity.chars().all(|c| self.colors.contains(&c))
    }

    // -------------------------------------------------- the printing picker

    /// The open picker, if there is one.
    #[must_use]
    pub fn picker(&self) -> Option<&Picker> {
        self.picker.as_ref()
    }

    /// Opens the picker on a pool card, and asks the gateway for its
    /// printings.
    ///
    /// The dialog opens *before* the answer arrives, showing the printing the
    /// pool row already names: a picker that appeared only once the network
    /// answered would feel like a dropped tap.
    pub fn open_picker(&mut self, slot: usize, zone: Zone) -> Option<LobbyRequest> {
        let card = self.pool.get(slot)?;
        let reference = Printing {
            scryfall_id: card.scryfall_id.clone(),
            oracle_id: card.oracle_id.clone(),
            lang: "en".to_string(),
            name: card.english_name.clone(),
            layout: if card.has_back_image {
                "transform".into()
            } else {
                String::new()
            },
            ..Printing::default()
        };
        let index = card.index;
        self.focus_epoch = self.focus_epoch.wrapping_add(1);
        self.picker = Some(Picker {
            slot,
            zone,
            card: index,
            reference_id: card.scryfall_id.clone(),
            langs: vec!["en".to_string()],
            printings: vec![reference],
            loading: true,
            ..Picker::default()
        });
        if let Some((prints, catalog)) = self.printings_cache.get(&index).cloned() {
            self.set_printings(index, prints, catalog);
            None
        } else {
            Some(LobbyRequest::LoadPrintings { card: index })
        }
    }

    /// Change the artwork/finish of one exact row, preserving copies and notes.
    pub fn open_row_picker(&mut self, at: usize, zone: Zone) -> Option<LobbyRequest> {
        let entry = self.entries(zone).get(at)?.clone();
        let request = self.open_picker(entry.slot, zone);
        if let Some(picker) = &mut self.picker {
            picker.replacing = Some(entry);
            picker.select_original();
        }
        request
    }

    /// Forget the cached metadata and request a fresh printing catalog.
    pub fn refresh_printings(&mut self) -> Option<LobbyRequest> {
        let picker = self.picker.as_mut()?;
        if picker.loading {
            return None;
        }
        picker.refresh_selection = picker
            .current()
            .map(|p| (p.scryfall_id.clone(), picker.finish, picker.force_finish));
        self.printings_cache.remove(&picker.card);
        picker.loading = true;
        Some(LobbyRequest::LoadPrintings { card: picker.card })
    }

    /// Toggle cosmetic finishes independently of physical availability.
    pub fn picker_force_finish(&mut self) {
        if let Some(picker) = &mut self.picker {
            picker.force_finish = !picker.force_finish;
            picker.settle();
        }
    }

    /// Filter by a set from the multilingual set list.
    pub fn picker_set_set(&mut self, at: Option<usize>) {
        if let Some(picker) = &mut self.picker {
            picker.set = at.and_then(|i| picker.sets().get(i).map(|(s, _)| (*s).to_string()));
            if picker.set.as_ref().is_some_and(|set| {
                !picker.printings.iter().any(|p| {
                    p.set == *set && picker.lang.as_ref().is_none_or(|lang| *lang == p.lang)
                })
            }) {
                picker.lang = None;
            }
            picker.set_open = false;
            picker.set_query = crate::textbuf::TextBuffer::default();
            picker.at = 0;
            picker.settle();
        }
    }

    /// Close the autocomplete without changing the selected set.
    pub fn picker_close_sets(&mut self) {
        if let Some(picker) = &mut self.picker {
            picker.set_open = false;
            picker.set_query = crate::textbuf::TextBuffer::default();
        }
    }

    /// Keyboard navigation of the set suggestions.
    pub fn picker_move_set(&mut self, forward: bool) {
        if let Some(p) = &mut self.picker {
            let n = p.matching_sets().len();
            if n > 0 {
                p.set_cursor = (p.set_cursor + if forward { 1 } else { n - 1 }) % n;
            }
        }
    }

    /// Accept the highlighted set result.
    pub fn picker_choose_set(&mut self) {
        let at = self
            .picker
            .as_ref()
            .and_then(|p| p.matching_sets().get(p.set_cursor).copied());
        if at.is_some() {
            self.picker_set_set(at);
        }
    }

    /// Closes the picker without adding anything.
    pub fn close_picker(&mut self) {
        // Returning to the builder is not a request to raise the phone keyboard.
        self.focus = BuildField::Search;
        self.picker = None;
    }

    /// The gateway's answer.
    ///
    /// Matched on the registry index rather than accepted blindly: a slow
    /// answer for a card the player has already moved on from would otherwise
    /// replace the printings of the one they are looking at.
    pub fn set_printings(&mut self, card: u32, printings: Vec<Printing>, from_catalog: bool) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        if picker.card != card {
            return;
        }
        picker.loading = false;
        picker.from_catalog = from_catalog;
        if printings.is_empty() {
            return;
        }
        let mut langs: Vec<String> = Vec::new();
        for printing in &printings {
            if !printing.lang.is_empty() && !langs.contains(&printing.lang) {
                langs.push(printing.lang.clone());
            }
        }
        self.printings_cache
            .insert(card, (printings.clone(), from_catalog));
        picker.printings = printings;
        picker.langs = langs;
        picker.at = 0;
        if picker
            .lang
            .as_ref()
            .is_some_and(|lang| !picker.langs.contains(lang))
        {
            picker.lang = None;
        }
        if picker
            .set
            .as_ref()
            .is_some_and(|set| !picker.sets().iter().any(|(code, _)| *code == set))
        {
            picker.set = None;
        }
        // Both filter values may still exist separately while their combination
        // vanished in the refreshed catalog. Keep the language, widen the set.
        if picker.is_empty() {
            picker.set = None;
        }
        picker.select_original();
        if let Some((id, finish, force)) = picker.refresh_selection.take() {
            picker.at = picker
                .visible()
                .iter()
                .position(|p| p.scryfall_id == id)
                .unwrap_or(0);
            picker.finish = finish;
            picker.force_finish = force;
        }
        picker.settle();
    }

    /// Moves the carousel, wrapping at both ends.
    ///
    /// Wrapping rather than stopping because the carousel is a ring of art
    /// with no beginning: a player flicking through twelve printings should
    /// not have to notice which one the list happened to start at.
    pub fn picker_step(&mut self, by: i32) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        let len = picker.len();
        if len == 0 {
            return;
        }
        let len_i = i64::try_from(len).unwrap_or(1);
        let at = i64::try_from(picker.at).unwrap_or(0);
        let next = (at + i64::from(by)).rem_euclid(len_i);
        picker.at = usize::try_from(next).unwrap_or(0);
        picker.settle();
    }

    /// Jumps the carousel to one printing.
    pub fn picker_go(&mut self, at: usize) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        picker.at = at;
        picker.settle();
    }

    /// Limits the carousel to one language, or to all of them.
    pub fn picker_set_lang(&mut self, lang: Option<&str>) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        picker.lang = lang.map(str::to_string);
        picker.set = None;
        // The card the player was looking at is almost certainly not at the
        // same offset in a shorter list, so the carousel restarts rather than
        // landing somewhere arbitrary.
        picker.at = 0;
        picker.settle();
    }

    /// Chooses a finish, if the current printing was sold in it.
    pub fn picker_set_finish(&mut self, finish: Finish) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        if picker.force_finish || picker.current().is_some_and(|p| p.has(finish)) {
            picker.finish = finish;
        }
    }

    /// Adds the picked printing to the deck and closes the dialog.
    ///
    /// Returns whether it was added: the copy limit still applies, and it
    /// applies to the *card* — four Lightning Bolts are four Lightning Bolts
    /// however many different pieces of cardboard they are.
    pub fn picker_confirm(&mut self) -> bool {
        let Some(picker) = self.picker.as_ref() else {
            return false;
        };
        let (slot, zone) = (picker.slot, picker.zone);
        let choice = self.picked_choice();
        let replacement = picker.replacing.clone();
        let added = if let Some(mut original) = replacement {
            let entries = match zone {
                Zone::Main => &mut self.main,
                Zone::Side => &mut self.side,
            };
            if let Some(at) = entries.iter().position(|e| e == &original) {
                entries.remove(at);
                original.print = choice;
                if let Some(existing) = entries.iter_mut().find(|e| {
                    e.slot == original.slot && e.print == original.print && e.note == original.note
                }) {
                    existing.count += original.count;
                } else {
                    entries.push(original);
                }
                self.sort_zone(zone);
                self.dirty = true;
                true
            } else {
                false
            }
        } else {
            self.add_print(slot, zone, choice)
        };
        self.picker = None;
        added
    }

    /// The deck row the current pick writes.
    ///
    /// Narrow, not exhaustive: a row records what the player *chose*, and a
    /// choice that changes nothing writes nothing. Picking the default
    /// printing of a card leaves `4 Lightning Bolt` exactly as it was, which
    /// is what keeps a deck built before this feature existed from growing
    /// noise the first time it is saved.
    #[must_use]
    fn picked_choice(&self) -> PrintChoice {
        let Some(picker) = self.picker.as_ref() else {
            return PrintChoice::default();
        };
        let Some(printing) = picker.current() else {
            return PrintChoice::default();
        };
        let reference = self
            .pool
            .get(picker.slot)
            .map(|c| c.scryfall_id.as_str())
            .unwrap_or_default();

        let mut choice = PrintChoice {
            finish: (picker.finish != Finish::Normal).then_some(picker.finish),
            ..PrintChoice::default()
        };
        if !printing.lang.is_empty() && printing.lang != "en" {
            choice.lang = Some(printing.lang.clone());
        }
        if !printing.set.is_empty() {
            choice.set = Some(printing.set.to_uppercase());
            if !printing.collector_number.is_empty() {
                choice.collector_number = Some(printing.collector_number.clone());
            }
        }
        if !printing.scryfall_id.is_empty() && printing.scryfall_id != reference {
            // Preserve the actual art key even with set metadata: the client
            // cannot resolve a set/number back to an image without a catalog.
            choice.scryfall_id = Some(printing.scryfall_id.clone());
        }
        choice
    }

    // ------------------------------------------------------------ the deck

    /// Which list the next add goes to.
    pub fn set_zone(&mut self, zone: Zone) {
        self.zone = zone;
    }

    /// Adds one copy, up to what the format allows.
    ///
    /// Returns whether anything changed, so a shell can say why a click did
    /// nothing rather than looking broken.
    pub fn add(&mut self, slot: usize, zone: Zone) -> bool {
        self.add_print(slot, zone, PrintChoice::default())
    }

    /// Adds one copy of a card in a printing the player chose.
    ///
    /// Two copies with different printings are two rows, because that is what
    /// a deck list says and what a collection holds — but the copy limit is
    /// on the *card*: four Lightning Bolts are four Lightning Bolts however
    /// many different pieces of cardboard they are.
    pub fn add_print(&mut self, slot: usize, zone: Zone, print: PrintChoice) -> bool {
        let Some(card) = self.pool.get(slot) else {
            return false;
        };
        let limit = if card.basic_land {
            u16::MAX
        } else {
            MAX_COPIES
        };
        // The gateway caps each list on its own, so this does too: a full
        // main deck must not be what stops a sideboard being built.
        let counts = self.counts();
        let filled = match zone {
            Zone::Main => counts.main,
            Zone::Side => counts.side,
        };
        if filled >= MAX_DECK_CARDS || self.count_of(slot, zone) >= limit {
            return false;
        }
        let entries = match zone {
            Zone::Main => &mut self.main,
            Zone::Side => &mut self.side,
        };
        if let Some(entry) = entries
            .iter_mut()
            .find(|e| e.slot == slot && e.print == print)
        {
            entry.count += 1;
        } else {
            if entries.len() >= MAX_DECK_LINES {
                return false;
            }
            entries.push(Entry {
                slot,
                count: 1,
                print,
                note: None,
            });
        }
        self.dirty = true;
        self.sort_zone(zone);
        true
    }

    /// Removes one copy, dropping the row when the last one goes.
    ///
    /// From the *last* row of that card, so it undoes the most recent add:
    /// picking a foil and then changing your mind takes the foil back, not
    /// one of the plain copies that were already there.
    pub fn remove(&mut self, slot: usize, zone: Zone) -> bool {
        let entries = match zone {
            Zone::Main => &mut self.main,
            Zone::Side => &mut self.side,
        };
        let Some(at) = entries.iter().rposition(|e| e.slot == slot) else {
            return false;
        };
        Self::take_one(entries, at);
        self.dirty = true;
        true
    }

    /// Removes one copy from a named row of the deck list.
    ///
    /// The list addresses rows, not cards: two printings of the same card are
    /// two lines, and a player tapping one of them means that one.
    pub fn remove_at(&mut self, at: usize, zone: Zone) -> bool {
        let entries = match zone {
            Zone::Main => &mut self.main,
            Zone::Side => &mut self.side,
        };
        if at >= entries.len() {
            return false;
        }
        Self::take_one(entries, at);
        self.dirty = true;
        true
    }

    /// One copy off a row, and the row itself when that was the last.
    fn take_one(entries: &mut Vec<Entry>, at: usize) {
        entries[at].count -= 1;
        if entries[at].count == 0 {
            entries.remove(at);
        }
    }

    /// Empties both lists, keeping the name and the deck being edited.
    pub fn clear_deck(&mut self) {
        self.main.clear();
        self.side.clear();
        self.commanders.clear();
        self.pending.clear();
        self.missing.clear();
        self.dirty = true;
    }

    /// Sets the deck's name.
    pub fn set_name(&mut self, name: &str) {
        self.name = crate::textbuf::TextBuffer::new(name);
        self.dirty = true;
    }

    /// Types one character into the name.
    pub fn type_name(&mut self, ch: char) {
        self.name.insert(&ch.to_string());
        self.dirty = true;
    }

    /// Deletes the last character of the name.
    pub fn backspace_name(&mut self) {
        self.name.delete_back();
        self.dirty = true;
    }

    // ----------------------------------------------------------- one card

    /// The card whose full text is being read, if any.
    #[must_use]
    pub fn inspecting(&self) -> Option<usize> {
        self.inspecting
    }

    /// Opens a card. Reading one is a separate act from adding it: on a touch
    /// screen there is no hover to read with, and a builder where a card
    /// cannot be read is not one.
    pub fn inspect(&mut self, slot: usize) {
        self.inspecting = (slot < self.pool.len()).then_some(slot);
    }

    /// Closes it again.
    pub fn stop_inspecting(&mut self) {
        self.inspecting = None;
    }

    // ------------------------------------------------------------ the caret

    /// Which box the caret is in.
    #[must_use]
    pub fn focus(&self) -> BuildField {
        self.focus
    }

    /// How many times the caret has been placed. See [`BuildField`].
    #[must_use]
    pub fn focus_epoch(&self) -> u64 {
        self.focus_epoch
    }

    /// Puts the caret in a box.
    pub fn focus_on(&mut self, field: BuildField) {
        self.focus = field;
        if field == BuildField::PickerSet
            && let Some(p) = &mut self.picker
        {
            p.set_open = true;
            p.set_query.clear();
            p.set_cursor = 0;
        }
        self.focus_epoch = self.focus_epoch.wrapping_add(1);
    }

    /// Moves the caret to the other box.
    pub fn cycle_focus(&mut self) {
        self.focus_on(match self.focus {
            BuildField::Search => BuildField::Name,
            BuildField::Name | BuildField::PickerSet => BuildField::Search,
        });
    }

    /// What the focused box holds.
    #[must_use]
    pub fn focused_text(&self) -> &str {
        match self.focus {
            BuildField::Search => self.text.text(),
            BuildField::Name => self.name.text(),
            BuildField::PickerSet => self.buffer(BuildField::PickerSet).text(),
        }
    }

    /// Replaces the focused box wholesale, as a platform text field does.
    pub fn set_focused(&mut self, value: &str) {
        match self.focus {
            BuildField::Search => self.set_text(value),
            BuildField::Name => self.set_name(value),
            BuildField::PickerSet => self.edit_buffer(BuildField::PickerSet, |b| {
                b.clear();
                b.insert(value);
            }),
        }
    }

    /// The field's real caret/selection, shared by native and browser editing.
    #[must_use]
    pub fn buffer(&self, field: BuildField) -> &crate::textbuf::TextBuffer {
        match field {
            BuildField::Search => &self.text,
            BuildField::Name => &self.name,
            BuildField::PickerSet => self.picker.as_ref().map_or(&self.text, |p| &p.set_query),
        }
    }

    /// Apply one batch of input and filter only when the text actually changes.
    pub fn edit_buffer(
        &mut self,
        field: BuildField,
        edit: impl FnOnce(&mut crate::textbuf::TextBuffer),
    ) {
        let buffer = match field {
            BuildField::Search => &mut self.text,
            BuildField::Name => &mut self.name,
            BuildField::PickerSet => {
                let Some(p) = &mut self.picker else {
                    return;
                };
                &mut p.set_query
            }
        };
        let before = buffer.text().to_owned();
        edit(buffer);
        if buffer.text() == before {
            return;
        }
        match field {
            BuildField::Search => self.retext(),
            BuildField::Name => self.dirty = true,
            BuildField::PickerSet => {
                if let Some(p) = &mut self.picker {
                    p.set_cursor = 0;
                    p.set_open = true;
                }
            }
        }
    }

    /// Types one character into the focused box.
    pub fn type_focused(&mut self, ch: char) {
        match self.focus {
            BuildField::Search => self.type_char(ch),
            BuildField::Name => self.type_name(ch),
            BuildField::PickerSet => {
                self.edit_buffer(BuildField::PickerSet, |b| b.insert(&ch.to_string()));
            }
        }
    }

    /// Deletes the last character of the focused box.
    pub fn backspace_focused(&mut self) {
        match self.focus {
            BuildField::Search => self.backspace(),
            BuildField::Name => self.backspace_name(),
            BuildField::PickerSet => self.edit_buffer(BuildField::PickerSet, |b| {
                b.delete_back();
            }),
        }
    }

    /// Keeps a zone in deck-list order: by group, then cost, then name.
    /// Files a zone's entries the way a deck list is printed.
    ///
    /// Same-card entries sort next to each other and then by the printing, so
    /// the plain copies come before the foils and the order does not shuffle
    /// between saves.
    fn sort_zone(&mut self, zone: Zone) {
        let pool = &self.pool;
        let entries = match zone {
            Zone::Main => &mut self.main,
            Zone::Side => &mut self.side,
        };
        entries.sort_by(|a, b| {
            let (x, y) = (&pool[a.slot], &pool[b.slot]);
            x.group()
                .cmp(&y.group())
                .then_with(|| x.cmc.cmp(&y.cmc))
                .then_with(|| x.name.cmp(&y.name))
                .then_with(|| print_key(&a.print).cmp(&print_key(&b.print)))
        });
    }

    // ------------------------------------------------------- what it adds up to

    /// What the deck adds up to.
    #[must_use]
    pub fn counts(&self) -> Counts {
        let mut counts = Counts::default();
        for entry in &self.main {
            let Some(card) = self.pool.get(entry.slot) else {
                continue;
            };
            let n = u32::from(entry.count);
            counts.main += n;
            if card.is("Land") {
                counts.lands += n;
            } else if card.is("Creature") {
                counts.creatures += n;
            } else {
                counts.spells += n;
            }
            if !card.coverage.trustworthy() {
                counts.shaky += n;
            }
        }
        for entry in &self.side {
            counts.side += u32::from(entry.count);
        }
        counts
    }

    /// The mana curve of the deck's non-land cards. The last bucket is
    /// "that mana value or more".
    #[must_use]
    pub fn curve(&self) -> [u16; CURVE_BUCKETS] {
        let mut curve = [0u16; CURVE_BUCKETS];
        for entry in &self.main {
            if let Some(card) = self.pool.get(entry.slot)
                && let Some(bucket) = card.bucket()
            {
                curve[bucket] = curve[bucket].saturating_add(entry.count);
            }
        }
        curve
    }

    /// Coloured mana symbols in the deck's costs, in `WUBRG` order — the
    /// number a mana base is actually built against.
    #[must_use]
    pub fn pips(&self) -> [u16; 5] {
        let mut pips = [0u16; 5];
        for entry in &self.main {
            let Some(card) = self.pool.get(entry.slot) else {
                continue;
            };
            for symbol in card.mana_cost.chars() {
                if let Some(at) = "WUBRG".find(symbol) {
                    pips[at] = pips[at].saturating_add(entry.count);
                }
            }
        }
        pips
    }

    /// Everything worth telling the player, refusals first.
    ///
    /// The blocking half is exactly what `POST /decks` enforces. Keeping the
    /// two in step is what lets the save button be trusted: if it is live, the
    /// deck saves.
    #[must_use]
    pub fn problems(&self, lang: Lang) -> Vec<Problem> {
        let mut out = Vec::new();
        let counts = self.counts();
        let name = self.name.text().trim();
        if name.is_empty() {
            out.push(Problem {
                blocking: true,
                message: Phrase::DeckNeedsName.text(lang).to_string(),
            });
        } else if name.len() > 64 {
            out.push(Problem {
                blocking: true,
                message: Phrase::DeckNameTooLong.text(lang).to_string(),
            });
        }
        if self.main.is_empty() {
            out.push(Problem {
                blocking: true,
                message: Phrase::DeckIsEmpty.text(lang).to_string(),
            });
        }
        if self.main.len() > MAX_DECK_LINES || self.side.len() > MAX_DECK_LINES {
            out.push(Problem {
                blocking: true,
                message: Phrase::TooManyLines.fill(lang, &[&MAX_DECK_LINES.to_string()]),
            });
        }
        if counts.main > MAX_DECK_CARDS || counts.side > MAX_DECK_CARDS {
            out.push(Problem {
                blocking: true,
                message: Phrase::TooManyCards.fill(lang, &[&MAX_DECK_CARDS.to_string()]),
            });
        }
        for name in &self.missing {
            out.push(Problem {
                blocking: true,
                message: Phrase::CardGoneFromPool.fill(lang, &[name]),
            });
        }
        // Advice from here down. None of it stops a save.
        if let Some(name) = &self.stale_commander {
            // Advice rather than a block, and the wording says what will
            // happen rather than asking for something. There is no button
            // that puts this mark back — the card is not one the `?` menu
            // offers a commander chip on, which is the whole reason it is
            // stale — so blocking here would be a deck that cannot be saved
            // and cannot be fixed either. Naming another commander clears
            // it; saving as it stands is a Freeform deck, and the player is
            // told so before they press the button rather than at the table.
            out.push(Problem {
                blocking: false,
                message: Phrase::CommanderNoLongerEligible.fill(lang, &[name]),
            });
        }
        if counts.main > 0 && counts.main < MIN_CONSTRUCTED {
            out.push(Problem {
                blocking: false,
                message: Phrase::DeckTooSmall.fill(
                    lang,
                    &[&counts.main.to_string(), &MIN_CONSTRUCTED.to_string()],
                ),
            });
        }
        if counts.side > MAX_SIDEBOARD {
            out.push(Problem {
                blocking: false,
                message: Phrase::SideboardTooBig.fill(lang, &[&MAX_SIDEBOARD.to_string()]),
            });
        }
        if counts.main >= MIN_CONSTRUCTED && counts.lands * 3 < counts.main {
            out.push(Problem {
                blocking: false,
                message: Phrase::ThinOnLands
                    .fill(lang, &[&counts.lands.to_string(), &counts.main.to_string()]),
            });
        }
        if counts.shaky > 0 {
            out.push(Problem {
                blocking: false,
                message: Phrase::counted(
                    counts.shaky as usize,
                    Phrase::ShakyCard,
                    Phrase::ShakyCards,
                )
                .fill(lang, &[&counts.shaky.to_string()]),
            });
        }
        out
    }

    /// Whether the deck would save.
    #[must_use]
    pub fn saveable(&self) -> bool {
        // English, because nothing is shown: only whether the list is empty
        // is being asked, and that answer is the same in every language.
        !self.problems(Lang::En).iter().any(|p| p.blocking)
    }

    // ------------------------------------------------------- the commander

    /// The deck's commanders, as slots in the pool.
    ///
    /// Two of them under the partner rule (CR 702.124), and the order is the
    /// order they were named in.
    #[must_use]
    pub fn commanders(&self) -> &[usize] {
        &self.commanders
    }

    /// Whether this slot is one of them.
    #[must_use]
    pub fn is_commander(&self, slot: usize) -> bool {
        self.commanders.contains(&slot)
    }

    /// The deck row that holds this commander, as an index into
    /// `entries(Zone::Main)`, whichever list is on screen.
    ///
    /// The commander box draws the printing this row names and opens the
    /// picker on it, as a deck row does for itself. `None` when the card is
    /// not in the deck, for instance after it was moved to the sideboard.
    #[must_use]
    pub fn commander_row(&self, slot: usize) -> Option<usize> {
        self.main.iter().position(|entry| entry.slot == slot)
    }

    /// The commanders' English names — what the gateway is told.
    #[must_use]
    pub fn commander_names(&self) -> Vec<String> {
        self.commanders
            .iter()
            .filter_map(|slot| self.pool.get(*slot))
            .map(|card| card.english_name.clone())
            .collect()
    }

    /// Makes a card the deck's commander.
    ///
    /// Refused for a card the rules cannot seat as one — the pool says which,
    /// and offering the choice on a card that would be rejected on save is
    /// worse than not offering it.
    ///
    /// A commander is also a card in the deck, so this puts one there if it
    /// is not already: choosing a leader that is not in the ninety-nine is a
    /// deck nobody meant to build.
    ///
    /// Replaces the commander selection. Use `add_partner` to keep the first leader.
    pub fn set_commander(&mut self, slot: usize) -> bool {
        if !self.pool.get(slot).is_some_and(|card| card.commander) {
            return false;
        }
        if self.commanders == [slot] {
            return true;
        }
        if self.count_of(slot, Zone::Main) == 0 && !self.add(slot, Zone::Main) {
            return false;
        }
        self.commanders = vec![slot];
        self.dirty = true;
        self.stale_commander = None;
        true
    }

    /// Whether the server permits this card beside the selected commander.
    #[must_use]
    pub fn can_partner(&self, slot: usize) -> bool {
        let [first] = self.commanders.as_slice() else {
            return false;
        };
        let (Some(a), Some(b)) = (self.card(*first), self.card(slot)) else {
            return false;
        };
        *first != slot && b.commander && a.partners.contains(&b.index)
    }

    /// Add a compatible second commander without replacing the first.
    pub fn add_partner(&mut self, slot: usize) -> bool {
        if !self.can_partner(slot) {
            return false;
        }
        if self.count_of(slot, Zone::Main) == 0 && !self.add(slot, Zone::Main) {
            return false;
        }
        self.commanders.push(slot);
        self.dirty = true;
        true
    }

    /// Remove only one role, keeping both the other commander and the deck cards.
    pub fn remove_commander(&mut self, slot: usize) {
        let before = self.commanders.len();
        self.commanders.retain(|leader| *leader != slot);
        self.dirty |= before != self.commanders.len();
    }

    /// Takes every commander mark off, leaving the cards in the deck.
    pub fn clear_commander(&mut self) {
        // Both taken, then asked: `||` would short-circuit past the second
        // one and leave a warning standing about a deck that no longer has
        // the mark it is about.
        let marked = !std::mem::take(&mut self.commanders).is_empty();
        let stale = self.stale_commander.take().is_some();
        if marked || stale {
            self.dirty = true;
        }
    }

    // -------------------------------------------------- between the zones

    /// Moves one copy of an entry to the other zone, printing and all.
    ///
    /// Not remove-then-add at the call site, because that would drop the
    /// chosen printing: a foil moved to the sideboard has to arrive as the
    /// same piece of cardboard it left as.
    pub fn move_entry(&mut self, at: usize, from: Zone, to: Zone) -> bool {
        if from == to {
            return false;
        }
        let Some(entry) = self.entries(from).get(at) else {
            return false;
        };
        let (slot, print) = (entry.slot, entry.print.clone());
        if !self.remove_at(at, from) {
            return false;
        }
        self.add_print(slot, to, print)
    }

    // ---------------------------------------------------------- the wire

    /// One zone as the `"N Card Name"` rows the gateway stores.
    ///
    /// Always the English name: a deck saved by a player reading German has to
    /// be the same deck when the gateway resolves it against the registry.
    /// The deck as rows, in the form `docs/deck-format.md` specifies.
    ///
    /// This is the stored form *and* the exported form: what comes out here
    /// is what a player can paste into a text file, and what
    /// `baylee_core::deckrow::parse` reads back is this deck. A printing the
    /// player chose travels with the row.
    #[must_use]
    pub fn rows(&self, zone: Zone) -> Vec<String> {
        self.entries(zone)
            .iter()
            .filter_map(|entry| {
                let card = self.pool.get(entry.slot)?;
                Some(
                    Row {
                        count: u32::from(entry.count),
                        name: card.english_name.clone(),
                        print: entry.print.clone(),
                        note: entry.note.clone(),
                    }
                    .to_string(),
                )
            })
            .collect()
    }

    /// The request that saves this deck, or `None` when it would be refused.
    #[must_use]
    pub fn save(&self) -> Option<LobbyRequest> {
        if !self.saveable() {
            return None;
        }
        Some(LobbyRequest::SaveDeck {
            deck_id: self.editing.clone(),
            name: self.name.text().trim().to_string(),
            cards: self.rows(Zone::Main),
            sideboard: self.rows(Zone::Side),
            commanders: self.commander_names(),
        })
    }

    /// Marks the deck as saved.
    pub fn saved(&mut self, deck_id: Option<&str>) {
        self.dirty = false;
        // A new deck becomes the deck being edited the moment it has an id.
        // Without this the next save would post it a second time, and the
        // player would find two decks where they saved one.
        if let Some(id) = deck_id {
            self.editing = Some(id.to_string());
        }
    }

    /// Starts a new, empty deck.
    pub fn start_new(&mut self) {
        self.main.clear();
        self.side.clear();
        self.commanders.clear();
        self.pending.clear();
        self.missing.clear();
        self.name.clear();
        self.editing = None;
        self.zone = Zone::Main;
        self.dirty = false;
        self.inspecting = None;
        self.commanders.clear();
        self.pending_commander.clear();
        self.stale_commander = None;
        // A nameless deck cannot be saved, so that is where the caret starts.
        self.focus_on(BuildField::Name);
    }

    /// Loads a stored deck for editing.
    ///
    /// The rows are card *names*; resolving them takes the pool, and the pool
    /// may not have arrived yet — the two requests race. So a row that cannot
    /// be resolved is held, not dropped, and [`DeckBuilder::set_pool`] tries
    /// again. What is still unresolved once the pool is here is genuinely
    /// missing, and [`DeckBuilder::problems`] refuses to save over it: losing
    /// a card silently is the one outcome a deck builder must not have.
    pub fn load(
        &mut self,
        id: &str,
        name: &str,
        cards: &[String],
        sideboard: &[String],
        commanders: &[String],
    ) {
        self.start_new();
        self.editing = Some(id.to_string());
        self.name = crate::textbuf::TextBuffer::new(name);
        // A commander is a name too, and races the pool the same way its
        // rows do.
        self.pending_commander = commanders.to_vec();
        for (rows, zone) in [(cards, Zone::Main), (sideboard, Zone::Side)] {
            for row in rows {
                match baylee_core::deckrow::parse(row) {
                    // The printing travels with the row: a deck reopened and
                    // saved again has to come back out the way it went in, or
                    // editing one line would quietly strip every other line's
                    // foils.
                    Ok(parsed) => self.pending.push(Held {
                        count: u16::try_from(parsed.count).unwrap_or(u16::MAX),
                        name: parsed.name,
                        zone,
                        print: parsed.print,
                        note: parsed.note,
                    }),
                    // A malformed row will never resolve, whatever the pool
                    // holds, so it is missing right away.
                    Err(_) => self.missing.push(row.clone()),
                }
            }
        }
        self.resolve_pending();
        self.dirty = false;
        // This one already has a name; what is wanted is the next card.
        self.focus_on(BuildField::Search);
    }

    /// Turns held rows into deck entries, as far as the pool allows.
    fn resolve_pending(&mut self) {
        for name in std::mem::take(&mut self.pending_commander) {
            let Some(slot) = self.slot_of(&name) else {
                // No pool yet, or no such card: keep holding the name.
                self.pending_commander.push(name);
                continue;
            };
            // The same question `set_commander` asks, and for the same
            // reason: a mark the rules will not seat is one the gateway
            // refuses on save. Asking it in only one of the two places meant
            // a deck could be *loaded* into a state it could never be saved
            // from — and the mark that got there that way looked exactly
            // like one the player had chosen.
            if self.pool.get(slot).is_some_and(|card| card.commander) {
                // Pushed rather than `set_commander`ed: a stored deck's
                // leaders are already in its rows, and re-adding them here
                // would put a second copy of each in the library. The pair's
                // legality is the gateway's to refuse on save; a deck that
                // is already stored is not the place to start arguing about
                // it, or reopening one would silently drop a commander.
                if !self.commanders.contains(&slot) {
                    self.commanders.push(slot);
                }
            } else {
                self.stale_commander = Some(name);
            }
        }
        if self.pending.is_empty() {
            return;
        }
        let held = std::mem::take(&mut self.pending);
        for Held {
            count,
            name,
            zone,
            print,
            note,
        } in held
        {
            match self.slot_of(&name) {
                Some(slot) => {
                    let entries = match zone {
                        Zone::Main => &mut self.main,
                        Zone::Side => &mut self.side,
                    };
                    // Rows merge only when they name the same printing; two
                    // that do not are two lines in the list they came from.
                    match entries
                        .iter_mut()
                        .find(|e| e.slot == slot && e.print == print)
                    {
                        Some(entry) => entry.count = entry.count.saturating_add(count),
                        None => entries.push(Entry {
                            slot,
                            count,
                            print,
                            note,
                        }),
                    }
                }
                None if self.loaded() => self.missing.push(name),
                // No pool yet: keep holding it.
                None => self.pending.push(Held {
                    count,
                    name,
                    zone,
                    print,
                    note,
                }),
            }
        }
        self.sort_zone(Zone::Main);
        self.sort_zone(Zone::Side);
    }
}

/// A printing choice as something sortable.
///
/// `PrintChoice` is a bag of options with no natural order; a deck list needs
/// one, or two saves of the same deck would differ only in row order.
fn print_key(print: &PrintChoice) -> (String, String, String, u8) {
    (
        print.set.clone().unwrap_or_default(),
        print.collector_number.clone().unwrap_or_default(),
        print.lang.clone().unwrap_or_default(),
        match print.finish.unwrap_or_default() {
            Finish::Normal => 0,
            Finish::Foil => 1,
            Finish::Etched => 2,
            Finish::Holographic => 3,
            Finish::Glitter => 4,
            Finish::Galaxy => 5,
        },
    )
}

/// The flags a pool row carries, for `is:`.
///
/// A list and not a set of `bool` fields: [`Facts::flags`] is what a card
/// *is*, and [`Surface::POOL`] is what may be asked — the pair is how
/// "this card is not a commander" and "nobody here knows about commanders"
/// stay two different answers.
fn pool_flags(card: &PoolCard) -> Vec<Flag> {
    let mut flags = Vec::with_capacity(4);
    flags.push(match card.coverage {
        Coverage::Implemented => Flag::Playable,
        Coverage::Partial => Flag::Partial,
        Coverage::Unimplemented => Flag::Stub,
    });
    if card.commander {
        flags.push(Flag::Commander);
    }
    if card.basic_land {
        flags.push(Flag::Basic);
    }
    // The rules question, not the picture one: this filter is printed to the
    // player as "double-faced" and reached by the alias `transform`, and
    // CR 715.1 makes an adventurer card a two-part frame rather than a second
    // side. It asked `two_faced` — the compiled face count — until #115, and
    // answered with nine Adventures and two Splits.
    if card.double_faced {
        flags.push(Flag::Dfc);
    }
    flags
}

/// A pool row, as the query language reads it.
fn pool_facts<'a>(card: &'a PoolCard, flags: &'a [Flag]) -> Facts<'a> {
    let (power, toughness, loyalty) = printed_numbers(card.stats.as_deref());
    Facts {
        name: &card.name,
        english_name: &card.english_name,
        alt_names: &card.alt_names,
        type_line: &card.type_line,
        kinds: &card.kinds,
        oracle: &card.oracle_text,
        colors: Colors::of_letters(&card.colors),
        identity: Colors::of_letters(&card.identity),
        mana_cost: &card.mana_cost,
        mana_value: card.cmc,
        power,
        toughness,
        loyalty,
        flags,
    }
}

/// The numbers `PoolCard::stats` is holding, told apart by their shape.
///
/// `baylee_cards::pool::stats` writes `p/t` for anything with a power and a
/// toughness and the bare number otherwise, so the slash is the whole of
/// what distinguishes a creature from a planeswalker here. Reading it back
/// rather than asking the pool for three fields keeps the wire as it is;
/// what it costs is that a card printing both — none in this pool — would
/// be read as a creature, which is what the row already draws it as.
fn printed_numbers(stats: Option<&str>) -> (Option<i32>, Option<i32>, Option<i32>) {
    let Some(stats) = stats else {
        return (None, None, None);
    };
    match stats.split_once('/') {
        Some((power, toughness)) => (
            power.trim().parse().ok(),
            toughness.trim().parse().ok(),
            None,
        ),
        None => (None, None, stats.trim().parse().ok()),
    }
}
