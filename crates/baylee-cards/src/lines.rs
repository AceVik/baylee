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

use baylee_cards_dsl::{AbilityDef, CopyMod, Effect, Modifier};
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
    /// Per mode this face offers, in `SpellMode` order: which printed
    /// sentence the mode is.
    ///
    /// A mode is not an ability and has no row in `lines`: the whole
    /// ability is one sentence or one block of them, and what a player
    /// picks between are the sentences *inside* it. The engine names one
    /// as `CastModeKind::Mode(i)` and says nothing else about it, so
    /// without this a chooser can only offer two numbers.
    ///
    /// Both kinds of modal ability, which [`face_modes`] is the reading
    /// of: a modal **spell** (Cyclonic Rift's overload) and a modal
    /// **trigger** (Charming Prince's "choose one" on entering). Reading
    /// the spell alone is how every one of the pool's six modal triggers
    /// arrived here with no row — see #54.
    ///
    /// Empty for a face with no modal ability, and `None` per mode whose
    /// printing `baylee_cards_codegen::lines::map_modes` could not read
    /// whole — including, by construction, the effect-less mode a player
    /// declines "choose up to one" with, which a card prints nowhere.
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

/// The modes one face offers, whichever kind of modal ability offers them.
///
/// Both variants, and that is the whole of the fix this function exists
/// for. [`baylee_cards_dsl::AbilityDef::ModalTriggered`] is `ModalSpell`'s
/// forgotten twin — the same shape as `Activated`/`ActivatedConditional`,
/// the same kind of miss — and reading only the spell left every modal
/// *trigger* in the pool with an empty [`FaceLines::modes`] row — six
/// cards, whose chooser drew "Mode 1 / Mode 2" over a card that prints the
/// sentences: Charming Prince; Aether Channeler; Ertai Resurrected;
/// Primaris Eliminator; Derevi, Empyrial Tactician; and Inspirit, Flagship
/// Vessel. Nothing said so, because an empty row is also what a card with
/// no modal ability at all has. See #54.
///
/// It takes the **first** modal ability's modes and is right to, but only
/// because a lint says so. `CastModeKind::Mode(i)` names a mode and not
/// the ability it belongs to — the engine's `ChooseCastMode` carries an
/// `ObjectId` and nothing else — so a face whose modal abilities offered
/// *different* modes could not be labelled at all, whatever this table
/// held. Derevi is the pool's face with two of them: one printed sentence,
/// two trigger conditions, two `modal_triggered!` abilities over one set
/// of modes. `a_face_offers_one_set_of_modes` is what keeps that true as
/// cards are added, rather than this paragraph.
///
/// It lives here rather than in `xtask` because both ends need the same
/// reading: the walk that *writes* the table, and the tests that hold the
/// table against the pool. Two spellings of it is how the first one went
/// unnoticed.
#[must_use]
pub fn face_modes(
    abilities: &[baylee_cards_dsl::AbilityDef],
) -> &'static [baylee_cards_dsl::SpellMode] {
    use baylee_cards_dsl::AbilityDef;

    abilities
        .iter()
        .find_map(|a| match a {
            AbilityDef::ModalSpell { modes } | AbilityDef::ModalTriggered { modes, .. } => {
                Some(*modes)
            }
            _ => None,
        })
        .unwrap_or(&[])
}

/// Which printed sentence one **mode** is, if it is known.
///
/// The twin of [`ability_line`] for `CastModeKind::Mode(i)`, and every
/// bound is checked here for the same reason: a cast chooser builds its
/// rows from whatever the engine has just offered, so a card that has
/// since changed face, a mode index from a pool the client does not have,
/// and a card with no modal ability at all must all answer `None` rather
/// than take the game down over a label.
///
/// A mode of a modal **trigger** is asked for through here too — the
/// engine asks both with one `Pending::ChooseCastMode` — which is what
/// [`face_modes`] is the reading of.
///
/// The face is **0** for every caller there is today — `cast_wizard` reads
/// `def.abilities_for_face(0)` when it enumerates modes, and every modal
/// trigger in the pool is on a front face — but it is asked for rather
/// than assumed, because that is a fact about the callers and not about
/// this table.
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

/// Which of the three ways a card writes an activated ability it grants.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GrantDoor {
    /// The ability is a static whose modifier is the grant: Chromatic
    /// Lantern's lands, Great Divide Guide's.
    Static,
    /// An effect the ability resolves creates the grant: Urza's Saga's
    /// chapters, Spawning Pool's `{1}{B}:`.
    Effect,
    /// A copy clause gives it to the copy (CR 707.9a): Machine God's
    /// Effigy's `…except it has "{T}: Add {U}."`.
    Copy,
}

