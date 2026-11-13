//! What a search's played worlds teach: each candidate's fork advantage over
//! the policy's own answer, the choice the worlds make on each half of them,
//! and the item of a root's payable menu each candidate stands for
//! (`docs/trained-ai.md` §"Lookahead search").
//!
//! One definition for every reader: the tool that judges a run's search
//! (`fork_advantage4`), the converter that writes a student's labels
//! (`search_advantage_v1`), and the evaluator that plays the student. A
//! searched decision's `worlds[p][i]` is candidate `i`'s score in world `p`,
//! the same worlds for every candidate, candidate 0 the policy's own answer.

use baylee_engine::choice::{Pending, PlayerAction};

use crate::features4::{EncodedCandidates4, encode_executed_action4};

/// The mean advantage another answer has to show over the policy's own, on
/// the worlds it is chosen on, for the halves of a teacher run's labels: the
/// search's own `--search-margin` in its collection runs.
pub const HALVES_MARGIN: f64 = 0.03;

/// A count as a float, for means.
fn real(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// The mean advantage of `candidate` over the policy's answer (0) on
/// `worlds`; `0` for the policy's own answer and for no worlds.
pub fn advantage<W: AsRef<[f64]>>(worlds: &[W], candidate: usize) -> f64 {
    if worlds.is_empty() || candidate == 0 {
        return 0.0;
    }
    let sum: f64 = worlds
        .iter()
        .map(|world| world.as_ref()[candidate] - world.as_ref()[0])
        .sum();
    sum / real(worlds.len())
}

/// The mean advantage of `candidate` over every world and its paired
/// standard error, as the search's selection computes them
/// (`search/select.rs`: the variance over `n − 1`, then over `n` for the
/// mean); `(0, Some(0))` for the policy's own answer, and no standard error
/// with fewer than two worlds.
pub fn paired<W: AsRef<[f64]>>(worlds: &[W], candidate: usize) -> (f64, Option<f64>) {
    if candidate == 0 {
        return (0.0, Some(0.0));
    }
    let mean = advantage(worlds, candidate);
    if worlds.len() < 2 {
        return (mean, None);
    }
    let n = real(worlds.len());
    let variance = worlds
        .iter()
        .map(|world| (world.as_ref()[candidate] - world.as_ref()[0] - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0);
    (mean, Some((variance / n).sqrt()))
}

/// The candidate with the best mean advantage over the policy's answer on
/// `worlds`, if it clears `margin`; otherwise the policy's own (0). Ties go
/// to the earlier candidate.
pub fn choose<W: AsRef<[f64]>>(worlds: &[W], margin: f64) -> usize {
    let Some(k) = worlds.first().map(|world| world.as_ref().len()) else {
        return 0;
    };
    let mut chosen = 0;
    let mut best = margin;
    for candidate in 1..k {
        let advantage = advantage(worlds, candidate);
        if advantage > best {
            best = advantage;
            chosen = candidate;
        }
    }
    chosen
}

/// The choices the two halves of a decision's worlds make, each judged on
/// the other half: the even worlds (0, 2, …) and the odd ones.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Halves {
    /// The candidate chosen on the even worlds ([`choose`]).
    pub on_even: usize,
    /// The candidate chosen on the odd worlds.
    pub on_odd: usize,
    /// `on_even`'s mean advantage on the odd worlds.
    pub judged_even: f64,
    /// `on_odd`'s mean advantage on the even worlds.
    pub judged_odd: f64,
}

impl Halves {
    /// The lead of the two choices together: the mean of their judged
    /// advantages, `0` where both kept the policy's answer.
    #[must_use]
    pub fn lead(&self) -> f64 {
        f64::midpoint(self.judged_even, self.judged_odd)
    }
}

/// The choice on each half of `worlds` with `margin`, judged on the other.
pub fn halves<W: AsRef<[f64]>>(worlds: &[W], margin: f64) -> Halves {
    let even: Vec<&[f64]> = worlds.iter().step_by(2).map(AsRef::as_ref).collect();
    let odd: Vec<&[f64]> = worlds
        .iter()
        .skip(1)
        .step_by(2)
        .map(AsRef::as_ref)
        .collect();
    let on_even = choose(&even, margin);
    let on_odd = choose(&odd, margin);
    Halves {
        on_even,
        on_odd,
        judged_even: advantage(&odd, on_even),
        judged_odd: advantage(&even, on_odd),
    }
}

/// Why a search candidate has no item of a root's payable menu
/// ([`macro_items`]): the root is then left out, never guessed at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MacroItemError4 {
    /// No item takes the answer, exactly or in its group.
    Unmapped,
    /// Two candidates take the same item.
    SharedItem,
    /// A planned cast whose cast the payable menu did not add.
    NotPayable,
}

impl MacroItemError4 {
    /// A stable name for counts.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Unmapped => "unmapped",
            Self::SharedItem => "shared_item",
            Self::NotPayable => "not_payable",
        }
    }
}

/// The item of `cands` that one search candidate stands for. `cands` is a
/// root's payable menu (`netplay4::menu::PayableMenu4::extend`), its items
/// from `first_added` on the casts the planner pays for. A planned cast
/// (`planned`, the cast its taps pay for) is the item the menu added for
/// that cast; any other answer is the item the encoder records it as,
/// exactly or in its group (identical lands are one item,
/// [`encode_executed_action4`]).
///
/// # Errors
/// [`MacroItemError4::NotPayable`] for a planned cast the menu did not add,
/// [`MacroItemError4::Unmapped`] for any other answer no item takes.
pub fn macro_item(
    cands: &EncodedCandidates4,
    pending: &Pending,
    first_added: usize,
    action: &PlayerAction,
    planned: Option<&PlayerAction>,
) -> Result<usize, MacroItemError4> {
    if let Some(cast) = planned {
        return cands
            .candidates
            .iter()
            .enumerate()
            .skip(first_added)
            .find(|(_, cand)| cand.action.as_ref() == Some(cast))
            .map(|(at, _)| at)
            .ok_or(MacroItemError4::NotPayable);
    }
    encode_executed_action4(cands, pending, action)
        .map(|target| usize::from(target.single_idx))
        .filter(|at| *at < cands.candidates.len())
        .ok_or(MacroItemError4::Unmapped)
}

