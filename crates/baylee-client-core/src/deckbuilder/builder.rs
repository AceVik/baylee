//! What a deck builder *does* with its pool: search, filter, group, and
//! the two lists it maintains.
//!
//! [`DeckBuilder::problems`] is the half worth reading first — it mirrors
//! what `POST \/decks` enforces, split into blocking and advisory, so a live
//! save button means the deck will save.

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;

impl DeckBuilder {
    /// An empty builder with no pool yet.
    #[must_use]
    pub fn new() -> Self {
        Self {
            playable_only: true,
            name: String::new(),
            ..Self::default()
        }
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
        &self.text
    }

    /// The deck's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
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
    pub fn playable_only(&self) -> bool {
        self.playable_only
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

    /// Takes the pool and rebuilds the results.
    pub fn set_pool(&mut self, cards: Vec<PoolCard>, has_text: bool) {
        self.pool = cards;
        self.has_text = has_text;
        // A deck may have been loaded before the pool arrived; its rows were
        // held by name and become real entries now.
        self.resolve_pending();
        self.refilter();
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
        self.text = text.to_string();
        self.refilter();
    }

    /// Types one character into the search box.
    pub fn type_char(&mut self, ch: char) {
        self.text.push(ch);
        self.refilter();
    }

    /// Deletes the last character of the search box.
    pub fn backspace(&mut self) {
        self.text.pop();
        self.refilter();
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
        self.playable_only = !self.playable_only;
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
        self.refilter();
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
        let needle = self.text.trim().to_lowercase();
        let mut hits: Vec<usize> = (0..self.pool.len())
            .filter(|slot| self.matches(&self.pool[*slot], &needle))
            .collect();
        let sort = self.sort;
        // `sort_by` rather than `sort_unstable_by`: every comparison ends in
        // the name, so the order is total, but a stable sort keeps it obvious
        // that two runs of the same filter cannot disagree.
        hits.sort_by(|a, b| {
            let (x, y) = (&self.pool[*a], &self.pool[*b]);
            match sort {
                Sort::Name => x.name.cmp(&y.name),
                Sort::Cost => x.cmc.cmp(&y.cmc).then_with(|| x.name.cmp(&y.name)),
                Sort::Type => x
                    .group()
                    .cmp(&y.group())
                    .then_with(|| x.cmc.cmp(&y.cmc))
                    .then_with(|| x.name.cmp(&y.name)),
            }
        });
        self.results = hits;
    }

    /// Whether one card survives the current filter.
    fn matches(&self, card: &PoolCard, needle: &str) -> bool {
        if self.playable_only && card.coverage == Coverage::Unimplemented {
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
        if needle.is_empty() {
            return true;
        }
        // Every name the card answers to, in every language it was printed
        // in. A player searching for their own copy types what is on it.
        card.name.to_lowercase().contains(needle)
            || card.english_name.to_lowercase().contains(needle)
            || card
                .alt_names
                .iter()
                .any(|n| n.to_lowercase().contains(needle))
            || card.type_line.to_lowercase().contains(needle)
            || card.oracle_text.to_lowercase().contains(needle)
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
            ..Printing::default()
        };
        let index = card.index;
        self.picker = Some(Picker {
            slot,
            zone,
            card: index,
            langs: vec!["en".to_string()],
            printings: vec![reference],
            loading: true,
            ..Picker::default()
        });
        Some(LobbyRequest::LoadPrintings { card: index })
    }

    /// Closes the picker without adding anything.
    pub fn close_picker(&mut self) {
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
        picker.printings = printings;
        picker.langs = langs;
        picker.at = 0;
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
        if picker.current().is_some_and(|p| p.has(finish)) {
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
        let added = self.add_print(slot, zone, choice);
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
        } else if !printing.scryfall_id.is_empty() && printing.scryfall_id != reference {
            // No set to name it by, and not the printing the row would
            // resolve to anyway: the id is the only thing that pins it.
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
        self.pending.clear();
        self.missing.clear();
        self.dirty = true;
    }

    /// Sets the deck's name.
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
        self.dirty = true;
    }

    /// Types one character into the name.
    pub fn type_name(&mut self, ch: char) {
        self.name.push(ch);
        self.dirty = true;
    }

    /// Deletes the last character of the name.
    pub fn backspace_name(&mut self) {
        self.name.pop();
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
        self.focus_epoch = self.focus_epoch.wrapping_add(1);
    }

    /// Moves the caret to the other box.
    pub fn cycle_focus(&mut self) {
        self.focus_on(match self.focus {
            BuildField::Search => BuildField::Name,
            BuildField::Name => BuildField::Search,
        });
    }

    /// What the focused box holds.
    #[must_use]
    pub fn focused_text(&self) -> &str {
        match self.focus {
            BuildField::Search => &self.text,
            BuildField::Name => &self.name,
        }
    }

    /// Replaces the focused box wholesale, as a platform text field does.
    pub fn set_focused(&mut self, value: &str) {
        match self.focus {
            BuildField::Search => self.set_text(value),
            BuildField::Name => self.set_name(value),
        }
    }

    /// Types one character into the focused box.
    pub fn type_focused(&mut self, ch: char) {
        match self.focus {
            BuildField::Search => self.type_char(ch),
            BuildField::Name => self.type_name(ch),
        }
    }

    /// Deletes the last character of the focused box.
    pub fn backspace_focused(&mut self) {
        match self.focus {
            BuildField::Search => self.backspace(),
            BuildField::Name => self.backspace_name(),
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
        let name = self.name.trim();
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
                message: Phrase::ShakyCards.fill(lang, &[&counts.shaky.to_string()]),
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

    /// The deck's commander, as a slot in the pool.
    #[must_use]
    pub fn commander(&self) -> Option<usize> {
        self.commander
    }

    /// The commander's English name — what the gateway is told.
    #[must_use]
    pub fn commander_name(&self) -> Option<&str> {
        self.commander
            .and_then(|slot| self.pool.get(slot))
            .map(|card| card.english_name.as_str())
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
    pub fn set_commander(&mut self, slot: usize) -> bool {
        if !self.pool.get(slot).is_some_and(|card| card.commander) {
            return false;
        }
        if self.count_of(slot, Zone::Main) == 0 {
            self.add(slot, Zone::Main);
        }
        if self.commander != Some(slot) {
            self.commander = Some(slot);
            self.dirty = true;
        }
        true
    }

    /// Takes the commander mark off, leaving the card in the deck.
    pub fn clear_commander(&mut self) {
        if self.commander.take().is_some() {
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
            name: self.name.trim().to_string(),
            cards: self.rows(Zone::Main),
            sideboard: self.rows(Zone::Side),
            commander: self.commander_name().map(ToString::to_string),
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
        self.pending.clear();
        self.missing.clear();
        self.name.clear();
        self.editing = None;
        self.zone = Zone::Main;
        self.dirty = false;
        self.inspecting = None;
        self.commander = None;
        self.pending_commander = None;
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
        commander: Option<&str>,
    ) {
        self.start_new();
        self.editing = Some(id.to_string());
        self.name = name.to_string();
        // The commander is a name too, and races the pool the same way its
        // rows do.
        self.pending_commander = commander.map(ToString::to_string);
        for (rows, zone) in [(cards, Zone::Main), (sideboard, Zone::Side)] {
            for row in rows {
                match baylee_core::deckrow::parse(row) {
                    // The printing travels with the row: a deck reopened and
                    // saved again has to come back out the way it went in, or
                    // editing one line would quietly strip every other line's
                    // foils.
                    Ok(parsed) => self.pending.push((
                        u16::try_from(parsed.count).unwrap_or(u16::MAX),
                        parsed.name,
                        zone,
                        parsed.print,
                    )),
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
        if let Some(name) = self.pending_commander.clone()
            && let Some(slot) = self.slot_of(&name)
        {
            self.commander = Some(slot);
            self.pending_commander = None;
        }
        if self.pending.is_empty() {
            return;
        }
        let held = std::mem::take(&mut self.pending);
        for (count, name, zone, print) in held {
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
                        None => entries.push(Entry { slot, count, print }),
                    }
                }
                None if self.loaded() => self.missing.push(name),
                // No pool yet: keep holding it.
                None => self.pending.push((count, name, zone, print)),
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
        },
    )
}