/// Every `Modifier::GrantActivated` one ability writes, and through which
/// door.
///
/// The one walk over the three doors, for the pool lint that holds a
/// granted ability to CR 605.1 and for [`grant_home`], which a view asks
/// whose sentence a grant is. Two walks would be a grant the lint checks
/// and the view cannot find, or the other way round. `seen` counts the
/// effects read, which is what makes a door that reports nought news
/// rather than silence (`Effect::walk`).
///
/// Every variant is matched by name, so a new kind of ability is a compile
/// error here and not a door this walk quietly never opens.
pub fn grants_in(
    ability: &'static AbilityDef,
    seen: &mut usize,
) -> Vec<(GrantDoor, &'static Modifier)> {
    let is_grant = |m: &Modifier| matches!(m, Modifier::GrantActivated { .. });
    let mut found = Vec::new();
    let lists: Vec<&'static [Effect]> = match ability {
        AbilityDef::Static(rule) => {
            if is_grant(&rule.modifier) {
                found.push((GrantDoor::Static, &rule.modifier));
            }
            Vec::new()
        }
        AbilityDef::CopyOnEnter { mods, .. } | AbilityDef::CopyOnEnterUntilEot { mods, .. } => {
            for m in *mods {
                if let CopyMod::Grant(modifier) = m
                    && is_grant(modifier)
                {
                    found.push((GrantDoor::Copy, *modifier));
                }
            }
            Vec::new()
        }
        AbilityDef::Spell { effects, .. }
        | AbilityDef::Triggered { effects, .. }
        | AbilityDef::Activated { effects, .. }
        | AbilityDef::ActivatedConditional { effects, .. }
        | AbilityDef::SagaChapter { effects, .. }
        | AbilityDef::Loyalty { effects, .. } => vec![*effects],
        AbilityDef::ModalSpell { modes } | AbilityDef::ModalTriggered { modes, .. } => {
            modes.iter().map(|m| m.effects).collect()
        }
        // Nothing here resolves through an effect list a card wrote: a
        // keyword the engine synthesises, a cost, a replacement, a choice
        // made as the permanent enters.
        AbilityDef::Unimplemented
        | AbilityDef::Ward { .. }
        | AbilityDef::Prepared { .. }
        | AbilityDef::Echo { .. }
        | AbilityDef::Replacement(_)
        | AbilityDef::Suspend { .. } => Vec::new(),
    };
    for effects in lists {
        Effect::walk(effects, seen, &mut |effect| {
            if let Effect::CreateContinuousEffect { modifier, .. } = effect
                && is_grant(modifier)
            {
                found.push((GrantDoor::Effect, modifier));
            }
        });
    }
    found
}