/// Every candidate's item ([`macro_item`]), in candidate order, the
/// policy's own first: `candidates` are each answer at the root with the
/// cast its taps pay for where it is a planned one.
///
/// # Errors
/// The first candidate without an item, or [`MacroItemError4::SharedItem`]
/// when two take the same one.
pub fn macro_items<'a>(
    cands: &EncodedCandidates4,
    pending: &Pending,
    first_added: usize,
    candidates: impl IntoIterator<Item = (&'a PlayerAction, Option<&'a PlayerAction>)>,
) -> Result<Vec<usize>, MacroItemError4> {
    let mut items: Vec<usize> = Vec::new();
    for (action, planned) in candidates {
        let item = macro_item(cands, pending, first_added, action, planned)?;
        if items.contains(&item) {
            return Err(MacroItemError4::SharedItem);
        }
        items.push(item);
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features4::{Candidate4, HeadKind4, PendingKind4};
    use baylee_core::ids::{ObjectId, PlayerId};
    use baylee_engine::choice::LegalActions;

    #[test]
    fn a_planned_cast_is_the_item_the_menu_added_and_the_rest_the_encoders() {
        let island = ObjectId::new(70, 0);
        let opt = ObjectId::new(61, 0);
        let bolt = ObjectId::new(62, 0);
        let item = |action: PlayerAction| Candidate4 {
            action: Some(action),
            ..Candidate4::default()
        };
        let pending = Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions {
                can_pass: true,
                ..LegalActions::default()
            }),
        };
        let cands = EncodedCandidates4 {
            head: HeadKind4::SingleChoice,
            pending_kind: PendingKind4::of(&pending),
            min_choices: 1,
            max_choices: 1,
            candidates: vec![
                item(PlayerAction::PassPriority),
                item(PlayerAction::ActivateManaAbility { source: island }),
                item(PlayerAction::CastSpell { card: bolt }),
                // Added by the payable menu: Opt, which needs a tap first.
                item(PlayerAction::CastSpell { card: opt }),
            ],
            pairs: Vec::new(),
            pair_objects: Vec::new(),
        };
        let tap = PlayerAction::ActivateManaAbility { source: island };
        let cast_opt = PlayerAction::CastSpell { card: opt };
        let cast_bolt = PlayerAction::CastSpell { card: bolt };
        assert_eq!(macro_item(&cands, &pending, 3, &tap, None), Ok(1));
        assert_eq!(
            macro_item(&cands, &pending, 3, &tap, Some(&cast_opt)),
            Ok(3),
            "a planned cast's first tap stands for its cast"
        );
        assert_eq!(
            macro_item(&cands, &pending, 3, &tap, Some(&cast_bolt)),
            Err(MacroItemError4::NotPayable),
            "castable as things stand is no added item"
        );
        assert_eq!(macro_item(&cands, &pending, 3, &cast_bolt, None), Ok(2));
        let land = PlayerAction::PlayLand {
            card: ObjectId::new(80, 0),
        };
        assert_eq!(
            macro_item(&cands, &pending, 3, &land, None),
            Err(MacroItemError4::Unmapped)
        );

        let root = [
            (&tap, None),
            (&PlayerAction::PassPriority, None),
            (&tap, Some(&cast_opt)),
            (&cast_bolt, None),
        ];
        assert_eq!(macro_items(&cands, &pending, 3, root), Ok(vec![1, 0, 3, 2]));
        assert_eq!(
            macro_items(&cands, &pending, 3, [(&tap, None), (&tap, None)]),
            Err(MacroItemError4::SharedItem)
        );
    }

    #[test]
    fn advantages_and_their_errors_are_the_selections() {
        let worlds = [[0.4, 0.5, 0.3], [0.6, 0.8, 0.5], [0.5, 0.5, 0.6]];
        assert!((advantage(&worlds, 1) - 0.1).abs() < 1e-12);
        assert_eq!(advantage(&worlds, 0), 0.0);
        let (mean, se) = paired(&worlds, 1);
        assert!((mean - 0.1).abs() < 1e-12);
        // Differences 0.1, 0.2, 0.0: variance 0.01, over three worlds.
        assert!((se.unwrap() - (0.01_f64 / 3.0).sqrt()).abs() < 1e-12);
        assert_eq!(paired(&worlds, 0), (0.0, Some(0.0)));
        assert_eq!(paired(&worlds[..1], 1).1, None);
    }

    #[test]
    fn each_half_chooses_and_is_judged_on_the_other() {
        // Ahead in the even worlds by 0.4, behind in the odd ones by 0.3.
        let worlds = [[0.3, 0.7], [0.6, 0.3], [0.3, 0.7], [0.6, 0.3]];
        let halves = halves(&worlds, HALVES_MARGIN);
        assert_eq!((halves.on_even, halves.on_odd), (1, 0));
        assert!((halves.judged_even + 0.3).abs() < 1e-12);
        assert_eq!(halves.judged_odd, 0.0);
        assert!((halves.lead() + 0.15).abs() < 1e-12);
        assert_eq!(choose(&[[0.50, 0.52]; 4], HALVES_MARGIN), 0);
        assert_eq!(choose(&[[0.50, 0.52]; 4], 0.01), 1);
    }
}
