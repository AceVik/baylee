//! Exact, unordered knowledge of the viewing seat's submitted cards.
//!
//! Input is one seat's submission, never a game state or opponents' decks.
//! Section membership survives: outside-game cards are not library cards.
//! This is a registration inventory, not a claim about the current library.
//! Copies, face-down objects and hidden transfers require a separate,
//! uncertainty-aware movement ledger before deriving remaining-card counts.
//! This multiset also does not encode setup procedure: no fixed hand and an
//! explicitly empty fixed hand both register zero additional hand cards, but
//! only the former requests the ordinary opening draw. The first observation
//! and any future setup encoder must retain that distinction.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context as _, ensure};
use baylee_core::card_metadata::{DeckChoices, RulesMetadata};
use baylee_core::ids::{CardIndex, ObjectId, PlayerId};
use baylee_core::preset::{DeckEntry, GamePreset, SeatSpec};
use baylee_view::{ObjectStatus, PlayerView};
use serde::{Deserialize, Serialize};

/// Distinct sources of a seat's registered cards, never combined implicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeckSection7 {
    /// Cards submitted to be shuffled into the library.
    Library,
    /// Outside-game cards, including wish targets.
    Sideboard,
    /// Designated commanders, initially outside the library.
    Commander,
    /// Explicit additional starting hand in configured games/puzzles.
    StartingHand,
    /// Explicit starting permanents in configured games/puzzles.
    StartingBattlefield,
}

/// Resolved rules facts, without preset-local printing/watermark indices.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrintingFacts7 {
    /// Exact watermark tags, one optional value per printed face.
    pub watermarks: Vec<Option<String>>,
    /// Silver-bordered or acorn physical printing.
    pub is_un: bool,
}

/// One semantically distinct kind of physical card in the submission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredCard7 {
    /// Stable catalog identity, including valid card index zero.
    pub card_id: u32,
    /// Library, sideboard, commander or explicit starting position.
    pub section: DeckSection7,
    /// Per-copy pregame choices; differently circled Spires stay separate.
    pub choices: DeckChoices,
    /// Independently submitted physical rules facts. `None` means absent
    /// metadata (catalog fallback), not a verified absence of watermarks.
    pub printing: Option<PrintingFacts7>,
    /// Exact multiplicity, without the encoder-6 saturation at 255.
    pub copies: u32,
}

/// A canonical registration multiset, bound to the seat it belongs to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredDeck7 {
    /// Whose private submission this is; callers must match their actor seat.
    pub viewer: PlayerId,
    /// Canonical unordered rows; no submitted or shuffled library order.
    pub cards: Vec<RegisteredCard7>,
}

impl RegisteredDeck7 {
    /// Builds only the supplied seat's multiset. The shared metadata table is
    /// dereferenced only by that seat's entries; unrelated rows are not read
    /// or exported. Artwork references have no rules meaning and are omitted.
    ///
    /// # Errors
    /// Rejects invalid identities, choices, referenced printing facts and
    /// counts too large to represent, instead of silently merging or clipping.
    pub fn from_submission(
        viewer: PlayerId,
        own: &SeatSpec,
        rules: &RulesMetadata,
    ) -> anyhow::Result<Self> {
        let sections = [
            (DeckSection7::Library, own.deck.as_slice()),
            (DeckSection7::Sideboard, own.sideboard.as_slice()),
            (DeckSection7::Commander, own.commanders.as_slice()),
            (
                DeckSection7::StartingHand,
                own.starting_hand.as_deref().unwrap_or_default(),
            ),
            (
                DeckSection7::StartingBattlefield,
                own.starting_battlefield.as_slice(),
            ),
        ];
        let mut rows = BTreeMap::<Vec<u8>, RegisteredCard7>::new();
        for (section, entries) in sections {
            for entry in entries {
                ensure!(
                    baylee_cards::by_index(entry.card).is_some(),
                    "registered card has no catalog identity"
                );
                ensure!(entry.choices.is_valid(), "invalid registered card choices");
                let card = RegisteredCard7 {
                    card_id: entry.card.get(),
                    section,
                    choices: entry.choices,
                    printing: printing_facts(entry, rules)?,
                    copies: 1,
                };
                // A typed row serialized with a fixed count is a complete,
                // deterministic key. New semantic fields cannot accidentally
                // be omitted from a hand-maintained equivalence comparator.
                let key = serde_json::to_vec(&card)?;
                match rows.entry(key) {
                    std::collections::btree_map::Entry::Vacant(slot) => {
                        slot.insert(card);
                    }
                    std::collections::btree_map::Entry::Occupied(mut slot) => {
                        let count = &mut slot.get_mut().copies;
                        *count = count.checked_add(1).context("registered count overflow")?;
                    }
                }
            }
        }
        Ok(Self {
            viewer,
            cards: rows.into_values().collect(),
        })
    }
}