/// Which of `abilities` writes `grant`: the index of the first whose
/// [`grants_in`] holds a modifier **equal** to it.
///
/// By value, and never by where the modifier lives in memory: two cards
/// that write the same grant share one constant, in release more than in
/// debug, so an address says nothing about which card wrote it
/// (`docs/card-identity.md`). Equal values on two cards are not a problem,
/// because the caller asks of one grantor's own list; equal values twice on
/// one face would be, and `every_grant_is_found_on_its_own_face` holds
/// that the pool has none.
///
/// `None` where no ability of the list writes it, and that is the answer
/// to pass on: a grant whose sentence is not found draws no sentence. It
/// is never the nearest one.
#[must_use]
pub fn grant_home(abilities: &'static [AbilityDef], grant: &Modifier) -> Option<usize> {
    abilities.iter().position(|ability| {
        grants_in(ability, &mut 0)
            .iter()
            .any(|(_, found)| *found == grant)
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
        // A static or a copy clause that grants an ability is placed too
        // (#212), and is no more a stack entry than a mana ability is: it
        // is counted apart for the same reason.
        let grants_on_no_stack = |ability: &'static baylee_cards_dsl::AbilityDef| {
            grants_in(ability, &mut 0)
                .iter()
                .any(|(door, _)| *door != GrantDoor::Effect)
        };
        let mut stackable = 0usize;
        let mut mapped = 0usize;
        let mut mana = 0usize;
        let mut grants = 0usize;
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
                    } else if abilities.get(at).is_some_and(grants_on_no_stack) {
                        grants += 1;
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
        assert!(
            grants >= 6,
            "{grants} statics and copy clauses know the sentence that grants \
             their ability; six did (#212)"
        );
    }

    /// Every grant the pool writes is found again on its own face, and the
    /// face has a sentence for it (#212).
    ///
    /// A view answers "whose sentence is this granted ability" with
    /// [`grant_home`], by value, so two things have to be true of the pool
    /// for its answer to be the right sentence. Both are asserted for every
    /// grant [`grants_in`] finds: `grant_home` lands on the ability that
    /// wrote it, which fails if one face writes an equal grant twice (the
    /// first would answer for both), and the table has a line there, which
    /// a static or a copy clause did not before codegen's `LineShape::Grant`.
    ///
    /// The floor is the grantors this was written against, one card each:
    /// Chromatic Lantern, Machine God's Effigy, Wrenn and Realmbreaker,
    /// Urza's Saga (two chapters), Forgotten Monument, Spawning Pool,
    /// Wandering Fumarole, Enduring Vitality, Great Divide Guide.
    #[test]
    fn every_grant_is_found_on_its_own_face() {
        let mut grantors = 0usize;
        let mut grants = 0usize;
        let mut seen = 0usize;
        let mut wrong = Vec::new();
        for (def, card) in crate::generated::BY_INDEX.iter().zip(ABILITY_LINES) {
            let Some(def) = def else { continue };
            let before = grants;
            for (face, lines) in card.iter().enumerate() {
                let abilities = def.abilities_for_face(face);
                for (at, ability) in abilities.iter().enumerate() {
                    for (door, grant) in grants_in(ability, &mut seen) {
                        grants += 1;
                        let home = grant_home(abilities, grant);
                        if home != Some(at) {
                            wrong.push(format!(
                                "{} face {face}: the {door:?} grant of ability {at} is found at {home:?}",
                                def.name()
                            ));
                        }
                        if lines.lines.get(at).copied().flatten().is_none() {
                            wrong.push(format!(
                                "{} face {face}: ability {at} grants through {door:?} and has no line",
                                def.name()
                            ));
                        }
                    }
                }
            }
            grantors += usize::from(grants > before);
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
        assert!(
            grantors >= 9,
            "{grantors} cards write {grants} grants over {seen} effects read; nine cards did"
        );
    }

    /// The lookup is exact: a grant no ability of the list writes has no
    /// home, even beside one that differs only in what it costs.
    ///
    /// The counter-test to the walk above, which could pass with a lookup
    /// that answered the first grant of a face for anything at all.
    #[test]
    fn a_grant_nobody_wrote_has_no_home() {
        let lantern = crate::all()
            .find(|def| def.name() == "Chromatic Lantern")
            .expect("the Lantern is in the pool");
        let abilities = lantern.abilities_for_face(0);
        let written = grants_in(&abilities[0], &mut 0)[0].1;
        assert_eq!(grant_home(abilities, written), Some(0));
        let Modifier::GrantActivated {
            effects,
            mana_ability,
            ..
        } = *written
        else {
            panic!("the Lantern grants an activated ability");
        };
        let dearer = Modifier::GrantActivated {
            cost: baylee_cards_dsl::Cost {
                mana: baylee_core::mana::ManaCost::parse("{1}"),
                ..baylee_cards_dsl::Cost::TAP
            },
            effects,
            mana_ability,
        };
        assert_eq!(grant_home(abilities, &dearer), None);
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
    ///
    /// Read again on 20.09.2026, at **three**: one card batch added Hostile
    /// Hostel and Balamb Garden in a single round, which is the part that
    /// matters more than the number. One was a curiosity; three arriving two
    /// at a time from a generator says the population grows with the pool
    /// rather than with the years. Still not enough to pay for a face on
    /// every `AbilityRef` — the cost is on the wire and on every caller —
    /// so the answer stands and the trigger is now the rate, not the count.
    /// #162 carries the decision so it is scheduled rather than rediscovered
    /// by whoever this test stops next.
    ///
    /// Read again on 22.09.2026, at **twelve**, and the jump is not what it
    /// looks like: nine of the nine new names were already in the pool and
    /// the table had no row for them, because `generated_lines.rs` is
    /// rebuilt from the compiled pool by hand and nobody had rebuilt it.
    /// Three was never the population — it was what a stale table could
    /// see, and this pin under-reported by four times for as long as that
    /// lasted. So the number moves and the reading does not: what #162 asks
    /// is whether the population grows, and a count taken off a table that
    /// is only refreshed sometimes cannot answer that. The guard is in
    /// `the_table_is_parallel_to_the_registry`, which now refuses a card
    /// with abilities and no row at all.
    ///
    /// Read again on 24.09.2026, at **thirteen**: Metzali, Tower of Triumph,
    /// whose "deals 2 damage to each opponent" was written by hand in the
    /// commit that gave its front face `DealDamageEach`. One card, arriving
    /// with a DSL change rather than a generator round, so the rate #162
    /// watches did not move.
    ///
    /// Read again the same day, at **fourteen**: Grasping Shadows, whose
    /// Shadows' Lair spends a dread counter to draw once `counters::DREAD`
    /// gave the word an id. Hand-written again, one card, no generator
    /// round.
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
        assert_eq!(
            named,
            [
                "Conqueror's Galleon",
                "Treasure Map",
                "Vance's Blasting Cannons",
                "Hadana's Climb",
                "Journey to Eternity",
                "Path of Mettle",
                "Hostile Hostel",
                "Sheoldred",
                "Dowsing Device",
                "Grasping Shadows",
                "Ojer Kaslem, Deepest Growth",
                "Ojer Pakpatiq, Deepest Epoch",
                "Balamb Garden, SeeD Academy",
                "Sidequest: Catch a Fish"
            ],
            "the back faces that reach the stack"
        );
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
            // An empty row is "this card has no abilities", and it was also
            // "nobody has rebuilt this table since the card was written" —
            // one spelling for two things, which is how nine cards sat here
            // with no row and `the_back_of_a_card_is_a_rarity` counted three
            // where the pool held twelve. The table is two-phase, so this is
            // red between the codegen run that adds a card and the one after
            // it; that is the same bargain `decks::name_table_tests` already
            // takes, and it is what makes the counts below measurements
            // rather than a reading of whatever was last written down.
            assert!(
                !card.is_empty() || def.abilities.is_empty(),
                "{} has {} abilities and no row at all — run \
                 `cargo run -p xtask -- codegen --tables` twice",
                def.name(),
                def.abilities.len()
            );
            for (face, lines) in card.iter().enumerate() {
                assert!(
                    lines.lines.len() == def.abilities_for_face(face).len(),
                    "{} face {face} has {} abilities and {} rows",
                    def.name(),
                    def.abilities_for_face(face).len(),
                    lines.lines.len()
                );
                // Both modal variants, which is what this line was missing:
                // counted over `ModalSpell` alone it agreed with a table
                // that had no row for a single modal *trigger* in the
                // pool, and said so for as long as both were wrong
                // together. See #54.
                let modes = super::face_modes(def.abilities_for_face(face)).len();
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

    /// The pool's modal abilities whose choice is printed **inside** one
    /// sentence instead of as a bulleted list.
    ///
    /// Derevi prints "you may tap or untap target permanent" and Inspirit
    /// "put your choice of a +1/+1 counter or two charge counters" — one
    /// sentence carrying both modes, so neither mode *is* a sentence and
    /// the number a chooser falls back to is the honest label. A named
    /// list rather than a tolerance, so that a modal card added tomorrow
    /// that reads as unknown stops a build and is looked at, rather than
    /// joining these two in silence. Tireless Provisioner is the third and
    /// arrived exactly that way — "create a Food token or a Treasure token"
    /// is one sentence and two modes, read and admitted rather than
    /// tolerated.
    const MODES_PRINTED_INLINE: &[&str] = &[
        "Derevi, Empyrial Tactician",
        "Inspirit, Flagship Vessel",
        "Tireless Provisioner",
    ];

    /// Every mode and every alternative cost in the pool knows which
    /// sentence it is, or there is a printed reason it cannot.
    ///
    /// An alternative cost is an **equality**, which is what separates it
    /// from `nearly_every_stack_ability_knows_its_printed_sentence` above.
    /// An ability is allowed to have no sentence of its own — a static is
    /// many-to-one with the printing, and evoke's trigger is printed as a
    /// keyword line — while an alternative cost *is* a printed sentence by
    /// construction: a card states what you may do instead, or it does not
    /// offer the option at all. A miss there is the reader needing work
    /// rather than a fact about the pool, and what it costs is the thing
    /// this table exists to remove — a chooser row reading "Mode 2".
    ///
    /// A **mode** was the same equality until modal triggers reached this
    /// table at all (#54), and the wider population is what showed the
    /// claim to be a fact about modal *spells* rather than about modes.
    /// Two printings answer to nothing this table can hold: a mode that
    /// does nothing, which is how a player declines "choose up to one" and
    /// which is printed nowhere (Ertai Resurrected's third), and a choice
    /// stated inside one sentence ([`MODES_PRINTED_INLINE`]). Both are
    /// asserted from the side that says which — an effect-less mode must
    /// know *no* sentence, and the inline ones are named — so the
    /// exception cannot quietly widen.
    #[test]
    fn every_mode_and_alternative_cost_knows_its_printed_sentence() {
        use baylee_cards_dsl::AbilityDef;

        let mut modes_known = 0usize;
        let mut modal_faces = 0usize;
        let mut alternatives_known = 0usize;
        let mut inline: Vec<&str> = Vec::new();
        for (def, card) in crate::generated::BY_INDEX.iter().zip(ABILITY_LINES) {
            let Some(def) = def else { continue };
            for (face, lines) in card.iter().enumerate() {
                let abilities = def.abilities_for_face(face);
                let modes = super::face_modes(abilities);
                let is_spell = abilities
                    .iter()
                    .any(|a| matches!(a, AbilityDef::ModalSpell { .. }));
                if !modes.is_empty() {
                    modal_faces += 1;
                }
                for (at, line) in lines.modes.iter().enumerate() {
                    let mode = modes.get(at).unwrap_or_else(|| {
                        panic!("{} face {face} has a row for no mode {at}", def.name())
                    });
                    if mode.effects.is_empty() {
                        assert!(
                            line.is_none(),
                            "{} face {face} mode {at} does nothing, and a card prints no sentence for declining",
                            def.name()
                        );
                        continue;
                    }
                    if line.is_some() {
                        modes_known += 1;
                    } else {
                        assert!(
                            !is_spell,
                            "{} face {face} mode {at} knows no sentence",
                            def.name()
                        );
                        inline.push(def.name());
                    }
                }
                for (at, line) in lines.alternatives.iter().enumerate() {
                    assert!(
                        line.is_some(),
                        "{} face {face} alternative cost {at} knows no sentence",
                        def.name()
                    );
                    alternatives_known += 1;
                }
            }
        }
        inline.sort_unstable();
        inline.dedup();
        assert_eq!(
            inline, MODES_PRINTED_INLINE,
            "a modal trigger whose modes have no printed sentence is named here or it is a defect"
        );
        assert!(
            modal_faces >= 10,
            "only {modal_faces} faces carry a row of modes; the pool has 10 \
             (four modal spells and six modal triggers)"
        );
        assert!(
            modes_known >= 19,
            "only {modes_known} modes know their sentence; the pool printed 19"
        );
        assert!(
            alternatives_known >= 8,
            "only {alternatives_known} alternative costs know their sentence; the pool printed 8"
        );
    }

    /// A mode index counts the modes of **one** set.
    ///
    /// `cast_wizard` walks every `AbilityDef::ModalSpell` on the face and
    /// enumerates each one's modes from zero, so a face carrying two of
    /// them would offer two different modes under the same
    /// `CastModeKind::Mode(0)`. That is an ambiguity in the engine's own
    /// handle before it is one in this table. The pool has never printed
    /// such a card; this is what says so rather than the comment that used
    /// to.
    ///
    /// The second half is the same argument for *modes* rather than for
    /// abilities, and it is what makes [`super::face_modes`] — which takes
    /// the first modal ability's — a contract instead of a guess. A modal
    /// trigger is not cast, so two of them on one face is a shape the
    /// engine allows and Derevi prints: one sentence, two trigger
    /// conditions, two `modal_triggered!` abilities. They may sit there
    /// because they offer the *same* modes; two that differed would put
    /// one ability's sentence on the other's mode, and nothing downstream
    /// could tell, because `Mode(i)` names no ability.
    #[test]
    fn a_face_offers_one_set_of_modes() {
        use baylee_cards_dsl::AbilityDef;

        let mut seen = 0usize;
        for def in crate::all() {
            for face in 0..def.faces.len() {
                let abilities = def.abilities_for_face(face);
                let spells = abilities
                    .iter()
                    .filter(|a| matches!(a, AbilityDef::ModalSpell { .. }))
                    .count();
                assert!(
                    spells <= 1,
                    "{} face {face} prints {spells} modal spells, and `Mode(i)` names one",
                    def.name()
                );
                let sets: Vec<&'static [baylee_cards_dsl::SpellMode]> = abilities
                    .iter()
                    .filter_map(|a| match a {
                        AbilityDef::ModalSpell { modes }
                        | AbilityDef::ModalTriggered { modes, .. } => Some(*modes),
                        _ => None,
                    })
                    .collect();
                let Some(first) = sets.first() else { continue };
                seen += 1;
                assert!(
                    sets.iter().all(|modes| modes == first),
                    "{} face {face} carries {} modal abilities offering different modes, \
                     and `Mode(i)` names no ability",
                    def.name(),
                    sets.len()
                );
            }
        }
        assert!(
            seen >= 10,
            "only {seen} faces carry a modal ability at all, so this proves nothing; \
             the pool has 10"
        );
    }
}
