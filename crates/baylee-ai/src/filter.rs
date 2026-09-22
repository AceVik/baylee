//! Reading a card's [`Filter`] against a [`PlayerView`], for the questions a
//! seat may ask about its own board.
//!
//! The engine answers these with `eval::matches`, which takes a `GameState` —
//! the reference this crate is never given, and the seam that makes an AI
//! seat and a networked player see the same game. So the agent needs its own
//! reader, and it is here rather than in `baylee-view` or `baylee-client-core`
//! because it has one caller and `eval::matches` is the definition it mirrors:
//! a second general copy is a second place for those semantics to drift.
//!
//! **The answer is three-valued and that is the whole design.** `Some(true)`
//! and `Some(false)` mean the view could see the answer; `None` means it
//! could not, and a caller must fall back to what it would have done without
//! asking. A reader that returned `false` for "cannot see" would produce a
//! decision that looks considered and is not — the same fault the honest-stub
//! rule exists to prevent one level up, where an unread clause refuses the
//! card instead of shipping a wrong one.
//!
//! Four variants are principled refusals rather than gaps to fill in later:
//!
//! - [`Filter::MatchesChosenTypeOfSource`] reads `chosen_subtype` off the
//!   source object. The view carries no such field for any object, so there
//!   is nothing to read rather than something awkward to reach.
//! - [`Filter::AttachedToBySource`] needs what the source is attached to
//!   *and* `GameState::ltb_attachments`, the record of what a permanent was
//!   wearing as it left the battlefield (CR 603.10a). The second half exists
//!   only in the engine, and answering from the first half alone would be
//!   right until the moment the rule is about.
//! - [`Filter::SharesSubtypeWithCommander`] asks about every commander a seat
//!   has, wherever it is. The view shows the command zone and the
//!   battlefield; a commander in a hidden zone is a count. Answering from the
//!   visible ones would be a different question that agrees most of the time.
//! - [`Filter::CmcAtMostX`] is bounded by the X announced for the ability's
//!   source (CR 107.3a), which the engine keeps on the source object and no
//!   view carries. Answering `true` would let an agent plan a tutor for a
//!   card the search may not legally find, which is exactly the
//!   considered-looking wrong decision above.
//!
//! [`Filter::IsToken`] is a fifth refusal, and only sometimes. The engine
//! asks `card.is_none()`, which in a view is three objects and not one: a
//! registry token, which says so through `token`; a permanent the seat may
//! not look at, which has no card because it is not entitled to one; and a
//! token that copies a card, which `resolve::tokens::create_token_copies`
//! leaves with **neither** field, because it inherits the original's empty
//! `token` and only its rules text is written back. The last two are
//! indistinguishable in a view and sit on opposite sides of the question, so
//! a token says `Some(true)`, a visible card says `Some(false)`, and the pair
//! that cannot be told apart says `None`.
//!
//! One reading deliberately differs, because the view is the better source:
//! the zone is passed in rather than read off the object, because a view
//! sorts objects into zone lists instead of stamping each one. Everything
//! else is the engine's reading word for word, including the two that look
//! like they might not be: `ControlledByOpponent` is `HeuristicAgent::hostile`
//! because that is this crate's `GameState::is_opponent` — team-aware, and
//! the same answer — and `HasKeyword` is has-*any* rather than has-all,
//! because `KeywordSet::contains` is an intersection test.
//!
//! The module holds the reader and its first caller, [`HeuristicAgent::cast_mode`],
//! for the same reason it is not a general crate: a reader with no caller has
//! nothing measuring it.

use baylee_cards_dsl::{AbilityDef, Effect, Filter, PlayerRel, SpellMode, ZoneRef};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::{CastModeDesc, CastModeKind};
use baylee_view::{ObjectStatus, PlayerView, PublicObject};

use crate::HeuristicAgent;

/// Three-valued `all`: one `false` settles it, otherwise an unknown wins.
fn all(parts: impl Iterator<Item = Option<bool>>) -> Option<bool> {
    let mut unknown = false;
    for part in parts {
        match part {
            Some(false) => return Some(false),
            None => unknown = true,
            Some(true) => {}
        }
    }
    (!unknown).then_some(true)
}

/// Three-valued `any`: one `true` settles it, otherwise an unknown wins.
fn any(parts: impl Iterator<Item = Option<bool>>) -> Option<bool> {
    let mut unknown = false;
    for part in parts {
        match part {
            Some(true) => return Some(true),
            None => unknown = true,
            Some(false) => {}
        }
    }
    (!unknown).then_some(false)
}