fn printing_facts(
    entry: &DeckEntry,
    rules: &RulesMetadata,
) -> anyhow::Result<Option<PrintingFacts7>> {
    let Some(reference) = entry.rule_print else {
        return Ok(None);
    };
    let print = rules
        .prints
        .get(usize::from(reference.get()))
        .context("missing own printing metadata")?;
    ensure!(
        print.card == entry.card,
        "own printing names a different card"
    );
    ensure!(
        !print.watermarks.is_empty() && u8::try_from(print.watermarks.len()).is_ok(),
        "invalid own printing face count"
    );
    let watermarks = print
        .watermarks
        .iter()
        .map(|id| {
            id.map(|id| {
                let tag = rules
                    .watermarks
                    .get(usize::from(id.get()))
                    .context("missing own watermark metadata")?;
                ensure!(
                    !tag.is_empty()
                        && tag.len() <= 80
                        && tag
                            .bytes()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-'),
                    "invalid own watermark tag"
                );
                Ok(tag.clone())
            })
            .transpose()
        })
        .collect::<anyhow::Result<_>>()?;
    Ok(Some(PrintingFacts7 {
        watermarks,
        is_un: print.is_un,
    }))
}

/// Remaining copies of one distinct card group in a seat's library or sideboard.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemainingCardGroup7 {
    /// Stable catalog identity.
    pub card_id: u32,
    /// Originating registration section (`Library` for in-library groups, `Sideboard` for outside-game wish pool).
    pub section: DeckSection7,
    /// Per-copy pregame choices.
    pub choices: DeckChoices,
    /// Resolved physical printing rules facts.
    pub printing: Option<PrintingFacts7>,
    /// Total copies registered in this group at deck submission.
    pub registered_copies: u32,
    /// Exact copies currently remaining in this zone (unclipped).
    pub remaining_copies: u32,
}

/// Uncertainty-aware movement ledger deriving exact remaining library and
/// sideboard counts solely from a seat's own registration and `PlayerView`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryLedger7 {
    viewer: Option<PlayerId>,
    registered: Option<RegisteredDeck7>,
    /// Persistent per-game map from an own physical object's arena handle to its
    /// printed catalog card ID, recorded whenever the object's identity is visible.
    known_object_cards: BTreeMap<ObjectId, u32>,
    /// Current count of own physical cards outside the library, keyed by `card_id`.
    outside_library_by_card: BTreeMap<u32, u32>,
    /// Count of own physical cards currently outside the library whose identity
    /// is hidden from the viewer (e.g., exiled face-down by an opponent).
    unidentified_outside_library: u32,
    /// Most recently observed `SeatView::library_count` for the viewing seat.
    observed_library_count: u32,
}

impl InventoryLedger7 {
    /// Creates an empty ledger with no registered deck.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates a ledger from an already-validated [`RegisteredDeck7`].
    #[must_use]
    pub fn from_registered(registered: RegisteredDeck7) -> Self {
        Self {
            viewer: Some(registered.viewer),
            registered: Some(registered),
            known_object_cards: BTreeMap::new(),
            outside_library_by_card: BTreeMap::new(),
            unidentified_outside_library: 0,
            observed_library_count: 0,
        }
    }

    /// Creates a ledger from the viewing seat's own [`SeatSpec`] submission.
    ///
    /// # Errors
    /// Returns an error if the submission references invalid card/printing metadata.
    pub fn from_submission(
        viewer: PlayerId,
        own: &SeatSpec,
        rules: &RulesMetadata,
    ) -> anyhow::Result<Self> {
        Ok(Self::from_registered(RegisteredDeck7::from_submission(
            viewer, own, rules,
        )?))
    }

