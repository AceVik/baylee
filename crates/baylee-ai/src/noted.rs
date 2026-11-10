//! A [`DecisionContext`] rebuilt from the [`DecisionNote`] a seat is sent
//! beside its question, for a house that answers over a socket
//! (`baylee_seat::HouseMind`).
//!
//! The in-process house reads the engine's context; a seat reads the note
//! and this rebuilds the same context from the card registry: the asking
//! ability's effects from the printed card (or token) and the ability's
//! place in its list, the chosen mode's, and for a resolving ability only
//! the part of its program still to run. What the registry cannot give
//! back — an ability a continuous effect grants, a reflexive trigger's
//! body — is left empty, as the in-process house leaves an ability it
//! cannot read, and the answer falls back to the view as before.

use crate::HeuristicAgent;
use crate::intelligence::DeckIntel;
use baylee_cards_dsl::{AbilityDef, CopyMod, Effect};
use baylee_core::ids::SeatSet;
use baylee_core::preset::AIProfile;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::{DecisionContext, DecisionNote, ability_effects};
use baylee_engine::object::PrintedFace;
use baylee_view::PlayerView;

/// The context a note names, owned, so a [`DecisionContext`] can borrow it.
#[derive(Clone, Debug, Default)]
pub struct Noted {
    note: DecisionNote,
    effects: Vec<Effect>,
    copying: Option<&'static [CopyMod]>,
}

impl Noted {
    /// Rebuilds what `note` names.
    #[must_use]
    pub fn new(note: DecisionNote) -> Self {
        let effects = asking_effects(&note);
        let copying = note.copying.then(|| copy_mods(&note).unwrap_or(&[]));
        Self {
            note,
            effects,
            copying,
        }
    }

    /// The note this was rebuilt from.
    #[must_use]
    pub const fn note(&self) -> &DecisionNote {
        &self.note
    }

    /// The context, as the in-process house would be handed it.
    #[must_use]
    pub fn context(&self) -> DecisionContext<'_> {
        let note = &self.note;
        DecisionContext {
            source: note.source,
            printed: note
                .printed
                .and_then(|(card, face)| PrintedFace::new(card, face)),
            provenance: None,
            ability_index: note.ability,
            mode: note.mode.and_then(|m| usize::try_from(m).ok()),
            whole_spell: note.whole_spell,
            effects: &self.effects,
            cost: note.cost,
            cast_mode: note.cast_mode,
            alternative_cost: note.alternative_cost,
            life_x: note.life_x,
            x: note.x,
            x_targets: note.x_targets,
            second_instance: note.second_instance,
            first_targets: &note.first_targets,
            copying: self.copying,
        }
    }
}

/// The printed list the asking ability is in: a token's, or a card face's.
fn printed_list(note: &DecisionNote) -> Option<&'static [AbilityDef]> {
    if let Some(token) = note.token {
        return baylee_cards::tokens::by_token_id(token).map(|def| def.abilities);
    }
    let (card, face) = note.printed?;
    Some(baylee_cards::by_index(card)?.abilities_for_face(usize::from(face)))
}

/// The effects the asking ability or spell asks with, what is still to run
/// of them where it is resolving.
fn asking_effects(note: &DecisionNote) -> Vec<Effect> {
    let Some(list) = printed_list(note) else {
        return Vec::new();
    };
    let mode = note.mode.and_then(|m| usize::try_from(m).ok());
    let ability = match note.ability {
        Some(at) => usize::try_from(at).ok().and_then(|at| list.get(at)),
        // A spell names no ability: its face's spell ability asks.
        None => list
            .iter()
            .find(|a| matches!(a, AbilityDef::Spell { .. } | AbilityDef::ModalSpell { .. })),
    };
    let Some(ability) = ability else {
        return Vec::new();
    };
    let effects = ability_effects(ability, mode);
    match note.done {
        // The engine runs a resolution over the flattened list
        // (`resolve::flatten`), and counts its place in that.
        Some(done) => {
            let program = baylee_engine::resolve::flatten(effects);
            usize::try_from(done)
                .ok()
                .and_then(|done| program.get(done..))
                .map(<[Effect]>::to_vec)
                .unwrap_or_default()
        }
        None => effects.to_vec(),
    }
}

/// The copy modifications the note says are printed for its source.
fn copy_mods(note: &DecisionNote) -> Option<&'static [CopyMod]> {
    let (card, face, at) = note.copy_mods?;
    let def = baylee_cards::by_index(card)?;
    match def
        .abilities_for_face(usize::from(face))
        .get(usize::try_from(at).ok()?)?
    {
        AbilityDef::CopyOnEnter { mods, .. } => Some(*mods),
        _ => None,
    }
}

/// The view the house answers from in a seat's chair, made from the view a
/// seat is sent: what `Session::agent_view` leaves out of a house chair's
/// view, left out again. The clock's remainder (wall time is not an input
/// to a house decision, #87), what the seat's standing policies answered
/// (told to the player, not to a decision), and who asked whom for a hand.
/// The hands teammates show the seat stay where its level plays as a team,
/// as the table's own house reads them (`AIProfile::TEAM_PLAY`); the frozen
/// reference keeps only what a rule reveals to everyone.
#[must_use]
pub fn seated_view(mut view: PlayerView, profile: AIProfile) -> PlayerView {
    view.decision_remaining_ms = None;
    view.policy_acts.clear();
    if !profile.plays(AIProfile::TEAM_PLAY) {
        view.shared_hands.retain(|hand| hand.public);
    }
    view.hand_shared_with = SeatSet::new();
    view.hand_requests = SeatSet::new();
    view.hand_requested = SeatSet::new();
    view
}

/// What the house answers in a seat's chair, from what the seat holds: the
/// view it is sent ([`seated_view`]), the note beside its question and the
/// deck it brought. The table's own house chair answers the same question
/// from the engine's context and the host's report of the same deck, and
/// the two agree (`baylee-seat`'s `seated_house` test). Without a deck the
/// answer is given without the report, as a seat that knows none.
#[must_use]
pub fn answer_as_seated(
    agent: &HeuristicAgent,
    view: PlayerView,
    pending: &Pending,
    note: Option<&DecisionNote>,
    deck: Option<&DeckIntel>,
) -> PlayerAction {
    let view = seated_view(view, agent.profile());
    let noted = Noted::new(note.cloned().unwrap_or_default());
    match deck {
        Some(deck) => agent.act_with_own_deck(&view, pending, &noted.context(), deck),
        None => agent.act_with_context(&view, pending, &noted.context()),
    }
}