impl HeuristicAgent {
    /// Whether `object`, seen in `zone`, matches `filter`.
    ///
    /// `this` is the object the filter is written on — the source of the
    /// ability — and is `None` where the caller does not have one, which
    /// makes `This` and `Another` unreadable rather than false.
    pub(crate) fn filter_matches(
        &self,
        filter: &Filter,
        view: &PlayerView,
        object: &PublicObject,
        zone: ZoneRef,
        this: Option<ObjectId>,
    ) -> Option<bool> {
        let recur = |f: &Filter| self.filter_matches(f, view, object, zone, this);
        match filter {
            Filter::Any => Some(true),
            Filter::This => this.map(|id| object.id == id),
            Filter::Another => this.map(|id| object.id != id),
            Filter::And(parts) => all(parts.iter().map(recur)),
            Filter::Or(parts) => any(parts.iter().map(recur)),
            Filter::Not(inner) => recur(inner).map(|answer| !answer),
            Filter::HasType(t) => Some(object.types.intersects(*t)),
            Filter::LacksType(t) => Some(!object.types.intersects(*t)),
            Filter::HasSupertype(t) => Some(object.supertypes.contains(*t)),
            Filter::HasSubtype(s) => Some(object.subtypes.contains(*s)),
            Filter::HasColor(c) => Some(object.colors.intersects(*c)),
            Filter::IsColorless => Some(object.colors.is_colorless()),
            Filter::Monocolored => Some(object.colors.len() == 1),
            // Three cases, two of which the view cannot tell apart; the
            // header has the reason. `card` is asked second because a
            // registry token never carries one.
            Filter::IsToken => match (object.token, object.card) {
                (Some(_), _) => Some(true),
                (None, Some(_)) => Some(false),
                (None, None) => None,
            },
            Filter::ControlledByYou => Some(object.controller == view.seat),
            Filter::ControlledByOpponent => Some(self.hostile(object.controller, view.seat)),
            Filter::OwnedByYou => Some(object.owner == view.seat),
            Filter::Tapped => Some(object.status.contains(ObjectStatus::TAPPED)),
            Filter::Untapped => Some(!object.status.contains(ObjectStatus::TAPPED)),
            Filter::Attacking => Some(
                view.combat
                    .attackers
                    .iter()
                    .any(|attacker| attacker.creature == object.id),
            ),
            // `KeywordSet::contains` is an intersection test, so this is
            // has-*any* rather than has-all whatever a filter names.
            Filter::HasKeyword(k) => Some(object.keywords & k.bits() != 0),
            Filter::CmcAtMost(n) => Some(object.mana_value <= *n),
            Filter::CmcAtLeast(n) => Some(object.mana_value >= *n),
            Filter::ToughnessAtMost(n) => Some(object.toughness.is_some_and(|t| t <= *n)),
            Filter::InZone(want) => Some(zone == *want),
            // The four the view cannot answer. Named in this module's own
            // documentation with the reason each one is a refusal and not an
            // omission; a caller gets `None` and falls back.
            Filter::MatchesChosenTypeOfSource
            | Filter::AttachedToBySource
            | Filter::CmcAtMostX
            | Filter::SharesSubtypeWithCommander => None,
        }
    }

    /// Whether any permanent one of `controllers` controls matches `filter`.
    ///
    /// The battlefield and not a zone parameter, because all three callers
    /// below are printed sentences about permanents — "each opponent
    /// sacrifices a creature", "destroy all creatures".
    ///
    /// `None` keeps its meaning: it says the question could not be read of at
    /// least one candidate, so "nothing matched" is not established. A single
    /// readable match still answers `Some(true)`, because one is all the
    /// question asks for.
    fn battlefield_has(
        &self,
        filter: &Filter,
        view: &PlayerView,
        controllers: &[PlayerId],
        this: Option<ObjectId>,
    ) -> Option<bool> {
        any(view
            .battlefield
            .iter()
            .filter(|object| controllers.contains(&object.controller))
            .map(|object| self.filter_matches(filter, view, object, ZoneRef::Battlefield, this)))
    }