    /// Creates a ledger from `preset` for `viewer`, reading only `viewer`'s submission.
    ///
    /// # Errors
    /// Returns an error if `viewer` is out of range or metadata is invalid.
    pub fn from_preset(preset: &GamePreset, viewer: PlayerId) -> anyhow::Result<Self> {
        let own = preset
            .seats
            .get(usize::from(viewer.get()))
            .context("viewer seat out of range in preset")?;
        Self::from_submission(viewer, own, &preset.rules)
    }

    /// Creates a library-only ledger from `(card, count)` pairs.
    #[must_use]
    pub fn from_own_cards<I>(viewer: PlayerId, cards: I) -> Self
    where
        I: IntoIterator<Item = (CardIndex, u32)>,
    {
        let mut counts = BTreeMap::<u32, u32>::new();
        for (card, count) in cards {
            if count == 0 {
                continue;
            }
            let entry = counts.entry(card.get()).or_insert(0);
            *entry = entry.saturating_add(count);
        }
        let rows = counts
            .into_iter()
            .map(|(card_id, copies)| RegisteredCard7 {
                card_id,
                section: DeckSection7::Library,
                choices: DeckChoices::default(),
                printing: None,
                copies,
            })
            .collect();
        Self::from_registered(RegisteredDeck7 {
            viewer,
            cards: rows,
        })
    }

    /// Creates a ledger from explicit library, sideboard, and commander sections.
    #[must_use]
    pub fn from_sections<L, S, C>(viewer: PlayerId, library: L, sideboard: S, commanders: C) -> Self
    where
        L: IntoIterator<Item = (CardIndex, u32)>,
        S: IntoIterator<Item = (CardIndex, u32)>,
        C: IntoIterator<Item = (CardIndex, u32)>,
    {
        let mut rows = Vec::<RegisteredCard7>::new();
        for (section, iter) in [
            (
                DeckSection7::Library,
                library.into_iter().collect::<Vec<_>>(),
            ),
            (
                DeckSection7::Sideboard,
                sideboard.into_iter().collect::<Vec<_>>(),
            ),
            (
                DeckSection7::Commander,
                commanders.into_iter().collect::<Vec<_>>(),
            ),
        ] {
            let mut counts = BTreeMap::<u32, u32>::new();
            for (card, count) in iter {
                if count == 0 || card.get() == crate::carddsl::UNKNOWN_CARD {
                    continue;
                }
                let entry = counts.entry(card.get()).or_insert(0);
                *entry = entry.saturating_add(count);
            }
            for (card_id, copies) in counts {
                rows.push(RegisteredCard7 {
                    card_id,
                    section,
                    choices: DeckChoices::default(),
                    printing: None,
                    copies,
                });
            }
        }
        Self::from_registered(RegisteredDeck7 {
            viewer,
            cards: rows,
        })
    }

    /// Returns whether a non-empty deck registration is present.
    #[must_use]
    pub fn has_registration(&self) -> bool {
        self.registered
            .as_ref()
            .is_some_and(|reg| !reg.cards.is_empty())
    }

    /// Returns the registered deck multiset, if any.
    #[must_use]
    pub fn registered(&self) -> Option<&RegisteredDeck7> {
        self.registered.as_ref()
    }

    /// Returns the count of own cards currently outside the library with hidden identity.
    #[must_use]
    pub fn unidentified_outside_library(&self) -> u32 {
        self.unidentified_outside_library
    }

    /// Returns the most recently observed `library_count` for the viewing seat.
    #[must_use]
    pub fn observed_library_count(&self) -> u32 {
        self.observed_library_count
    }

    /// Resets per-game movement state while preserving the registered deck.
    pub fn reset_preserve_deck(&mut self) {
        self.known_object_cards.clear();
        self.outside_library_by_card.clear();
        self.unidentified_outside_library = 0;
        self.observed_library_count = 0;
    }

