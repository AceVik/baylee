//! What the builder's deck side and Stats tab read (the shell design, §7):
//! the deck list in sections by type, mana value or colour; the curve with
//! one type lit; the type counts; a sample opening hand; the local draft the
//! builder keeps while a deck is unsaved; the credit of a printing the
//! builder has already been told about.
//!
//! All of it is arithmetic over the two lists, so it is tested here, without
//! a window.

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;

/// How the deck list is sectioned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Grouping {
    /// Creatures, instants, … lands: the way a deck list is printed.
    #[default]
    Type,
    /// By mana value; lands in a section of their own, last.
    ManaValue,
    /// By colour: each single colour, then multicoloured, colourless, lands.
    Colour,
}

impl Grouping {
    /// The three, in the switch's order.
    pub const ALL: [Self; 3] = [Self::Type, Self::ManaValue, Self::Colour];

    /// The switch's label.
    #[must_use]
    pub fn label(self) -> Phrase {
        match self {
            Self::Type => Phrase::BuildByType,
            Self::ManaValue => Phrase::BuildByManaValue,
            Self::Colour => Phrase::BuildByColour,
        }
    }
}

/// A section of the deck list: what it is filed under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SectionKey {
    /// A card-type section.
    Type(Group),
    /// A mana value (the last bucket is "that or more").
    Mana(u8),
    /// One colour, by its place in `WUBRG`.
    Colour(u8),
    /// Two colours or more.
    Multicolour,
    /// No colour, and not a land.
    Colourless,
    /// Lands, under mana value or colour.
    Lands,
}

/// One section of the deck list: its key, how many cards it holds, and the
/// rows in it as indices into [`DeckBuilder::entries`] — the address every
/// row press uses, so a section changes the order rows are drawn in and
/// nothing about which row a press means.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    /// What the section is filed under.
    pub key: SectionKey,
    /// Cards in it (copies, not rows).
    pub cards: u32,
    /// Its rows, as indices into `entries(zone)`.
    pub rows: Vec<usize>,
}

impl SectionKey {
    /// The section's heading, in the player's words.
    #[must_use]
    pub fn heading(self, lang: Lang) -> String {
        match self {
            Self::Type(group) => group.label().text(lang).to_string(),
            Self::Mana(mv) if usize::from(mv) + 1 == CURVE_BUCKETS => {
                Phrase::BuildManaValueUp.fill(lang, &[&mv.to_string()])
            }
            Self::Mana(mv) => Phrase::BuildManaValue.fill(lang, &[&mv.to_string()]),
            Self::Colour(at) => [
                Phrase::ColorWhite,
                Phrase::ColorBlue,
                Phrase::ColorBlack,
                Phrase::ColorRed,
                Phrase::ColorGreen,
            ]
            .get(usize::from(at))
            .map_or_else(String::new, |p| p.text(lang).to_string()),
            Self::Multicolour => Phrase::BuildMulticolour.text(lang).to_string(),
            Self::Colourless => Phrase::ColorColourless.text(lang).to_string(),
            Self::Lands => Phrase::GroupLands.text(lang).to_string(),
        }
    }
}

/// The section a card is filed under.
fn section_of(card: &PoolCard, grouping: Grouping) -> SectionKey {
    match grouping {
        Grouping::Type => SectionKey::Type(card.group()),
        Grouping::ManaValue => card.bucket().map_or(SectionKey::Lands, |bucket| {
            SectionKey::Mana(u8::try_from(bucket).unwrap_or(u8::MAX))
        }),
        Grouping::Colour => {
            if card.is("Land") {
                return SectionKey::Lands;
            }
            let colours: Vec<u8> = card
                .colors
                .chars()
                .filter_map(|c| "WUBRG".find(c))
                .filter_map(|at| u8::try_from(at).ok())
                .collect();
            match colours.as_slice() {
                [] => SectionKey::Colourless,
                [one] => SectionKey::Colour(*one),
                _ => SectionKey::Multicolour,
            }
        }
    }
}

/// The type groups the Stats tab counts and its chips light, in order.
pub const STAT_GROUPS: [Group; 6] = [
    Group::Creature,
    Group::Instant,
    Group::Sorcery,
    Group::Artifact,
    Group::Enchantment,
    Group::Land,
];

impl DeckBuilder {
    /// One zone's rows in sections, sections in their printed order and
    /// rows in deck-list order inside each.
    #[must_use]
    pub fn sections(&self, zone: Zone, grouping: Grouping) -> Vec<Section> {
        let mut out: Vec<Section> = Vec::new();
        for (at, entry) in self.entries(zone).iter().enumerate() {
            let Some(card) = self.pool.get(entry.slot) else {
                continue;
            };
            let key = section_of(card, grouping);
            if let Some(section) = out.iter_mut().find(|s| s.key == key) {
                section.cards += u32::from(entry.count);
                section.rows.push(at);
            } else {
                out.push(Section {
                    key,
                    cards: u32::from(entry.count),
                    rows: vec![at],
                });
            }
        }
        out.sort_by_key(|s| s.key);
        out
    }

