//! Which printed sentence each of a card's abilities came from.
//!
//! The engine carries no card text, so an ability on the stack reaches a
//! client as `AbilityRef { card, index }` and nothing else — and a client
//! that can only say "Jace's second ability" draws a stack a player cannot
//! read. What it *can* do is fetch the printed text of the printing and
//! language that player chose, and print one sentence of it; all it needs
//! is which one.
//!
//! That answer cannot be computed at runtime. It comes from reading the
//! **English** oracle text against the compiled ability list
//! (`baylee_cards_codegen::lines`), and no English oracle text exists
//! anywhere in a running game. So it is precomputed:
//! `cargo xtask codegen` writes [`generated_lines::ABILITY_LINES`] and
//! `codegen --check` is what keeps it from going stale.
//!
//! [`generated_lines::ABILITY_LINES`]: crate::generated_lines::ABILITY_LINES
//!
//! # The unit is a face
//!
//! A card's abilities are per face ([`CardDef::abilities_for_face`] — face
//! 0 falls back to the card-level list, a back face never inherits), and a
//! client renders one face's text at a time. An index into the two faces'
//! sentences joined together would point into the face nobody is reading.
//!
//! [`CardDef::abilities_for_face`]: baylee_cards_dsl::CardDef::abilities_for_face
//!
//! `AbilityRef` carries no face, and deliberately still does not: it is
//! also the handle a player's standing answer ("always say yes to this")
//! is filed under, so a field on it is a protocol change — for exactly one
//! card in the pool, Sheoldred, whose back face is the only one that puts
//! an ability on the stack (`the_back_of_a_card_is_a_rarity` counts them).
//! Whoever looks a line up supplies the face it is asking about, which for
//! a stack entry is the face the source object is showing.
//!
//! # Why the sentence count travels with the index
//!
//! The index is into the **English** text; the client resolves it against
//! a *localized* one, which may be an older printing with pre-errata
//! wording or a translation that joins two lines into one. An index that
//! is merely out of range is caught by anyone; an index that is in range
//! and points one sentence off is shown to the player as precise text and
//! is worse than "+1". [`AbilityLine::of`] is what lets a client refuse
//! the whole answer when its own split came out a different length.

use baylee_core::ids::CardIndex;

/// One face's answer: how many sentences it prints, and where each of its
/// abilities came from.
///
/// Written by `cargo xtask codegen` into [`crate::generated_lines`] — the
/// only thing that ever constructs one.
pub struct FaceLines {
    /// How many sentences this face's English oracle text is made of
    /// (`baylee_core::oracle::sentences`).
    pub sentences: u8,
    /// How many of this face's abilities can be on the stack at all — an
    /// activated, triggered, loyalty or saga-chapter ability. A static
    /// never goes on the stack, so it is not a miss when it has no
    /// sentence of its own; this is the denominator that says so.
    pub stackable: u8,
    /// Per ability of this face, in `abilities_for_face` order: which
    /// sentence it came from. `None` for an ability that is not a stack
    /// entry, and for one no sentence fits — a keyword-printed echo or
    /// evoke trigger has no sentence of its own to point at.
    pub lines: &'static [Option<u8>],
    /// Per mode of this face's modal spell, in `SpellMode` order: which
    /// printed sentence the mode is.
    ///
    /// A mode is not an ability and has no row in `lines`: the whole card
    /// is the `AbilityDef::ModalSpell`'s text, and what a player picks
    /// between are the sentences *inside* it. The engine names one as
    /// `CastModeKind::Mode(i)` and says nothing else about it, so without
    /// this a chooser can only offer two numbers.
    ///
    /// Empty for a face with no modal spell, and all `None` for one whose
    /// printing `baylee_cards_codegen::lines::map_modes` could not read
    /// whole.
    pub modes: &'static [Option<u8>],
    /// Per alternative cost of this face, in `alternative_costs` order:
    /// which printed sentence states it.
    ///
    /// The twin of `modes` for `CastModeKind::Alternative(i)` — "Evoke—Exile
    /// a white card from your hand" rather than "Alternative cost". An
    /// alternative cost is not an ability either; it is a field on the face.
    pub alternatives: &'static [Option<u8>],
}