    /// Updates the movement ledger from the seat's current `PlayerView`.
    pub fn observe(&mut self, view: &PlayerView) {
        let you = view.seat;
        if self.viewer.is_none() {
            self.viewer = Some(you);
        }
        self.observed_library_count = view
            .seats
            .iter()
            .find(|s| s.player == you)
            .map_or(0, |s| s.library_count);

        // 1. Record printed/known card identities for own physical objects across all visible zones.
        for h in &view.hand {
            self.known_object_cards.insert(h.id, h.card.index.get());
        }
        for ps in view.shared_hands.iter().chain(view.controlled_hands.iter()) {
            if ps.player == you {
                for h in &ps.cards {
                    self.known_object_cards.insert(h.id, h.card.index.get());
                }
            }
        }
        for seat in &view.seats {
            if seat.player == you {
                for cmd in &seat.commanders {
                    if let Some(card) = cmd.card {
                        self.known_object_cards.insert(cmd.object, card.index.get());
                    }
                }
            }
        }
        let all_public_objects = view
            .battlefield
            .iter()
            .chain(view.stack.iter())
            .chain(view.exile.iter().flatten())
            .chain(view.command.iter().flatten())
            .chain(view.looking_at.iter())
            .chain(view.graveyards.iter().flatten())
            .chain(view.library_tops.iter());

        for obj in all_public_objects {
            if obj.owner == you
                && is_physical_public_object(obj)
                && let Some(card_id) = physical_object_card_id(obj)
            {
                self.known_object_cards.entry(obj.id).or_insert(card_id);
            }
        }

        // 2. Collect deduplicated own physical objects currently outside the library.
        let mut outside_ids = BTreeSet::<ObjectId>::new();
        let mut outside_by_card = BTreeMap::<u32, u32>::new();
        let mut unidentified = 0u32;

        let mut record_outside = |id: ObjectId, direct_card: Option<u32>| {
            if !outside_ids.insert(id) {
                return;
            }
            let resolved = direct_card.or_else(|| self.known_object_cards.get(&id).copied());
            if let Some(card_id) = resolved {
                let entry = outside_by_card.entry(card_id).or_insert(0);
                *entry = entry.saturating_add(1);
            } else {
                unidentified = unidentified.saturating_add(1);
            }
        };

        for h in &view.hand {
            record_outside(h.id, Some(h.card.index.get()));
        }
        for ps in view.shared_hands.iter().chain(view.controlled_hands.iter()) {
            if ps.player == you {
                for h in &ps.cards {
                    record_outside(h.id, Some(h.card.index.get()));
                }
            }
        }
        let outside_public_objects = view
            .battlefield
            .iter()
            .chain(view.stack.iter())
            .chain(view.exile.iter().flatten())
            .chain(view.command.iter().flatten())
            .chain(view.graveyards.iter().flatten());

        for obj in outside_public_objects {
            if obj.owner == you && is_physical_public_object(obj) {
                let direct = physical_object_card_id(obj);
                // Ignore bare emblems in the command zone (which have no card identity).
                if obj.types.is_empty()
                    && direct.is_none()
                    && !self.known_object_cards.contains_key(&obj.id)
                    && !obj.commander
                {
                    continue;
                }
                record_outside(obj.id, direct);
            }
        }

        self.outside_library_by_card = outside_by_card;
        self.unidentified_outside_library = unidentified;
    }