    /// The main deck's curve, each bucket as (all nonland cards, those of
    /// `lit`'s type): the Stats tab's bars with one type lit.
    #[must_use]
    pub fn curve_lit(&self, lit: Option<Group>) -> [(u16, u16); CURVE_BUCKETS] {
        let mut curve = [(0u16, 0u16); CURVE_BUCKETS];
        for entry in &self.main {
            if let Some(card) = self.pool.get(entry.slot)
                && let Some(bucket) = card.bucket()
            {
                let bar = &mut curve[bucket];
                bar.0 = bar.0.saturating_add(entry.count);
                if lit.is_some_and(|group| card.group() == group) {
                    bar.1 = bar.1.saturating_add(entry.count);
                }
            }
        }
        curve
    }

    /// How many main-deck cards each group of [`STAT_GROUPS`] holds, with
    /// planeswalkers and battles and the rest counted where a deck list
    /// would print them (a planeswalker is not a creature).
    #[must_use]
    pub fn group_counts(&self) -> Vec<(Group, u32)> {
        let mut out: Vec<(Group, u32)> = Vec::new();
        for entry in &self.main {
            let Some(card) = self.pool.get(entry.slot) else {
                continue;
            };
            let group = card.group();
            if let Some(found) = out.iter_mut().find(|(g, _)| *g == group) {
                found.1 += u32::from(entry.count);
            } else {
                out.push((group, u32::from(entry.count)));
            }
        }
        for group in STAT_GROUPS {
            if !out.iter().any(|(g, _)| *g == group) {
                out.push((group, 0));
            }
        }
        out.sort_by_key(|(g, _)| *g);
        out
    }

    /// Seven cards of the main deck, drawn without replacement from a fresh
    /// shuffle seeded by `seed`: the Stats tab's "Draw seven". Nothing
    /// leaves the deck. Fewer than seven when the deck holds fewer.
    #[must_use]
    pub fn sample_hand(&self, seed: u64) -> Vec<usize> {
        let mut library: Vec<usize> = self
            .main
            .iter()
            .flat_map(|entry| std::iter::repeat_n(entry.slot, usize::from(entry.count)))
            .collect();
        let mut state = seed;
        // Fisher–Yates over a splitmix64 stream: the client's own shuffle,
        // nothing a rules path reads.
        for i in (1..library.len()).rev() {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            let j = usize::try_from(z % (i as u64 + 1)).unwrap_or(0);
            library.swap(i, j);
        }
        library.truncate(7);
        library
    }

    /// The set and artist of a printing the builder has been told about —
    /// the open picker's, or one `GET /printings` answered earlier — for a
    /// text face's foot (WP6). `scryfall_id` empty means the card's own
    /// printing, the pool row's.
    #[must_use]
    pub fn printing_credit(
        &self,
        index: u32,
        scryfall_id: &str,
    ) -> Option<crate::card_face::Credit> {
        let wanted = if scryfall_id.is_empty() {
            self.pool
                .iter()
                .find(|c| c.index == index)
                .map(|c| c.scryfall_id.as_str())?
        } else {
            scryfall_id
        };
        let from_picker = self
            .picker
            .as_ref()
            .filter(|p| p.card == index)
            .and_then(|p| p.printings.iter().find(|p| p.scryfall_id == wanted));
        let printing = from_picker.or_else(|| {
            self.printings_cache
                .get(&index)
                .and_then(|(prints, _)| prints.iter().find(|p| p.scryfall_id == wanted))
        })?;
        Some(crate::card_face::Credit {
            set: printing.set.clone(),
            set_name: printing.set_name.clone(),
            rarity: printing.rarity.clone(),
            artist: printing.artist.clone(),
        })
    }

    /// The deck as a draft the builder keeps on the device while it is
    /// unsaved (§2.1: "the builder autosaves a local draft").
    #[must_use]
    pub fn draft(&self) -> Draft {
        Draft {
            editing: self.editing.clone(),
            name: self.name.text().to_string(),
            main: self.rows(Zone::Main),
            side: self.rows(Zone::Side),
            commanders: self.commander_names(),
        }
    }

    /// Takes a kept draft back into the builder: the rows it names, its
    /// name, its commanders — and the deck is unsaved, since nothing of it
    /// reached the gateway. The deck it was a draft of stays the deck being
    /// edited.
    pub fn restore(&mut self, draft: &Draft) {
        let editing = draft.editing.clone();
        self.load(
            editing.as_deref().unwrap_or_default(),
            &draft.name,
            &draft.main,
            &draft.side,
            &draft.commanders,
        );
        self.editing = editing;
        self.touch();
    }

