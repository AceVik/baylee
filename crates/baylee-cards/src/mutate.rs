//! The L5 mutation switch: `BAYLEE_MUTATE=<card>:<index>` takes one ability
//! off one card wherever the registry hands the card out.
//!
//! The card-verification ladder (`docs/verification-hooks.md`) asks of a
//! card that disabling any one of its abilities makes at least one of its
//! tests fail. The consumer runs the prebuilt test binary once per mutant with
//! a name filter, so the switch is an environment variable read once per
//! process, not a rebuild.
//!
//! Compiled for this crate's own tests and under the non-default `mutate`
//! feature, which only `baylee-engine`'s dev-dependencies enable: no binary
//! and no library build of the workspace carries it.
//!
//! # One door
//!
//! [`crate::by_index`], [`crate::by_oracle_id`] and [`crate::all`] are the
//! registry's doors, and every lookup a test runs the engine with ends in
//! the first of them. So the mutant is a whole `CardDef` handed out in the
//! card's place, and every reader of it sees the same card: the engine's
//! `abilities_for_face`, the places that read `def.abilities` straight, a
//! copy taking the card's copiable values, a granted ability living inside
//! one of the card's statics, and the test kit's own readers. What it does
//! not reach is a direct read of the generated tables
//! (`generated::ALL`, `generated::BY_INDEX`), which the engine never makes.
//!
//! # Replaced, not removed
//!
//! The ability is overwritten with [`AbilityDef::Unimplemented`], the
//! placeholder every reader already passes over, rather than cut out of the
//! list. An `AbilityRef` is a position in that list: cutting would move every
//! later ability down one, so a mutant of the first ability would also break
//! the tests of the second and third, and every kill would be counted against
//! the wrong ability.

use crate::generated;
use baylee_cards_dsl::{AbilityDef, CardDef, FaceDef};
use baylee_core::ids::{AbilityRef, CardIndex};
use std::io::Write as _;
use std::sync::OnceLock;

/// The environment variable naming the mutant.
pub const VAR: &str = "BAYLEE_MUTATE";

/// The exit status of a process asked for a mutant that is not one.
///
/// A malformed value, a card the registry does not have, or an index naming
/// no ability of it would otherwise run the tests against the unmutated card
/// and report the mutant as survived. Failing the test that first looks the
/// card up would be as wrong the other way, reported as killed. So the whole
/// process stops with this status, which is neither libtest's success (0)
/// nor its failure (101).
pub const INVALID_MUTANT_EXIT: i32 = 3;

/// The card handed out instead of the registry's, and which one it replaces.
struct Mutant {
    card: CardIndex,
    def: &'static CardDef,
}

static MUTANT: OnceLock<Option<Mutant>> = OnceLock::new();

