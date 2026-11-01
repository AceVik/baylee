//! A decision taken up to noting its answer, with the policy's ranking of
//! the legal answers it chose among: what a lookahead compares.
use super::{Answer4, Pending, PlayerAction, PlayerView, TeacherTarget4, decode_action4};
use crate::features4::{EncodedCandidates4, HeadKind4};

/// One legal answer of a single-choice decision, as the policy scored it.
#[derive(Clone, Debug, PartialEq)]
pub struct Ranked4 {
    /// Its candidate's index in the decision's encoded menu.
    pub index: usize,
    /// The legal action the candidate decodes to.
    pub action: PlayerAction,
    /// The policy's raw logit for the candidate.
    pub logit: f32,
}

/// A decision taken as [`NetPlayer4::act_detailed`] takes it, up to noting
/// its answer.
///
/// [`NetPlayer4::act_detailed`]: super::NetPlayer4::act_detailed
#[derive(Clone, Debug)]
pub struct RootDecision4 {
    /// The answer `act_detailed` gives.
    pub answer: Answer4,
    /// When asked for and the answer is the policy's own single choice (not
    /// forced, unsupported, guarded or a fallback): every legal answer the
    /// policy chose among, best first ([`rank_single_choice`]).
    pub ranking: Option<Vec<Ranked4>>,
    /// Whether finishing the decision notes its answer as the seat's
    /// announced spell or ability, as `act_detailed` does after inference.
    pub notes_answer: bool,
}

/// Every candidate of a single-choice menu that decodes to a legal answer
/// of `pending`, by the policy's raw logit, best first, ties in menu order.
///
/// A candidate stands for an equivalence group and decodes to one member;
/// a later candidate decoding to an action already ranked is dropped. Empty
/// for any other head.
#[must_use]
pub fn rank_single_choice(
    cands: &EncodedCandidates4,
    logits: &[f32],
    view: &PlayerView,
    pending: &Pending,
) -> Vec<Ranked4> {
    if cands.head != HeadKind4::SingleChoice {
        return Vec::new();
    }
    let mut ranked: Vec<Ranked4> = (0..cands.candidates.len())
        .filter_map(|index| {
            let target = TeacherTarget4 {
                single_idx: u16::try_from(index).ok()?,
                ..TeacherTarget4::default()
            };
            let action = decode_action4(cands, view, pending, &target)?;
            pending.answer_fault(&action).is_none().then(|| Ranked4 {
                index,
                action,
                logit: logits.get(index).copied().unwrap_or(f32::NEG_INFINITY),
            })
        })
        .collect();
    ranked.sort_by(|a, b| b.logit.total_cmp(&a.logit).then(a.index.cmp(&b.index)));
    let mut kept: Vec<Ranked4> = Vec::with_capacity(ranked.len());
    for candidate in ranked {
        if !kept.iter().any(|k| k.action == candidate.action) {
            kept.push(candidate);
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PlayerId;
    use baylee_core::preset::AIProfile;

    /// The first priority of seat 0 at a fresh table, encoded.
    fn first_priority() -> (Pending, PlayerView, EncodedCandidates4) {
        let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
        let preset = crate::selfplay::table_for(8, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
        let mut session = baylee_gamehost::Session::new(&preset).unwrap();
        let me = PlayerId::new(0);
        session.take_over(me);
        for _ in 0..200 {
            if session.needs_pump() {
                session.pump_at_most(1);
                continue;
            }
            let (pending, view) = session.view_for_with_context(me).unwrap();
            if matches!(pending, Pending::Priority { .. }) {
                let state = crate::features7::encode_state7(&view, Some(&pending), None, &[]);
                let cands = crate::features7::encode_candidates7_grouped(&state, &view, &pending);
                if cands.candidates.len() > 1 {
                    return (pending, view, cands);
                }
            }
            let action = session.house_action(me).unwrap();
            session.act_slice(me, action, 0).unwrap();
        }
        panic!("no priority with a choice");
    }

    #[test]
    fn ranking_is_legal_best_first_and_ties_keep_menu_order() {
        let (pending, view, cands) = first_priority();
        let n = cands.candidates.len();
        let rising: Vec<f32> = (0..n)
            .map(|i| f32::from(u16::try_from(i).unwrap()))
            .collect();
        let ranked = rank_single_choice(&cands, &rising, &view, &pending);
        assert!(!ranked.is_empty());
        assert!(
            ranked
                .iter()
                .all(|r| pending.answer_fault(&r.action).is_none())
        );
        assert!(ranked.windows(2).all(|w| w[0].logit >= w[1].logit));
        assert_eq!(ranked[0].index, n - 1);

        let flat = vec![0.0; n];
        let tied = rank_single_choice(&cands, &flat, &view, &pending);
        assert!(tied.windows(2).all(|w| w[0].index < w[1].index));
        for (i, a) in tied.iter().enumerate() {
            assert!(tied[i + 1..].iter().all(|b| b.action != a.action));
        }
    }
}