    /// Whether a kept draft says something this deck does not.
    #[must_use]
    pub fn differs_from(&self, draft: &Draft) -> bool {
        self.draft() != *draft
    }
}

/// The deck as the builder keeps it on the device until it is saved: the
/// deck being edited (none for a new one), its name, its rows in the stored
/// form (`docs/deck-format.md`) and its commanders by name.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Draft {
    /// The deck this is a draft of; `None` for a deck never saved.
    #[serde(default)]
    pub editing: Option<String>,
    /// Its name as typed.
    #[serde(default)]
    pub name: String,
    /// The main deck's rows.
    #[serde(default)]
    pub main: Vec<String>,
    /// The sideboard's rows.
    #[serde(default)]
    pub side: Vec<String>,
    /// Its commanders, by English name.
    #[serde(default)]
    pub commanders: Vec<String>,
}

impl Draft {
    /// Whether it holds anything worth restoring.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.name.trim().is_empty()
            && self.main.is_empty()
            && self.side.is_empty()
            && self.commanders.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(index: u32, name: &str, kinds: &[&str], cmc: u32, colors: &str) -> PoolCard {
        PoolCard {
            index,
            name: name.into(),
            english_name: name.into(),
            kinds: kinds.iter().map(|k| (*k).to_string()).collect(),
            cmc,
            colors: colors.into(),
            coverage: Coverage::Implemented,
            basic_land: kinds == ["Land"],
            ..PoolCard::default()
        }
    }

    fn deck() -> DeckBuilder {
        let mut deck = DeckBuilder::new();
        deck.set_pool(
            vec![
                card(1, "Forest", &["Land"], 0, ""),
                card(2, "Elf", &["Creature"], 1, "G"),
                card(3, "Bolt", &["Instant"], 1, "R"),
                card(4, "Charm", &["Instant"], 3, "RG"),
                card(5, "Golem", &["Artifact", "Creature"], 7, ""),
            ],
            true,
        );
        deck.load(
            "d",
            "Test",
            &[
                "10 Forest".into(),
                "4 Elf".into(),
                "2 Bolt".into(),
                "1 Charm".into(),
                "1 Golem".into(),
            ],
            &["3 Bolt".into()],
            &[],
        );
        deck
    }

    #[test]
    fn sections_follow_the_grouping_and_keep_row_addresses() {
        let deck = deck();
        let by_type = deck.sections(Zone::Main, Grouping::Type);
        let keys: Vec<SectionKey> = by_type.iter().map(|s| s.key).collect();
        assert_eq!(
            keys,
            vec![
                SectionKey::Type(Group::Creature),
                SectionKey::Type(Group::Instant),
                SectionKey::Type(Group::Land)
            ]
        );
        assert_eq!(by_type[0].cards, 5, "Elf and the artifact creature");
        // Every row is in exactly one section, by its address.
        let mut all: Vec<usize> = by_type.iter().flat_map(|s| s.rows.clone()).collect();
        all.sort_unstable();
        assert_eq!(all, (0..deck.entries(Zone::Main).len()).collect::<Vec<_>>());

        let by_mv = deck.sections(Zone::Main, Grouping::ManaValue);
        let keys: Vec<SectionKey> = by_mv.iter().map(|s| s.key).collect();
        assert_eq!(
            keys,
            vec![
                SectionKey::Mana(1),
                SectionKey::Mana(3),
                SectionKey::Mana(7),
                SectionKey::Lands
            ]
        );
        assert_eq!(by_mv[0].cards, 6);

        let by_colour = deck.sections(Zone::Main, Grouping::Colour);
        let keys: Vec<SectionKey> = by_colour.iter().map(|s| s.key).collect();
        assert_eq!(
            keys,
            vec![
                SectionKey::Colour(3),
                SectionKey::Colour(4),
                SectionKey::Multicolour,
                SectionKey::Colourless,
                SectionKey::Lands
            ]
        );
        assert_eq!(
            SectionKey::Mana(7).heading(Lang::En),
            "Mana value 7+",
            "the last bucket says it is that or more"
        );
    }

    #[test]
    fn the_curve_lights_one_type_and_counts_only_spells() {
        let deck = deck();
        let curve = deck.curve_lit(Some(Group::Creature));
        assert_eq!(curve[1], (6, 4), "four elves of six one-drops");
        assert_eq!(curve[3], (1, 0));
        assert_eq!(curve[7], (1, 1), "the golem is a creature at seven");
        assert_eq!(curve[0], (0, 0), "lands are not on the curve");
        assert_eq!(deck.curve_lit(None)[1], (6, 0));
    }