    /// Computes exact remaining card groups in the viewing seat's library and sideboard.
    #[must_use]
    pub fn remaining_groups(&self) -> (Vec<RemainingCardGroup7>, Vec<RemainingCardGroup7>) {
        let Some(reg) = &self.registered else {
            return (Vec::new(), Vec::new());
        };
        let mut outside_remaining = self.outside_library_by_card.clone();

        // First pass: non-library in-game sections (Commander, StartingHand, StartingBattlefield)
        // start outside the library, so they absorb observed outside-library copies first.
        // If any copy from those sections is NOT outside the library (e.g. a commander or
        // starting-hand card was shuffled into the library), it is now in the library.
        let mut extra_in_library = Vec::<RemainingCardGroup7>::new();
        for row in &reg.cards {
            if matches!(
                row.section,
                DeckSection7::Commander
                    | DeckSection7::StartingHand
                    | DeckSection7::StartingBattlefield
            ) {
                let used = outside_remaining.entry(row.card_id).or_insert(0);
                let absorbed = (*used).min(row.copies);
                *used -= absorbed;
                let in_library_now = row.copies - absorbed;
                if in_library_now > 0 {
                    extra_in_library.push(RemainingCardGroup7 {
                        card_id: row.card_id,
                        section: DeckSection7::Library,
                        choices: row.choices,
                        printing: row.printing.clone(),
                        registered_copies: row.copies,
                        remaining_copies: in_library_now,
                    });
                }
            }
        }

        // Second pass: subtract remaining outside-library copies from Library rows.
        let mut library_groups = Vec::<RemainingCardGroup7>::new();
        for row in &reg.cards {
            if row.section == DeckSection7::Library {
                let used = outside_remaining.entry(row.card_id).or_insert(0);
                let absorbed = (*used).min(row.copies);
                *used -= absorbed;
                let rem = row.copies - absorbed;
                if rem > 0 {
                    library_groups.push(RemainingCardGroup7 {
                        card_id: row.card_id,
                        section: DeckSection7::Library,
                        choices: row.choices,
                        printing: row.printing.clone(),
                        registered_copies: row.copies,
                        remaining_copies: rem,
                    });
                }
            }
        }
        library_groups.extend(extra_in_library);

        // Third pass: any excess outside-library copies beyond the in-game pool
        // must have entered from the Sideboard (via a wish effect).
        let mut sideboard_groups = Vec::<RemainingCardGroup7>::new();
        for row in &reg.cards {
            if row.section == DeckSection7::Sideboard {
                let used = outside_remaining.entry(row.card_id).or_insert(0);
                let absorbed = (*used).min(row.copies);
                *used -= absorbed;
                let rem = row.copies - absorbed;
                if rem > 0 {
                    sideboard_groups.push(RemainingCardGroup7 {
                        card_id: row.card_id,
                        section: DeckSection7::Sideboard,
                        choices: row.choices,
                        printing: row.printing.clone(),
                        registered_copies: row.copies,
                        remaining_copies: rem,
                    });
                }
            }
        }

        (library_groups, sideboard_groups)
    }

    /// Returns remaining card groups in the viewing seat's library.
    #[must_use]
    pub fn remaining_library_groups(&self) -> Vec<RemainingCardGroup7> {
        self.remaining_groups().0
    }

    /// Returns remaining card groups in the viewing seat's sideboard.
    #[must_use]
    pub fn remaining_sideboard_groups(&self) -> Vec<RemainingCardGroup7> {
        self.remaining_groups().1
    }

    /// Returns remaining copies of `card_id` in the viewing seat's library.
    #[must_use]
    pub fn remaining_copies_in_library(&self, card_id: u32) -> u32 {
        let Some(reg) = &self.registered else {
            return 0;
        };
        let in_game_registered: u32 = reg
            .cards
            .iter()
            .filter(|r| r.card_id == card_id && r.section != DeckSection7::Sideboard)
            .map(|r| r.copies)
            .sum();
        let outside = self
            .outside_library_by_card
            .get(&card_id)
            .copied()
            .unwrap_or(0);
        in_game_registered.saturating_sub(outside)
    }

    /// Returns the total remaining library cards accounted for by the ledger.
    #[must_use]
    pub fn total_remaining_library_cards(&self) -> u32 {
        self.remaining_library_groups()
            .iter()
            .map(|g| g.remaining_copies)
            .sum::<u32>()
            .saturating_sub(self.unidentified_outside_library)
    }

    /// Returns the hypergeometric single-draw probability for `card_id`.
    #[must_use]
    pub fn draw_probability(&self, card_id: u32) -> f32 {
        let rem = self.remaining_copies_in_library(card_id);
        if rem == 0 {
            return 0.0;
        }
        let denom = self
            .observed_library_count
            .max(self.total_remaining_library_cards())
            .max(1);
        (rem as f32) / (denom as f32)
    }
}

fn is_physical_public_object(obj: &baylee_view::PublicObject) -> bool {
    if obj.status.contains(ObjectStatus::TOKEN) || obj.token.is_some() {
        return false;
    }
    if matches!(obj.stack_item, Some(baylee_view::StackItem::Ability { .. })) {
        return false;
    }
    true
}

fn physical_object_card_id(obj: &baylee_view::PublicObject) -> Option<u32> {
    obj.card
        .map(|c| c.index.get())
        .or_else(|| obj.rules.map(|r| r.card.get()))
}

#[cfg(test)]
mod tests;