/// Where one ability's text is, and how long the text it indexes is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AbilityLine {
    /// 0-based index into the face's sentences.
    pub line: u8,
    /// How many sentences the English text has. A client whose own split
    /// of the localized text yields a different number must not use
    /// `line` — see the module docs.
    pub of: u8,
}

/// Which printed sentence an ability came from, if it is known.
///
/// Every bound is checked here, so a caller may pass anything: a retired
/// card index, a face a card does not have, and the reserved ability
/// indices that count down from `u32::MAX` (`AbilityRef::SPELL`,
/// `ENTERS`, …) all answer `None` rather than panicking. A stack entry is
/// projected for every view of every frame; a lookup that could panic
/// there would take the game down over a cosmetic label.
#[must_use]
pub fn ability_line(card: CardIndex, face: usize, index: u32) -> Option<AbilityLine> {
    let face = crate::generated_lines::ABILITY_LINES
        .get(card.get() as usize)?
        .get(face)?;
    let line = (*face.lines.get(usize::try_from(index).ok()?)?)?;
    Some(AbilityLine {
        line,
        of: face.sentences,
    })
}

/// Which printed sentence one **mode** of a modal spell is, if it is known.
///
/// The twin of [`ability_line`] for `CastModeKind::Mode(i)`, and every
/// bound is checked here for the same reason: a cast chooser builds its
/// rows from whatever the engine has just offered, so a card that has
/// since changed face, a mode index from a pool the client does not have,
/// and a card with no modal spell at all must all answer `None` rather
/// than take the game down over a label.
///
/// The face is **0** for every caller there is today — the engine reads
/// `def.abilities_for_face(0)` when it enumerates modes, so `Mode(i)` is
/// always about the front — but it is asked for rather than assumed,
/// because that is a fact about `cast_wizard` and not about this table.
#[must_use]
pub fn mode_line(card: CardIndex, face: usize, mode: usize) -> Option<AbilityLine> {
    let face = crate::generated_lines::ABILITY_LINES
        .get(card.get() as usize)?
        .get(face)?;
    let line = (*face.modes.get(mode)?)?;
    Some(AbilityLine {
        line,
        of: face.sentences,
    })
}