    /// Which cast option to take.
    ///
    /// The old answer was "the `Normal` option, else the first one", and for
    /// a card that has a normal cast it stays exactly that — overload and its
    /// friends print a mode that costs more than the card does, and choosing
    /// it because it is a mode would be a worse answer than the one this
    /// replaces. What is decided here is the case where choosing a mode is
    /// the only way to cast the spell (CR 700.2a) or the ability is a modal
    /// trigger (CR 603.3c): there is no `Normal` option, and the first
    /// printed mode was taken whatever the table looked like.
    ///
    /// The engine has already dropped every mode that cannot find its
    /// targets, so what is left to read is the mode that targets nothing and
    /// reaches nothing anyway — "each opponent sacrifices a creature" against
    /// opponents who control none. A mode that reaches something beats one
    /// that reaches nothing, a mode nobody could read sits between them, and
    /// the printed order breaks every tie, which is what keeps a table this
    /// crate cannot read answering exactly as it did before.
    pub(crate) fn cast_mode(
        &self,
        view: &PlayerView,
        object: ObjectId,
        options: &[CastModeDesc],
    ) -> usize {
        if let Some(normal) = options
            .iter()
            .position(|option| matches!(option.kind, CastModeKind::Normal))
        {
            return normal;
        }
        let Some(modes) = Self::modal_modes(view, object) else {
            return 0;
        };
        options
            .iter()
            .enumerate()
            .filter_map(|(position, option)| match option.kind {
                CastModeKind::Mode(mode) => Some((position, modes.get(mode)?)),
                _ => None,
            })
            .max_by_key(|(position, mode)| {
                let reach = match self.mode_reaches(view, mode, object) {
                    Some(true) => 2,
                    None => 1,
                    Some(false) => 0,
                };
                (reach, std::cmp::Reverse(*position))
            })
            .map_or(0, |(position, _)| position)
    }

