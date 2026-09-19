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
/// Filters shared by more than one card file.
pub mod filters;
/// Generated: registry tables.
pub mod generated;
/// Generated: which printed sentence each ability came from.
pub mod generated_lines;
/// Generated: the name table — which card a printed English name is.
pub mod generated_names;
/// Generated: the token ledger — which id every token there is was assigned.
pub mod generated_tokens;
/// Which printed sentence an ability came from (the reader of
/// [`generated_lines`]).
pub mod lines;
/// Pool-wide lints over the card data (tests only).
#[cfg(test)]
mod lints;
/// The registry as deck-builder rows — what a deck may be built from.
pub mod pool;
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

/// Every card in the registry, in index order.
///
/// The pool is what a deck builder can offer, so something has to be able to
/// walk it. Index order rather than table order: the order is what a player
/// sees when nothing else sorts the list, and a `HashMap`'s order is not an
/// order.
pub fn all() -> impl Iterator<Item = &'static CardDef> {
    generated::BY_INDEX.iter().filter_map(|slot| *slot)
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

    /// Daybound sits on a front face and nightbound on a back one, and
    /// neither is ever written at card level (CR 702.145a).
    ///
    /// `CardDef::keywords_for_face` falls back to the card-level set for
    /// face 0 and never for a back face, which is what lets every ordinary
    /// card go on stating its keywords once. These two cannot use it. A
    /// card claiming daybound for the whole card would claim it for the
    /// night side as well, and then CR 702.145g's "no permanents with
    /// daybound on the battlefield" could never be true while a werewolf
    /// was in play — the game would never become night on its own. The
    /// mirror image is a nightbound front face, which CR 702.145f would
    /// turn over the instant it was day, in a loop it also could not leave.
    #[test]
    fn daybound_and_nightbound_are_printed_on_the_face_that_has_them() {
        use dsl::KeywordSet as K;
        let both = K::DAYBOUND.union(K::NIGHTBOUND);
        for (oracle_id, def) in generated::ALL {
            assert!(
                !def.keywords.contains(both),
                "{} ({oracle_id}) states daybound or nightbound at card level; \
                 they belong on a face (CR 702.145a)",
                def.name(),
            );
            for (i, face) in def.faces.iter().enumerate() {
                assert!(
                    !(i > 0 && face.keywords.contains(K::DAYBOUND)),
                    "{} ({oracle_id}) prints daybound on face {i}; it is a front-face keyword",
                    def.name(),
                );
                assert!(
                    !(i == 0 && face.keywords.contains(K::NIGHTBOUND)),
                    "{} ({oracle_id}) prints nightbound on its front face; it is a back-face \
                     keyword",
                    def.name(),
                );
            }
            assert!(
                !(def.all_keywords().contains(both) && def.faces.len() < 2),
                "{} ({oracle_id}) is daybound or nightbound with one face; \
                 both keywords need a card to turn over (CR 701.27c)",
                def.name(),
            );
        }
    }

    /// The back face of a transforming double-faced card is never cast (CR
    /// 712.2) — it is only ever reached by turning the card over.
    ///
    /// Nothing in a `CardDef` says which layout a card was printed in, so
    /// `castable_from_hand` is what carries the difference between a modal
    /// back a player may cast and a transformed back they may not. Get it
    /// wrong and the cast wizard offers the back face as a *mode*, at the
    /// mana cost that face prints — which for a transformed back is nothing
    /// at all. Tavern Smasher was on offer for {0} until this test existed,
    /// and so were Ormendahl, Creeping Inn and an airborne school.
    ///
    /// The marker is the **printed cost**, not nightbound: every back a
    /// player may cast prints one — an MDFC's (CR 712.3), a disturb back's,
    /// an adventure's — and a transformed back prints none. Reading
    /// nightbound instead would have guarded the five werewolves and let the
    /// next Delver of Secrets through, which is what happened: three cards in
    /// the pool were already free spells when it was written that way.
    ///
    /// What this cannot see is a cost that was **invented**. It reads the
    /// compiled `FaceDef`, so a transformed back written with a cost no
    /// printing has looks exactly like an MDFC's back to it and is waved
    /// through — The True Scriptures carried a `{2}{B}{B}` and was on offer
    /// out of hand for five mana. The half that answers it is in `xtask`:
    /// `check_code_matches_the_printing` compares each face's cost against
    /// Scryfall's, and its one tolerance for a costless back is now narrowed
    /// to a face the code calls `disturb`. The two are a pair — this test
    /// says what a cost *means*, that check says the cost is the card's.
    #[test]
    fn a_back_face_with_no_printed_cost_is_never_castable_from_the_hand() {
        use baylee_core::types::TypeSet;
        for (oracle_id, def) in generated::ALL {
            for face in def.faces.iter().skip(1) {
                assert!(
                    !(face.mana_cost == baylee_core::mana::ManaCost::ZERO
                        && !face.types.contains(TypeSet::LAND)
                        && face.castable_from_hand),
                    "{} ({oracle_id}): {} is a back face with no printed cost and may not be \
                     cast (CR 712.2) — set castable_from_hand: false",
                    def.name(),
                    face.name,
                );
            }
        }
    }

    /// An index is an identity, not a position: `DeckEntry` stores one, the
    /// gateway persists decks made of them, and a replay names them. They are
    /// handed out by the append-only `CardIndex` ledger rather than by a
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

    /// `coverage` defaults to `Unimplemented`, and a stub must be *empty*:
    /// abilities without the claim are abilities nothing will ever run,
    /// because the deckbuilder refuses to offer the card at all.
    ///
    /// This used to read "the pool contains no stubs", which held while the
    /// pool was 196 hand-finished cards and stopped being expressible the
    /// moment the pool became every land Scryfall prints. The rule that
    /// survives the growth is the one that was actually load-bearing: a card
    /// either claims what it can do or does nothing, never something in
    /// between. A generator that reads half a card and forgets to refuse it
    /// fails here.
    #[test]
    fn an_unimplemented_card_carries_no_abilities() {
        use baylee_cards_dsl::{Coverage, KeywordSet};

        let mut offenders = Vec::new();
        for (_, def) in generated::ALL {
            match def.coverage {
                Coverage::Implemented => {}
                Coverage::Partial(reason) => assert!(
                    !reason.is_empty(),
                    "{} is Partial without saying what is missing",
                    def.name()
                ),
                Coverage::Unimplemented => {
                    let has_rules = !def.abilities.is_empty()
                        || def
                            .faces
                            .iter()
                            .any(|f| !f.abilities.is_empty() || !f.enter_modifiers.is_empty())
                        || def.keywords != KeywordSet::EMPTY;
                    if has_rules {
                        offenders.push(def.name());
                    }
                }
            }
        }
        offenders.sort_unstable();
        assert!(
            offenders.is_empty(),
            "these cards carry rules but do not claim any coverage, so the \
             deckbuilder hides rules that would otherwise run: {offenders:?}"
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
            // A stub taps for nothing on purpose and is never offered as
            // playable; the bug this guards against is a *finished* land.
            if !def.is_implemented() {
                continue;
            }
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
                    effects.iter().any(|e| matches!(e, Effect::AddMana { .. }))
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
                    // `ModalTriggered` carries `SpellMode`s too and is not
                    // read here, which is a statement rather than the
                    // forgotten twin it looks like: a triggered ability is
                    // never cast, so it has no printed cost for a mode to
                    // override, and `no_modal_trigger_overrides_a_cost`
                    // below is what holds that — and is what would fail
                    // first if a card ever put a cost there.
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

    /// A mode of a modal **trigger** never overrides a cost.
    ///
    /// `SpellMode` is one struct serving both modal variants, so a modal
    /// trigger's mode structurally carries a `cost_override` — and there
    /// is nothing for it to override: a triggered ability is put on the
    /// stack by the game and is never cast, so it has no printed cost
    /// (CR 603.3) and nobody pays for a mode of it. The field is overload's
    /// (`AbilityDef::ModalSpell`), and this is what says the six modal
    /// triggers in the pool leave it alone.
    ///
    /// It is here because the lint is cheaper than the arm it replaces.
    /// `no_card_costs_a_colour_its_identity_leaves_out` reads
    /// `ModalSpell`'s modes and not `ModalTriggered`'s, which in a repo
    /// that has just been through `ActivatedConditional` reads exactly
    /// like the same omission — so the reason it is *not* one is written
    /// as a test rather than as a claim, and the day a card disagrees this
    /// fails first and names it.
    #[test]
    fn no_modal_trigger_overrides_a_cost() {
        use baylee_cards_dsl::AbilityDef;

        let mut seen = 0usize;
        for (_, def) in generated::ALL {
            for face in 0..def.faces.len() {
                for ability in def.abilities_for_face(face) {
                    let AbilityDef::ModalTriggered { modes, .. } = ability else {
                        continue;
                    };
                    seen += 1;
                    for (at, mode) in modes.iter().enumerate() {
                        assert!(
                            mode.cost_override.is_none(),
                            "{} face {face} mode {at} is a trigger's and prices itself",
                            def.name()
                        );
                    }
                }
            }
        }
        assert!(
            seen >= 7,
            "only {seen} modal triggers in the pool, so this proves nothing; \
             six cards print seven of them"
        );
    }

    /// Every card file, wherever the taxonomy has filed it.
    ///
    /// `cards/` is a tree — `<type>/<subtype>/mv_<n>/<slug>.rs` — and the two
    /// lints below used to read it with a bare `read_dir`, which sees six
    /// folders and a `mod.rs`. Both had been passing on an empty worklist
    /// ever since the files moved, which is the failure mode `CLAUDE.md`
    /// names for anything that reads card files: a non-recursive read of a
    /// tree finds nothing and reports it as an answer.
    ///
    /// So this walks, and panics on an empty result rather than handing back
    /// a list a caller would read as "nothing is wrong".
    pub(crate) fn every_card_file() -> Vec<(String, String)> {
        let root = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src/cards"));
        let mut found = Vec::new();
        let mut stack = vec![root.clone()];
        while let Some(here) = stack.pop() {
            for entry in std::fs::read_dir(&here).expect("a cards directory") {
                let path = entry.expect("a directory entry").path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs")
                    && path.file_name().is_some_and(|n| n != "mod.rs")
                {
                    let name = path
                        .file_name()
                        .expect("a file name")
                        .to_string_lossy()
                        .into_owned();
                    found.push((name, std::fs::read_to_string(&path).expect("read a card")));
                }
            }
        }
        assert!(
            found.len() > 1000,
            "only {} card files were found under {} — the sweep is not reaching the pool",
            found.len(),
            root.display()
        );
        found.sort();
        found
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
        let mut offenders = Vec::new();
        for (name, text) in every_card_file() {
            if text.contains("NOT SUPPORTED") && text.contains("coverage = Coverage::Implemented") {
                offenders.push(name);
            }
        }
        assert!(
            offenders.is_empty(),
            "these files name a missing mechanic but claim `Coverage::Implemented` \
             — fix the card or downgrade it to `Coverage::Partial`: {offenders:?}"
        );
    }

    /// "Basic land card" is the Basic *supertype* (CR 205.4a), and spelling it
    /// as the five basic land subtypes is a rules error, not a style choice:
    /// Breeding Pool has the Forest subtype and is not a basic land, so a
    /// Rampant Growth written that way fetches shocklands.
    ///
    /// Naming a few of the subtypes is normal and correct — a fetchland asks
    /// for "a Plains or Island card", a checkland looks for one you control.
    /// Two generated cards were written the wrong way, past `clippy`, the card
    /// tests and a review pass, and nothing here noticed until they were read
    /// by hand.
    ///
    /// This said "naming all five at once never means anything but basic" and
    /// forbade the shape outright, which held until the pool reached the one
    /// card it is not true of. Spoils of Victory prints *"a Plains, Island,
    /// Swamp, Mountain, or Forest card"* — Wizards' older wording for a land
    /// with a basic land type, which a Breeding Pool satisfies and a
    /// `SupertypeSet::BASIC` filter would wrongly refuse. So the question is
    /// asked of the **printing** and not of the filter: five subtypes are
    /// right where the card names five types, and wrong where it says "basic
    /// land". The card's own `//! Oracle:` header is that printing, and
    /// `xtask validate` is what holds it to Scryfall.
    ///
    /// Measured over the reference corpus: 369 scripts search for
    /// `Land.Basic` and exactly **one** spells the five names — this card.
    #[test]
    fn no_card_spells_basic_land_as_the_five_basic_subtypes() {
        const SUBTYPES: [&str; 5] = ["PLAINS", "ISLAND", "SWAMP", "MOUNTAIN", "FOREST"];
        let mut offenders = Vec::new();
        for (name, text) in every_card_file() {
            let names_all_five = SUBTYPES
                .iter()
                .all(|s| text.contains(&format!("HasSubtype(land::{s})")))
                || SUBTYPES
                    .iter()
                    .all(|s| text.contains(&format!("HasSubtype(subtypes::land::{s})")));
            let printed: String = text
                .lines()
                .filter(|l| l.starts_with("//! Oracle:"))
                .collect::<Vec<_>>()
                .join(" ")
                .to_uppercase();
            let prints_all_five = SUBTYPES.iter().all(|s| printed.contains(s));
            if names_all_five && !prints_all_five {
                offenders.push(name);
            }
        }
        assert!(
            offenders.is_empty(),
            "these files spell \"basic land\" as five subtypes and print no \
             such list — use `Filter::HasSupertype(SupertypeSet::BASIC)`: \
             {offenders:?}"
        );
        // The rule is only a rule if the other half of it is reachable: the
        // card that legitimately writes the five has to exist, or this has
        // quietly become a test of nothing.
        let named: Vec<String> = every_card_file()
            .into_iter()
            .filter(|(_, text)| {
                SUBTYPES
                    .iter()
                    .all(|s| text.contains(&format!("HasSubtype(subtypes::land::{s})")))
            })
            .map(|(name, _)| name)
            .collect();
        assert!(
            !named.is_empty(),
            "no card in the pool writes the five subtypes at all, so the \
             printing half of this test is unexercised"
        );
    }

    /// Equip is `[cost]: Attach this permanent to target creature you control.
    /// Activate only as a sorcery.` (CR 702.6a-b). Three things follow that a
    /// card can get wrong while compiling perfectly: it does not tap the
    /// Equipment, it is not instant speed, and it cannot target a creature an
    /// opponent controls.
    ///
    /// Lightning Greaves got all three wrong while Swiftfoot Boots, the same
    /// shape, had them right — so this is not something the DSL fails to
    /// express, it is a rule that was being restated once per card instead of
    /// held in one place.
    #[test]
    fn every_equip_ability_is_sorcery_speed_untapped_and_targets_your_own() {
        use baylee_cards_dsl::{AbilityDef, ActivationTiming, CostPart, Effect};
        use baylee_core::generated::subtypes;

        let mut offenders = Vec::new();
        for (_, def) in generated::ALL {
            if !def
                .faces
                .iter()
                .any(|f| f.subtypes.contains(&subtypes::artifact::EQUIPMENT))
            {
                continue;
            }
            for (i, _) in def.faces.iter().enumerate() {
                for ability in def.abilities_for_face(i) {
                    // Both arms, so a conditional equip ability cannot walk
                    // past the lint. No card prints one today — the two
                    // sibling lints above already read both, and this one
                    // was written without them.
                    let (AbilityDef::Activated {
                        cost,
                        effects,
                        target,
                        timing,
                        ..
                    }
                    | AbilityDef::ActivatedConditional {
                        cost,
                        effects,
                        target,
                        timing,
                        ..
                    }) = ability
                    else {
                        continue;
                    };
                    if !effects
                        .iter()
                        .any(|e| matches!(e, Effect::AttachSelf { .. }))
                    {
                        continue;
                    }
                    let name = def.name();
                    if *timing != ActivationTiming::SorcerySpeed {
                        offenders.push(format!("{name}: equip at {timing:?}"));
                    }
                    if cost.parts.contains(&CostPart::TapSelf) {
                        offenders.push(format!("{name}: equip taps the Equipment"));
                    }
                    if !format!("{target:?}").contains("ControlledByYou") {
                        offenders.push(format!("{name}: equip targets any creature"));
                    }
                }
            }
        }
        offenders.sort();
        assert!(offenders.is_empty(), "equip breaks CR 702.6: {offenders:?}");
    }
}