/// `def`, or the mutant standing in for it.
pub(crate) fn apply(def: &'static CardDef) -> &'static CardDef {
    match MUTANT.get_or_init(from_env) {
        Some(mutant) if mutant.card == def.index => mutant.def,
        _ => def,
    }
}

fn from_env() -> Option<Mutant> {
    let spec = std::env::var_os(VAR)?;
    let spec = spec.to_string_lossy();
    // Empty is unset, so a script can write `BAYLEE_MUTATE=` for "no mutant".
    if spec.is_empty() {
        return None;
    }
    let made = parse(&spec).and_then(|(card, index)| {
        let def = generated::by_index(card)
            .ok_or_else(|| format!("the registry has no card {}", card.get()))?;
        mutant_of(def, index).map(|mutant| (def, index, mutant))
    });
    match made {
        Ok((def, index, mutant)) => {
            eprintln!(
                "{VAR}: {} (card {}) has ability {} replaced by Unimplemented",
                def.name(),
                def.index.get(),
                describe(index),
            );
            Some(Mutant {
                card: def.index,
                def: Box::leak(Box::new(mutant)),
            })
        }
        Err(why) => {
            // Straight to the process's stderr: libtest captures `eprintln!`
            // per test and would drop it with the process.
            let _ = writeln!(
                std::io::stderr(),
                "{VAR}={spec}: {why}; stopping with status {INVALID_MUTANT_EXIT}"
            );
            std::process::exit(INVALID_MUTANT_EXIT)
        }
    }
}

fn describe(index: u32) -> String {
    if index == AbilityRef::SPELL {
        format!("{index} (SPELL)")
    } else {
        index.to_string()
    }
}

/// `"<card>:<index>"`, both decimal `u32`s.
fn parse(spec: &str) -> Result<(CardIndex, u32), String> {
    let (card, index) = spec
        .split_once(':')
        .ok_or_else(|| "expected <card u32>:<index u32>".to_owned())?;
    let card: u32 = card
        .trim()
        .parse()
        .map_err(|_| format!("`{card}` is not a card index"))?;
    let index: u32 = index
        .trim()
        .parse()
        .map_err(|_| format!("`{index}` is not an ability index"))?;
    Ok((CardIndex::new(card), index))
}

/// `def` with the ability `index` names replaced by
/// [`AbilityDef::Unimplemented`] in every list that holds it.
///
/// `index` is a position in a list, or [`AbilityRef::SPELL`] for the card's
/// spell ability, which is whichever entry is an `AbilityDef::Spell` or
/// `AbilityDef::ModalSpell`. A position is replaced in the card-level list
/// and in every face's own list that is long enough, because an
/// `AbilityRef` names no face: that is what `CardDef::abilities_for_face`
/// reads for face 0 (its own list, else the card's) and for every other face
/// (its own). The other reserved indices name questions a card raises
/// (kicker, an as-enters choice), not entries, and are refused.
///
/// # Errors
/// When nothing would be replaced: an index past every list, a reserved
/// index other than `SPELL`, `SPELL` on a card with no spell ability, or an
/// entry that is already `Unimplemented`.
pub fn mutant_of(def: &'static CardDef, index: u32) -> Result<CardDef, String> {
    if index >= AbilityRef::FIRST_RESERVED && index != AbilityRef::SPELL {
        return Err(format!(
            "{index} is a reserved index that names no ability entry; only list positions \
             and SPELL ({}) do",
            AbilityRef::SPELL
        ));
    }
    let mut replaced = 0;
    let mut mutate = |list: &'static [AbilityDef]| -> &'static [AbilityDef] {
        let hit = |at: usize, ability: &AbilityDef| {
            if index == AbilityRef::SPELL {
                matches!(
                    ability,
                    AbilityDef::Spell { .. } | AbilityDef::ModalSpell { .. }
                )
            } else {
                at == index as usize && !matches!(ability, AbilityDef::Unimplemented)
            }
        };
        if !list.iter().enumerate().any(|(at, a)| hit(at, a)) {
            return list;
        }
        let out: Vec<AbilityDef> = list
            .iter()
            .enumerate()
            .map(|(at, ability)| {
                if hit(at, ability) {
                    replaced += 1;
                    AbilityDef::Unimplemented
                } else {
                    *ability
                }
            })
            .collect();
        Box::leak(out.into_boxed_slice())
    };
    let abilities = mutate(def.abilities);
    let faces: Vec<FaceDef> = def
        .faces
        .iter()
        .map(|face| FaceDef {
            abilities: mutate(face.abilities),
            ..*face
        })
        .collect();
    if replaced == 0 {
        return Err(format!(
            "{} (card {}) has no ability at index {}",
            def.name(),
            def.index.get(),
            describe(index)
        ));
    }
    Ok(CardDef {
        abilities,
        faces: Box::leak(faces.into_boxed_slice()),
        ..*def
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn by_oracle(oracle_id: &str) -> &'static CardDef {
        generated::by_oracle_id(oracle_id).expect("the card is in the pool")
    }

    /// Lightning Bolt's whole text is its spell ability, at position 0 of
    /// the card-level list; both spellings of it are the same mutant.
    #[test]
    fn the_spell_ability_is_replaced_by_its_position_or_by_spell() {
        let bolt = by_oracle("4457ed35-7c10-48c8-9776-456485fdf070");
        assert!(matches!(bolt.abilities, [AbilityDef::Spell { .. }]));
        for index in [0, AbilityRef::SPELL] {
            let mutant = mutant_of(bolt, index).expect("Bolt has a spell ability");
            assert!(matches!(mutant.abilities, [AbilityDef::Unimplemented]));
            assert_eq!(mutant.index, bolt.index);
            assert_eq!(mutant.name(), bolt.name());
            assert_eq!(mutant.faces.len(), bolt.faces.len());
        }
        assert!(mutant_of(bolt, 1).is_err(), "Bolt has one ability");
        assert!(
            mutant_of(bolt, AbilityRef::ENTERS).is_err(),
            "a reserved index other than SPELL names no entry"
        );
    }

    /// Replacing keeps every other ability where it was: the position is the
    /// ability's name, and the one after the mutant must still answer to its
    /// own.
    #[test]
    fn replacing_one_ability_moves_no_other() {
        let def = crate::all()
            .find(|d| d.abilities_for_face(0).len() >= 3)
            .expect("the pool has a card with three abilities");
        let printed = def.abilities_for_face(0);
        let mutant = mutant_of(def, 1).expect("position 1 exists");
        let list = mutant.abilities_for_face(0);
        assert_eq!(list.len(), printed.len(), "nothing was cut out");
        assert!(matches!(list[1], AbilityDef::Unimplemented));
        assert_eq!(list[0], printed[0]);
        assert_eq!(list[2..], printed[2..]);
    }

    #[test]
    fn a_malformed_spec_is_refused() {
        assert_eq!(parse("12:3"), Ok((CardIndex::new(12), 3)));
        assert!(parse("12").is_err());
        assert!(parse("x:3").is_err());
        assert!(parse("12:-1").is_err());
    }
}