/// Which printed sentence one **alternative cost** is stated by, if it is
/// known.
///
/// The twin of [`mode_line`] for `CastModeKind::Alternative(i)`. A face's
/// `alternative_costs` is what the index counts, in printed order.
#[must_use]
pub fn alternative_line(card: CardIndex, face: usize, alt: usize) -> Option<AbilityLine> {
    let face = crate::generated_lines::ABILITY_LINES
        .get(card.get() as usize)?
        .get(face)?;
    let line = (*face.alternatives.get(alt)?)?;
    Some(AbilityLine {
        line,
        of: face.sentences,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated_lines::ABILITY_LINES;

    /// Every bound is the lookup's own, because a stack entry is projected
    /// for every view of every frame and the four things a caller can get
    /// wrong are all ordinary: a retired card, a face the card does not
    /// have, an ability list shorter than the index, and the reserved
    /// indices that count down from `u32::MAX` — which are not abilities
    /// at all and are the *common* case, since every spell on the stack
    /// arrives as one.
    #[test]
    fn a_lookup_out_of_any_bound_answers_nothing_rather_than_panicking() {
        let jace = crate::all()
            .find(|def| def.name() == "Jace, the Mind Sculptor")
            .expect("the pool has Jace");
        assert_eq!(
            ability_line(jace.index, 0, 0),
            Some(AbilityLine { line: 0, of: 4 })
        );
        assert_eq!(
            ability_line(jace.index, 0, baylee_core::ids::AbilityRef::SPELL),
            None
        );
        assert_eq!(ability_line(jace.index, 0, 4), None, "one past his last");
        assert_eq!(ability_line(jace.index, 1, 0), None, "he has one face");
        assert_eq!(ability_line(CardIndex::new(u32::MAX), 0, 0), None);
    }

    /// The floor, counted in **abilities**.
    ///
    /// Faces are the wrong unit: a face with four abilities and a face
    /// with one count the same, and the feature is about abilities. The
    /// number is a floor rather than an equality because the pool grows
    /// and a card that reads cleanly should not have to be added here —
    /// but it may not *fall*, which is what a regression in
    /// `baylee_cards_codegen::lines` would look like.
    ///
    /// Both halves are bounded. Without the second, the mapper could
    /// answer the first by simply deciding fewer abilities are stack
    /// entries at all.
    ///
    /// **A mana ability is placed and is not counted here.** It does not
    /// use the stack (CR 605.1), so [`FaceLines::stackable`] has never
    /// included one — and the table places one anyway now, for the ability
    /// sheet to draw the card's own sentence on a mana row. Adding those to
    /// `mapped` would make the ratio climb on its own, which is exactly the
    /// drift that hides a regression, so the walk asks the registry which
    /// abilities are mana and counts them separately.
    #[test]
    fn nearly_every_stack_ability_knows_its_printed_sentence() {
        let mut stackable = 0usize;
        let mut mapped = 0usize;
        let mut mana = 0usize;
        for (def, card) in crate::generated::BY_INDEX.iter().zip(ABILITY_LINES) {
            for (face, lines) in card.iter().enumerate() {
                stackable += lines.stackable as usize;
                let abilities = def.map_or(&[][..], |def| def.abilities_for_face(face));
                for (at, line) in lines.lines.iter().enumerate() {
                    if line.is_none() {
                        continue;
                    }
                    if abilities.get(at).is_some_and(is_mana_ability) {
                        mana += 1;
                    } else {
                        mapped += 1;
                    }
                }
            }
        }
        assert!(
            stackable >= 327,
            "the pool offers only {stackable} stack-capable abilities; it had 327"
        );
        assert!(
            mapped >= 318,
            "{mapped} of {stackable} stack abilities know their sentence; 318 did"
        );
        assert!(
            mana >= 400,
            "{mana} mana abilities know their printed sentence"
        );
    }

    /// Whether an ability is one the stack never sees (CR 605.1).
    fn is_mana_ability(ability: &baylee_cards_dsl::AbilityDef) -> bool {
        matches!(
            ability,
            baylee_cards_dsl::AbilityDef::Activated {
                mana_ability: true,
                ..
            } | baylee_cards_dsl::AbilityDef::ActivatedConditional {
                mana_ability: true,
                ..
            }
        )
    }

    /// A sentence index has to be *in* the text it indexes.
    ///
    /// The two numbers are written from different expressions — one counts
    /// the face's sentences, the other walks its abilities — so a renderer
    /// that paired a face's abilities with the *other* face's text would
    /// pass every count above and fail here.
    #[test]
    fn no_ability_points_past_the_end_of_its_own_face() {
        for (index, card) in ABILITY_LINES.iter().enumerate() {
            for (face, lines) in card.iter().enumerate() {
                for (ability, line) in lines.lines.iter().enumerate() {
                    let Some(line) = line else { continue };
                    assert!(
                        *line < lines.sentences,
                        "card {index} face {face} ability {ability} points at sentence \
                         {line} of a face with {} of them",
                        lines.sentences
                    );
                }
            }
        }
    }

    /// How much the missing face on `AbilityRef` actually costs, as a
    /// number rather than as a guess.
    ///
    /// A pool-wide claim in a comment is a claim a test can hold (it has
    /// been wrong before), and this one decides a protocol change: if a
    /// second, a tenth, a hundredth card turns up whose back face puts an
    /// ability on the stack, the caller supplying the face is no longer
    /// obviously the cheaper answer and this should be read again.
    #[test]
    fn the_back_of_a_card_is_a_rarity() {
        let named: Vec<&str> = crate::generated::BY_INDEX
            .iter()
            .zip(ABILITY_LINES)
            .filter_map(|(def, card)| {
                let back = card.get(1..)?;
                back.iter()
                    .any(|face| face.stackable > 0)
                    .then(|| def.map_or("<retired>", |d| d.name()))
            })
            .collect();
        assert_eq!(named, ["Sheoldred"], "the back faces that reach the stack");
    }

    /// The table is indexed by `CardIndex`, so a row that is not the card
    /// the index names is a lookup that answers with a stranger's text.
    #[test]
    fn the_table_is_parallel_to_the_registry() {
        assert!(
            ABILITY_LINES.len() <= crate::generated::BY_INDEX.len(),
            "the line table names more cards than the registry has"
        );
        for (index, card) in ABILITY_LINES.iter().enumerate() {
            let Some(def) = crate::generated::BY_INDEX[index] else {
                assert!(card.is_empty(), "retired card {index} has lines");
                continue;
            };
            assert!(
                card.is_empty() || card.len() == def.faces.len(),
                "{} has {} faces and {} rows",
                def.name(),
                def.faces.len(),
                card.len()
            );
            for (face, lines) in card.iter().enumerate() {
                assert!(
                    lines.lines.len() == def.abilities_for_face(face).len(),
                    "{} face {face} has {} abilities and {} rows",
                    def.name(),
                    def.abilities_for_face(face).len(),
                    lines.lines.len()
                );
                let modes = def
                    .abilities_for_face(face)
                    .iter()
                    .find_map(|a| match a {
                        baylee_cards_dsl::AbilityDef::ModalSpell { modes } => Some(modes.len()),
                        _ => None,
                    })
                    .unwrap_or(0);
                assert!(
                    lines.modes.len() == modes,
                    "{} face {face} prints {modes} modes and {} rows",
                    def.name(),
                    lines.modes.len()
                );
                let alternatives = def.faces[face].alternative_costs.len();
                assert!(
                    lines.alternatives.len() == alternatives,
                    "{} face {face} prints {alternatives} alternative costs and {} rows",
                    def.name(),
                    lines.alternatives.len()
                );
            }
        }
    }

    /// Every mode and every alternative cost in the pool knows which
    /// sentence it is.
    ///
    /// An **equality** rather than a floor, which is what separates it from
    /// `nearly_every_stack_ability_knows_its_printed_sentence` above. An
    /// ability is allowed to have no sentence of its own — a static is
    /// many-to-one with the printing, and evoke's trigger is printed as a
    /// keyword line — while a mode and an alternative cost *are* printed
    /// sentences by construction: a card states what you may do instead, or
    /// it does not offer the option at all. A miss here is therefore the
    /// reader needing work rather than a fact about the pool, and what it
    /// costs is the thing this table exists to remove — a chooser row
    /// reading "Mode 2".
    #[test]
    fn every_mode_and_alternative_cost_knows_its_printed_sentence() {
        let mut seen = 0usize;
        for (def, card) in crate::generated::BY_INDEX.iter().zip(ABILITY_LINES) {
            let Some(def) = def else { continue };
            for (face, lines) in card.iter().enumerate() {
                for (at, line) in lines.modes.iter().enumerate() {
                    assert!(
                        line.is_some(),
                        "{} face {face} mode {at} knows no sentence",
                        def.name()
                    );
                    seen += 1;
                }
                for (at, line) in lines.alternatives.iter().enumerate() {
                    assert!(
                        line.is_some(),
                        "{} face {face} alternative cost {at} knows no sentence",
                        def.name()
                    );
                    seen += 1;
                }
            }
        }
        assert!(
            seen >= 17,
            "the pool prints only {seen} cast options with a sentence; it printed 17"
        );
    }

    /// A mode index counts the modes of **one** modal spell.
    ///
    /// `cast_wizard` walks every `AbilityDef::ModalSpell` on the face and
    /// enumerates each one's modes from zero, so a face carrying two of
    /// them would offer two different modes under the same
    /// `CastModeKind::Mode(0)`. That is an ambiguity in the engine's own
    /// handle before it is one in this table — which reads the first modal
    /// spell and would be describing whichever the engine's walk reached
    /// first. The pool has never printed such a card; this is what says so
    /// rather than the comment that used to.
    #[test]
    fn a_face_prints_at_most_one_modal_spell() {
        for def in crate::all() {
            for face in 0..def.faces.len() {
                let modal = def
                    .abilities_for_face(face)
                    .iter()
                    .filter(|a| matches!(a, baylee_cards_dsl::AbilityDef::ModalSpell { .. }))
                    .count();
                assert!(
                    modal <= 1,
                    "{} face {face} prints {modal} modal spells, and `Mode(i)` names one",
                    def.name()
                );
            }
        }
    }
}