    /// The printed modes the question is about, or `None` when they cannot be
    /// named from the view.
    ///
    /// The question names no ability. A modal trigger asks through the
    /// permanent and its mode number, and which of the permanent's abilities
    /// is on the stack lives in the engine's trigger queue — so a face with
    /// two modal abilities is two mode lists and one number.
    ///
    /// The rule is therefore not "at most one modal ability" but **at most
    /// one list**, which is what the pool actually prints: Derevi, Empyrial
    /// Tactician is one printed sentence with two trigger conditions, written
    /// as two `modal_triggered!` over the same `TAP_OR_UNTAP` modes, and
    /// naming that shared list is unambiguous however the trigger arrived.
    /// Two *different* lists is the ambiguity, it is refused, and
    /// `no_pool_face_states_two_mode_lists` (`baylee-gamehost`'s
    /// `tests/ai_coverage_guards.rs`) holds the pool to it.
    ///
    /// Which makes the refusal **unreachable rather than untested**, and it
    /// is worth saying so here because an injection sweep cannot tell the two
    /// apart: delete the `found.all(…)` comparison and the whole suite stays
    /// green, because no card reaches it and none can — the lookup goes
    /// through `baylee_cards::by_index`, so there is no synthetic card to
    /// build one with either. The day that lint goes red is the day this
    /// branch starts running, and the lint is the test that says so.
    fn modal_modes(view: &PlayerView, object: ObjectId) -> Option<&'static [SpellMode]> {
        let card = view
            .hand
            .iter()
            .find(|held| held.id == object)
            .map(|held| held.card)
            .or_else(|| view.object(object).and_then(|seen| seen.card))?;
        let mut found = baylee_cards::by_index(card.index)?
            .abilities_for_face(usize::from(card.face))
            .iter()
            .filter_map(|ability| match ability {
                AbilityDef::ModalSpell { modes } | AbilityDef::ModalTriggered { modes, .. } => {
                    Some(*modes)
                }
                _ => None,
            });
        let only = found.next()?;
        found.all(|other| std::ptr::eq(other, only)).then_some(only)
    }

    /// Whether a mode reaches anything at all.
    ///
    /// A mode that needs a target is never asked: the engine offers a modal
    /// option only once it has proven that mode's targets can be chosen —
    /// "if one of the modes would be illegal (due to an inability to choose
    /// legal targets, for example), that mode can't be chosen", CR 700.2a for
    /// a spell and CR 700.2b for a trigger — so re-deciding it
    /// from a view could only contradict an answer already given with the
    /// whole game state. "Up to one target" is offered with nothing to point
    /// at, so it falls through to the effects like an untargeted mode.
    fn mode_reaches(&self, view: &PlayerView, mode: &SpellMode, this: ObjectId) -> Option<bool> {
        if mode.targets.is_some_and(|req| req.min >= 1) {
            return Some(true);
        }
        self.effects_reach(view, mode.effects, this)
    }

    /// Which seats a [`PlayerRel`] names, from the view alone.
    ///
    /// `eval::players`' reading, including its `has_lost` filter — a seat
    /// that has lost controls nothing, so counting it would find an effect
    /// somewhere to land that it has nowhere to land. The two relations that
    /// resolve against a spell rather than against the table are `None` there
    /// and are `None` here: only a resolution knows who was chosen or who
    /// controls the target, and answering "nobody" reads exactly like "no
    /// seat matched".
    fn seats(&self, rel: PlayerRel, view: &PlayerView) -> Option<Vec<PlayerId>> {
        let every = || {
            view.seats
                .iter()
                .filter(|seat| !seat.has_lost)
                .map(|seat| seat.player)
        };
        Some(match rel {
            PlayerRel::You => vec![view.seat],
            PlayerRel::EachPlayer => every().collect(),
            // One opponent or all of them is the same question for
            // reachability: either way the effect has somewhere to land.
            PlayerRel::Opponent | PlayerRel::EachOpponent => {
                every().filter(|p| self.hostile(*p, view.seat)).collect()
            }
            PlayerRel::Chosen | PlayerRel::ControllerOfTarget => return None,
        })
    }

    /// Whether an untargeted effect list reaches anything on the board.
    ///
    /// The point of the question is a mode nobody can answer: "each opponent
    /// sacrifices a creature" against a table with no creature on it does
    /// nothing, and the printed order of the modes says nothing about which
    /// one does. `None` where the shape is not one of the three read here, so
    /// a caller keeps its old answer rather than acting on a silent no.
    fn effects_reach(&self, view: &PlayerView, effects: &[Effect], this: ObjectId) -> Option<bool> {
        let everyone: Vec<PlayerId> = view
            .seats
            .iter()
            .filter(|seat| !seat.has_lost)
            .map(|seat| seat.player)
            .collect();
        any(effects.iter().map(|effect| match effect {
            Effect::SacrificeFilter { who, filter }
            | Effect::DestroyChosenForPlayers { who, filter } => {
                self.battlefield_has(filter, view, &self.seats(*who, view)?, Some(this))
            }
            Effect::DestroyAll { filter } => {
                self.battlefield_has(filter, view, &everyone, Some(this))
            }
            _ => None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::{all, any};

    /// One `false` settles it — including after an unknown, which is the
    /// half a "have I seen an unknown yet" flag gets wrong. A reader that
    /// answered `false` for "cannot see" would produce a confident wrong
    /// decision instead of a fallback, so the unknown only wins where
    /// nothing else has.
    #[test]
    fn all_is_three_valued_and_a_false_outranks_an_unknown() {
        assert_eq!(all([].into_iter()), Some(true), "nothing to refuse it");
        assert_eq!(all([Some(true), Some(true)].into_iter()), Some(true));
        assert_eq!(all([Some(true), None].into_iter()), None);
        assert_eq!(
            all([None, Some(false)].into_iter()),
            Some(false),
            "the unknown is not latched past a later false"
        );
        assert_eq!(all([Some(false), None].into_iter()), Some(false));
        assert_eq!(all([None, None].into_iter()), None);
    }

    /// The mirror: one `true` settles it, and an unknown beats a list of
    /// falses because the view could not see whether one of them was the
    /// one that matched.
    #[test]
    fn any_is_three_valued_and_a_true_outranks_an_unknown() {
        assert_eq!(any([].into_iter()), Some(false), "nothing to satisfy it");
        assert_eq!(any([Some(false), Some(false)].into_iter()), Some(false));
        assert_eq!(any([Some(false), None].into_iter()), None);
        assert_eq!(any([None, Some(true)].into_iter()), Some(true));
        assert_eq!(any([Some(true), None].into_iter()), Some(true));
        assert_eq!(any([None, None].into_iter()), None);
    }

    /// Both settle on the spot rather than reading the rest, which is what
    /// makes them safe over an iterator whose later parts are expensive —
    /// every caller builds these from `filter_matches` on each member of a
    /// filter list. An iterator that panics after the deciding element
    /// proves it without asserting on a count nobody keeps.
    #[test]
    fn a_settled_answer_reads_no_further() {
        let decided = [Some(false)]
            .into_iter()
            .chain(std::iter::from_fn(|| panic!("read past the answer")));
        assert_eq!(all(decided), Some(false));

        let decided = [Some(true)]
            .into_iter()
            .chain(std::iter::from_fn(|| panic!("read past the answer")));
        assert_eq!(any(decided), Some(true));
    }
}
