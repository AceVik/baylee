//! What a seat answered over a socket may be told about the decision it is
//! asked: [`DecisionContext`] with nothing borrowed, so it can ride beside
//! the question (`docs/protocol.md` §"What a question is about").
//!
//! An in-process house reads the engine's [`DecisionContext`]; a seat has
//! only what it is sent. The note is that context's public part — which
//! printed ability or mode asks, the X, the cast route, how far a resolving
//! ability has come — named by identities the seat already holds, so the
//! seat rebuilds the same context from the card registry
//! (`baylee_ai::noted`). It adds nothing to what the asked seat may see: the
//! host builds it only for the seat that answers the question, and leaves
//! the asking card's identity out where that seat may not see the source
//! (`baylee-gamehost`'s `view::decision_note`).

use super::{DecisionContext, Engine};
use crate::choice::CastModeKind;
use crate::state::CardLookup;
use baylee_cards_dsl::AbilityDef;
use baylee_core::ids::{CardIndex, ObjectId};
use baylee_core::mana::ManaCost;
use serde::{Deserialize, Serialize};

/// The public part of a [`DecisionContext`], owned.
///
/// A frame carries every field (`baylee_core::wire::left_out`); JSON leaves
/// out a field at its default, and a note with nothing in it is not sent at
/// all.
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is one fact the engine's context states about the question"
)]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionNote {
    /// The object whose spell or ability asks.
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub source: Option<ObjectId>,
    /// The card and face whose printed text holds the asking ability.
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub printed: Option<(CardIndex, u8)>,
    /// The registry token whose definition holds it, where a token's does.
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub token: Option<u16>,
    /// Which ability of that printed list asks.
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub ability: Option<u32>,
    /// The mode chosen, of a modal ability.
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub mode: Option<u32>,
    /// The whole spell face explains the question, not one sentence.
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub whole_spell: bool,
    /// How many of a resolving ability's operations have run: the question
    /// is asked by the rest.
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub done: Option<u32>,
    /// The cast's mana cost before X, while a cast is being announced.
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub cost: Option<ManaCost>,
    /// The casting route chosen.
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub cast_mode: Option<CastModeKind>,
    /// An alternative cost on that route.
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub alternative_cost: Option<CastModeKind>,
    /// X also costs life.
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub life_x: bool,
    /// The X announced.
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub x: u32,
    /// Distinct targets available, where the spell needs X of them.
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub x_targets: Option<u32>,
    /// The question is the second instance of the word "target".
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub second_instance: bool,
    /// What the first instance chose, while the second is asked.
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub first_targets: Vec<ObjectId>,
    /// The question is which object the source enters as a copy of.
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub copying: bool,
    /// Where the copy's modifications are printed: a card, its face and the
    /// copy ability's place in that face's list.
    #[serde(default, skip_serializing_if = "baylee_core::wire::left_out")]
    pub copy_mods: Option<(CardIndex, u8, u32)>,
}

impl DecisionNote {
    /// Whether the note says nothing: the question explains itself.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The note without the asking card's identity: what a seat may be told
    /// about a source it may not see. The route, the X and the targets it
    /// has already named stay; which card asks does not.
    #[must_use]
    pub fn without_identity(self) -> Self {
        Self {
            source: None,
            printed: None,
            token: None,
            ability: None,
            mode: None,
            whole_spell: false,
            done: None,
            copy_mods: None,
            ..self
        }
    }
}

impl<L: CardLookup> Engine<L> {
    /// The public part of [`Self::decision_context`], owned
    /// ([`DecisionNote`]). Like the context it explains the current choice
    /// to whoever answers it; the host decides who that may be.
    #[must_use]
    pub fn decision_note(&self) -> DecisionNote {
        let context = self.decision_context();
        // A resolving spell or ability asks with the part of its program
        // still to run, a slice of the resolution's own list; a context
        // from anywhere else borrows a printed list, never that one. So the
        // slice's place in the list is how far the program has run, read off
        // the addresses rather than from which branch explained it.
        let done = self.resolution.as_ref().and_then(|res| {
            let list = res.effects.as_ptr_range();
            let at = context.effects.as_ptr();
            let size = std::mem::size_of::<baylee_cards_dsl::Effect>();
            (size > 0 && !res.effects.is_empty() && (list.contains(&at) || at == list.end))
                .then(|| (at.addr() - list.start.addr()) / size)
                .and_then(|done| u32::try_from(done).ok())
        });
        let copy_mods = context
            .copying
            .zip(context.source)
            .and_then(|(mods, source)| self.copy_mods_origin(source, mods));
        note_of(&context, done, copy_mods)
    }

    /// The card, face and ability that print `mods` for `source`, where
    /// `source`'s own printing does.
    fn copy_mods_origin(
        &self,
        source: ObjectId,
        mods: &'static [baylee_cards_dsl::CopyMod],
    ) -> Option<(CardIndex, u8, u32)> {
        let object = self.state.object(source)?;
        let printed = object
            .printed_face()
            .or_else(|| crate::object::PrintedFace::new(object.card?.index, 0))?;
        let def = self.lookup.card(printed.card())?;
        let at = def
            .abilities_for_face(usize::from(printed.face()))
            .iter()
            .position(|ability| {
                matches!(ability, AbilityDef::CopyOnEnter { mods: printed, .. }
                    if std::ptr::eq(*printed, mods))
            })?;
        Some((printed.card(), printed.face(), u32::try_from(at).ok()?))
    }
}

/// The note of `context`, with `done` of its program run.
fn note_of(
    context: &DecisionContext<'_>,
    done: Option<u32>,
    copy_mods: Option<(CardIndex, u8, u32)>,
) -> DecisionNote {
    let origin = context.provenance.and_then(|p| p.origin);
    DecisionNote {
        source: context.source,
        printed: context.printed.map(|p| (p.card(), p.face())),
        token: origin
            .and_then(crate::object::AbilityOrigin::token)
            .map(|id| id.get() - 1),
        ability: context.ability_index,
        mode: context.mode.and_then(|m| u32::try_from(m).ok()),
        whole_spell: context.whole_spell,
        done,
        cost: context.cost,
        cast_mode: context.cast_mode,
        alternative_cost: context.alternative_cost,
        life_x: context.life_x,
        x: context.x,
        x_targets: context.x_targets,
        second_instance: context.second_instance,
        first_targets: context.first_targets.to_vec(),
        copying: context.copying.is_some(),
        copy_mods,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame reads fields by position, so a note leaves none out on the
    /// wire however many are at their default; one written with fields
    /// skipped would not read back at all.
    #[test]
    fn a_note_reads_back_from_a_frame() {
        let full = DecisionNote {
            source: Some(ObjectId::new(7, 1)),
            printed: Some((CardIndex::new(47), 0)),
            ability: Some(0),
            mode: Some(1),
            done: Some(2),
            x: 3,
            first_targets: vec![ObjectId::new(9, 0)],
            copying: true,
            ..DecisionNote::default()
        };
        for note in [
            full.clone(),
            DecisionNote::default(),
            full.clone().without_identity(),
        ] {
            let bytes = baylee_core::wire::encode(&note);
            let read: DecisionNote = baylee_core::wire::decode(&bytes).expect("a note reads back");
            assert_eq!(read, note);
        }
        assert!(DecisionNote::default().is_empty());
        let bare = full.without_identity();
        assert_eq!(
            (bare.source, bare.printed, bare.ability),
            (None, None, None)
        );
        assert_eq!(bare.x, 3, "the X stays: the seat chose it");
    }
}