    #[test]
    fn the_type_counts_name_every_stat_group() {
        let deck = deck();
        let counts = deck.group_counts();
        let find = |g| counts.iter().find(|(x, _)| *x == g).map(|(_, n)| *n);
        assert_eq!(find(Group::Creature), Some(5));
        assert_eq!(find(Group::Instant), Some(3));
        assert_eq!(find(Group::Land), Some(10));
        assert_eq!(
            find(Group::Sorcery),
            Some(0),
            "an empty group is still counted"
        );
    }

    #[test]
    fn a_sample_hand_is_seven_of_the_main_deck_and_takes_nothing() {
        let deck = deck();
        let before = deck.entries(Zone::Main).to_vec();
        let hand = deck.sample_hand(42);
        assert_eq!(hand.len(), 7);
        for slot in &hand {
            let held: u16 = deck
                .entries(Zone::Main)
                .iter()
                .filter(|e| e.slot == *slot)
                .map(|e| e.count)
                .sum();
            let drawn = hand.iter().filter(|s| *s == slot).count();
            assert!(drawn <= usize::from(held), "more copies drawn than held");
        }
        assert_eq!(deck.entries(Zone::Main), before.as_slice());
        assert_eq!(hand, deck.sample_hand(42), "one seed, one shuffle");
        let hands: Vec<Vec<usize>> = (0..8).map(|s| deck.sample_hand(s)).collect();
        assert!(
            hands.iter().any(|h| *h != hands[0]),
            "different seeds shuffle differently"
        );
        let mut small = DeckBuilder::new();
        small.set_pool(vec![card(1, "Forest", &["Land"], 0, "")], true);
        small.load("d", "x", &["3 Forest".into()], &[], &[]);
        assert_eq!(small.sample_hand(1).len(), 3);
    }

    #[test]
    fn changes_count_edits_and_a_save_clears_them() {
        let mut deck = deck();
        assert_eq!(deck.changes(), 0, "a loaded deck has none");
        assert!(deck.add(2, Zone::Main));
        assert!(deck.add(3, Zone::Side));
        assert_eq!(deck.changes(), 2);
        assert_eq!(deck.last_added(), &[3, 2]);
        for ch in "abc".chars() {
            deck.type_name(ch);
        }
        assert_eq!(deck.changes(), 3, "three letters of a name are one change");
        assert!(deck.remove(2, Zone::Main));
        assert_eq!(deck.changes(), 4);
        deck.saved(None);
        assert_eq!(deck.changes(), 0);
        assert!(!deck.dirty());
    }

    #[test]
    fn a_draft_round_trips_and_restores_as_unsaved() {
        let mut deck = deck();
        deck.add(2, Zone::Main);
        let draft = deck.draft();
        let text = serde_json::to_string(&draft).expect("a draft encodes");
        let back: Draft = serde_json::from_str(&text).expect("and decodes");
        assert_eq!(back, draft);

        let mut other = DeckBuilder::new();
        other.set_pool(deck.pool().to_vec(), true);
        other.restore(&back);
        assert_eq!(other.entries(Zone::Main), deck.entries(Zone::Main));
        assert_eq!(other.entries(Zone::Side), deck.entries(Zone::Side));
        assert_eq!(other.editing(), Some("d"));
        assert!(other.dirty(), "a restored draft is unsaved");
        assert!(!other.differs_from(&back));
        assert!(Draft::default().is_empty());

        let mut fresh = DeckBuilder::new();
        fresh.set_pool(deck.pool().to_vec(), true);
        fresh.restore(&Draft {
            editing: None,
            name: "New".into(),
            main: vec!["2 Elf".into()],
            ..Draft::default()
        });
        assert_eq!(fresh.editing(), None, "a new deck's draft stays a new deck");
        assert_eq!(fresh.counts().main, 2);
    }

    #[test]
    fn a_printing_the_builder_was_told_about_credits_its_face() {
        let mut deck = deck();
        let printing = Printing {
            scryfall_id: "abc".into(),
            set: "zen".into(),
            set_name: "Zendikar".into(),
            artist: "Ryan Pancoast".into(),
            rarity: "common".into(),
            ..Printing::default()
        };
        assert_eq!(deck.printing_credit(2, "abc"), None, "nothing told yet");
        let slot = deck
            .pool()
            .iter()
            .position(|c| c.index == 2)
            .expect("the elf");
        let _ = deck.open_picker(slot, Zone::Main);
        deck.set_printings(2, vec![printing], true);
        assert!(
            deck.printing_credit(2, "abc").is_some(),
            "the open picker's"
        );
        deck.close_picker();
        let credit = deck.printing_credit(2, "abc").expect("the cached printing");
        assert_eq!(credit.foot().as_deref(), Some("Zendikar · Ryan Pancoast"));
        assert_eq!(deck.printing_credit(2, "other"), None);
    }
}
