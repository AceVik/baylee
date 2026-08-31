//! baylee-cards — the compiled card registry.
//!
//! One file per card in [`cards`], dense lookup tables in [`generated`]
//! (both produced by `cargo xtask codegen`).

#![warn(missing_docs)]

use baylee_cards_dsl::CardDef;
use baylee_core::ids::CardIndex;

/// Generated: one module per card.
pub mod cards;
/// Deck parsing and name resolution against the registry (acceptance
/// deck format, `"N Card Name"` lines, preset assembly).
pub mod decks;
/// Generated: registry tables.
pub mod generated;
/// Central named token definitions (referenced by card files).
pub mod tokens;

pub use baylee_cards_dsl as dsl;

/// Looks up a card definition by Scryfall oracle id.
#[must_use]
pub fn by_oracle_id(oracle_id: &str) -> Option<&'static CardDef> {
    generated::by_oracle_id(oracle_id)
}

/// Looks up a card definition by its dense runtime index.
#[must_use]
pub fn by_index(index: CardIndex) -> Option<&'static CardDef> {
    generated::by_index(index)
}

/// Number of registered cards.
#[must_use]
pub fn count() -> usize {
    generated::ALL.len()
}

/// Hash of the whole pool (client cache invalidation / gateway handshake).
#[must_use]
pub fn pool_hash() -> u64 {
    generated::POOL_HASH
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Card files inherit unstated fields from [`dsl::CardDef::DEFAULT`],
    /// whose `index` is 0. That is a deliberate collision: a file that
    /// forgets its index would otherwise shadow card 0 in `by_index` and
    /// hand the engine the wrong card at cast time. This test is what
    /// turns that collision into a build failure.
    #[test]
    fn every_card_sits_at_the_index_it_claims() {
        for (i, slot) in generated::BY_INDEX.iter().enumerate() {
            let Some(def) = slot else { continue };
            assert_eq!(
                def.index.get() as usize,
                i,
                "{} is registered at index {i} but claims {}",
                def.name(),
                def.index.get()
            );
        }
        let filled = generated::BY_INDEX.iter().flatten().count();
        assert_eq!(filled, generated::ALL.len());
    }

    /// An index is an identity, not a position: `DeckEntry` stores one, the
    /// gateway persists decks made of them, and a replay names them. They are
    /// handed out by `data/card-index.tsv` (append-only) rather than by a
    /// card's place in the alphabetically sorted pool, which is what used to
    /// renumber every card after any newly added one — silently pointing
    /// every saved deck at a different card.
    ///
    /// The ledger's own rules are tested in `baylee-cards-codegen`; what is
    /// checked here is the half that reaches the engine: the table is indexed
    /// by that number, tolerates a retired slot, and answers `None` for one.
    #[test]
    fn the_index_table_is_addressed_by_index_and_tolerates_a_retired_slot() {
        for (i, slot) in generated::BY_INDEX.iter().enumerate() {
            let index = CardIndex::new(u32::try_from(i).expect("pool fits in u32"));
            match slot {
                Some(def) => assert!(std::ptr::eq(by_index(index).expect("filled slot"), *def)),
                None => assert!(
                    by_index(index).is_none(),
                    "index {i} is retired and must answer to nothing"
                ),
            }
        }
        assert!(
            by_index(CardIndex::new(u32::MAX)).is_none(),
            "an index past the end is not a card"
        );
    }

    /// Every card in the pool is reachable through the index it claims — the
    /// other direction of the same table, and the one a deck list travels.
    #[test]
    fn every_card_answers_to_its_own_index() {
        for (_, def) in generated::ALL {
            let found = by_index(def.index).unwrap_or_else(|| {
                panic!(
                    "{} claims index {} and nothing is there",
                    def.name(),
                    def.index.get()
                )
            });
            assert_eq!(found.oracle_id, def.oracle_id);
        }
    }

    /// The same for the oracle-id table the gateway resolves deck lists
    /// against: an empty or copied `oracle_id` would silently resolve one
    /// card's name to another card's rules.
    #[test]
    fn every_card_answers_to_the_oracle_id_it_is_filed_under() {
        for (oracle, def) in generated::ALL {
            assert_eq!(def.oracle_id, *oracle, "{} is misfiled", def.name());
            assert!(!def.oracle_id.is_empty());
            assert!(!def.scryfall_id.is_empty());
            assert!(!def.faces.is_empty(), "{} has no faces", def.name());
            assert!(!def.name().is_empty());
        }
    }

    /// `coverage` defaults to `Unimplemented`, so a card that reaches the
    /// acceptance pool without the line is reported as a stub rather than
    /// quietly offered to deckbuilders.
    #[test]
    fn coverage_is_claimed_explicitly_or_not_at_all() {
        let unimplemented: Vec<&str> = generated::ALL
            .iter()
            .filter(|(_, d)| !d.is_implemented())
            .map(|(_, d)| d.name())
            .collect();
        assert!(
            unimplemented.is_empty(),
            "acceptance pool contains stubs: {unimplemented:?}"
        );
    }

    /// CR 305.6 gives a land one mana ability per basic land type, so a dual
    /// has two and its controller picks. The engine's intrinsic shortcut
    /// (`casting::intrinsic_mana`) can only return one colour and has no way
    /// to ask, so it deliberately declines any land with more than one basic
    /// type — such a land is playable only through the `AddManaChoice`
    /// ability printed on its card.
    ///
    /// A file that forgets it produces a land that taps for nothing at all,
    /// which is quiet in a way a rules bug should never be: the four
    /// shocklands spent their whole life tapping for exactly one of their two
    /// colours because the shortcut answered for them.
    #[test]
    fn a_land_with_two_basic_types_prints_its_own_mana_ability() {
        use baylee_cards_dsl::{AbilityDef, Effect};
        use baylee_core::generated::subtypes::land;
        use baylee_core::types::TypeSet;

        const BASICS: [baylee_core::ids::SubtypeId; 5] = [
            land::PLAINS,
            land::ISLAND,
            land::SWAMP,
            land::MOUNTAIN,
            land::FOREST,
        ];
        let mut offenders = Vec::new();
        for (_, def) in generated::ALL {
            for (i, face) in def.faces.iter().enumerate() {
                if !face.types.contains(TypeSet::LAND) {
                    continue;
                }
                let basics = BASICS.iter().filter(|b| face.subtypes.contains(b)).count();
                if basics < 2 {
                    continue;
                }
                let makes_mana = def.abilities_for_face(i).iter().any(|a| {
                    let AbilityDef::Activated { effects, .. } = a else {
                        return false;
                    };
                    effects
                        .iter()
                        .any(|e| matches!(e, Effect::AddMana { .. } | Effect::AddManaChoice { .. }))
                });
                if !makes_mana {
                    offenders.push(format!("{} (face {i})", def.name()));
                }
            }
        }
        offenders.sort();
        assert!(
            offenders.is_empty(),
            "these lands have two basic types and no printed mana ability, so \
             they tap for nothing: {offenders:?}"
        );
    }

    /// CR 903.4: a card's color identity covers the colored mana symbols in
    /// its cost *and* in its rules text, on every face. `color_identity` is
    /// hand-written in each file while the costs are read by the engine, so
    /// the two can disagree — and nothing else would notice, because the
    /// engine never reads `color_identity` at all. The gateway does: it is
    /// what makes a commander deck legal or illegal.
    ///
    /// Checked as a lower bound, which is the half that can be decided from
    /// the `CardDef` alone. Mana symbols in reminder or rules text (a dual
    /// land's "{T}: Add {B} or {R}") legitimately push the identity wider,
    /// so a superset is fine; a card whose own cost names a colour it does
    /// not claim is not.
    #[test]
    fn no_card_costs_a_colour_its_identity_leaves_out() {
        use baylee_cards_dsl::AbilityDef;
        use baylee_core::color::ColorSet;

        let mut offenders = Vec::new();
        for (_, def) in generated::ALL {
            let mut used = ColorSet::EMPTY;
            let mut add = |cost: &baylee_core::mana::ManaCost| used = used.union(cost.colors());
            for face in def.faces {
                add(&face.mana_cost);
                for alt in face.alternative_costs {
                    add(&alt.cost.mana);
                }
                for extra in face.additional_costs {
                    add(&extra.mana);
                }
                if let Some(miracle) = face.miracle {
                    add(&miracle);
                }
            }
            for ability in def
                .faces
                .iter()
                .enumerate()
                .flat_map(|(i, _)| def.abilities_for_face(i))
            {
                match ability {
                    AbilityDef::Activated { cost, .. }
                    | AbilityDef::ActivatedConditional { cost, .. } => add(&cost.mana),
                    AbilityDef::Echo { cost } => add(cost),
                    AbilityDef::ModalSpell { modes } => {
                        for mode in *modes {
                            if let Some(cost) = mode.cost_override {
                                add(&cost);
                            }
                        }
                    }
                    _ => {}
                }
            }
            let missing = used.difference(def.color_identity);
            if !missing.is_empty() {
                offenders.push(format!("{} is missing {missing:?}", def.name()));
            }
        }
        offenders.sort();
        assert!(
            offenders.is_empty(),
            "color identity narrower than the card's own costs: {offenders:?}"
        );
    }

    /// A card file's `// NOT SUPPORTED:` note and its `coverage` line are two
    /// statements about the same thing, and only the second one is checked by
    /// anything. Five cards drifted apart that way: the mechanic landed, the
    /// note stayed, and the file went on advertising a gap that had been
    /// closed — which is worse than no note, because it tells the next reader
    /// not to bother with the card.
    ///
    /// So the two have to agree: a file that still names a missing mechanic
    /// must say `Coverage::Partial`, and a file that claims full coverage
    /// must not carry the note.
    #[test]
    fn a_card_that_names_a_missing_mechanic_does_not_also_claim_full_coverage() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/cards");
        let mut offenders = Vec::new();
        for entry in std::fs::read_dir(dir).expect("cards dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read card");
            if text.contains("NOT SUPPORTED") && text.contains("coverage: Coverage::Implemented") {
                offenders.push(
                    path.file_name()
                        .expect("file name")
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        offenders.sort();
        assert!(
            offenders.is_empty(),
            "these files name a missing mechanic but claim `Coverage::Implemented` \
             — fix the card or downgrade it to `Coverage::Partial`: {offenders:?}"
        );
    }
}
