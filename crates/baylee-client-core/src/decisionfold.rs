//! Whether the question's sheet is folded down to its pill (the owner,
//! 08.10.2026: *"Make it minimisable … so the table underneath is fully
//! visible and clickable for picking targets; clicking the pill or the key
//! restores it; it never auto-dismisses the question."*).
//!
//! The fold belongs to **one question**: it remembers the snapshot it was
//! folded on (the view's `seq`, which stands still while a seat decides and
//! moves on with the answer), so the next question opens unfolded by itself
//! and a fold can never hide a question nobody folded. Folding answers
//! nothing — the question stands, the table answers it, the pill brings the
//! sheet back.

/// What the drawer's reading asks to stand over the shelf right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asked {
    /// Nothing: no question of this seat's, nothing to say.
    Nothing,
    /// The open sheet (or any drawer panel).
    Sheet,
    /// The question's sheet folded to its pill.
    Pill,
}

/// One thing hanging from the drawer's root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hung {
    /// An open panel.
    Sheet,
    /// A panel on its way out.
    Leaving,
    /// A folded sheet's pill.
    Pill,
}

/// What becomes of one hung thing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fate {
    /// Left as it is: a panel finishing its way out.
    Keep,
    /// The one that stands for the reading: kept, its contents rewritten.
    Refill,
    /// An open panel sent on its way out.
    SendAway,
    /// Gone at once.
    Despawn,
}

/// What the drawer does to agree with its reading: one [`Fate`] per hung
/// thing, in order, and whether the asked thing has to be spawned afresh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// Each hung thing's fate, in the order given.
    pub fates: Vec<Fate>,
    /// Whether nothing hung can stand for the reading, so one is spawned.
    pub spawn: bool,
}

impl Plan {
    /// Whether the tree already agrees with the reading: nothing to spawn,
    /// send away or despawn. A refill alone is settled — whether its
    /// contents are stale is the revision's question, not the tree's.
    #[must_use]
    pub fn settled(&self) -> bool {
        !self.spawn
            && self
                .fates
                .iter()
                .all(|fate| matches!(fate, Fate::Keep | Fate::Refill))
    }
}

/// Reconciles what hangs from the drawer's root with what its reading asks.
///
/// **Every hung thing is decided from the reading alone**, never from the
/// event that changed it: a question answered by a press, a key, the table
/// under a folded pill, the opponent's response, the stack resolving, a
/// resync or a policy all arrive here as the same reading, and whatever hung
/// for the last question either stands for this one or goes. That is the
/// whole fix for the beta.6 reports where a resolved Path to Exile's and
/// Swords to Plowshares's target pill stayed on the table: the pill was
/// judged by the next reading's fold, which an empty reading never has, so
/// nothing ever sent it away.
///
/// - Asked nothing: an open panel is sent on its way out, a leaving one
///   finishes, a pill (which has no way out to run) goes now.
/// - Asked the sheet: the first open panel is refilled; everything else
///   (a pill, a leaving panel that cannot be turned round, a second panel)
///   goes, and a fresh panel opens when none stood.
/// - Asked the pill: the first pill is refilled — it is rewritten, never
///   left emptied — and everything else goes.
#[must_use]
pub fn reconcile(asked: Asked, hung: &[Hung]) -> Plan {
    let stands_for = match asked {
        Asked::Nothing => None,
        Asked::Sheet => Some(Hung::Sheet),
        Asked::Pill => Some(Hung::Pill),
    };
    let mut kept = false;
    let fates = hung
        .iter()
        .map(|&thing| match (stands_for, thing) {
            (None, Hung::Sheet) => Fate::SendAway,
            (None, Hung::Leaving) => Fate::Keep,
            (Some(wanted), thing) if wanted == thing && !kept => {
                kept = true;
                Fate::Refill
            }
            _ => Fate::Despawn,
        })
        .collect();
    Plan {
        fates,
        spawn: stands_for.is_some() && !kept,
    }
}

/// The fold, keyed by the snapshot it was made on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DecisionFold {
    folded: Option<u64>,
}

impl DecisionFold {
    /// Whether the sheet asked at snapshot `seq` is folded.
    #[must_use]
    pub fn is_folded(&self, seq: Option<u64>) -> bool {
        seq.is_some() && self.folded == seq
    }

    /// Folds the sheet asked at `seq`, or unfolds it if it is folded. With no
    /// question (`None`) the fold stays as it was.
    pub fn toggle(&mut self, seq: Option<u64>) {
        if seq.is_none() {
            return;
        }
        self.folded = if self.is_folded(seq) { None } else { seq };
    }

    /// Unfolds, whatever was folded.
    pub fn open(&mut self) {
        self.folded = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fold_is_the_questions_and_the_next_question_opens() {
        let mut fold = DecisionFold::default();
        assert!(!fold.is_folded(Some(4)));
        fold.toggle(Some(4));
        assert!(
            fold.is_folded(Some(4)),
            "folded on the question it was made on"
        );
        assert!(
            !fold.is_folded(Some(5)),
            "the game moved on: the next question is not hidden by an old fold"
        );
        fold.toggle(Some(4));
        assert!(!fold.is_folded(Some(4)), "the same key restores it");
        fold.toggle(None);
        assert!(!fold.is_folded(None), "nothing asked, nothing folded");
    }

    /// The beta.6 reports: a target question folded to its pill, answered
    /// from the table under it, and the spell resolved — the pill stood on
    /// for turns. Whatever ends the question, the reading is then empty, and
    /// an empty reading takes the pill with it.
    #[test]
    fn a_pill_goes_when_its_question_does() {
        let plan = reconcile(Asked::Nothing, &[Hung::Pill]);
        assert_eq!(plan.fates, [Fate::Despawn], "the pill goes now");
        assert!(!plan.spawn);
        assert!(!plan.settled(), "a pill under no question is not settled");
        assert!(reconcile(Asked::Nothing, &[]).settled());
    }

    /// An open sheet whose question ended runs its way out; one already
    /// leaving is let finish rather than cut short.
    #[test]
    fn an_open_sheet_leaves_and_a_leaving_one_finishes() {
        let plan = reconcile(Asked::Nothing, &[Hung::Sheet]);
        assert_eq!(plan.fates, [Fate::SendAway]);
        let plan = reconcile(Asked::Nothing, &[Hung::Leaving]);
        assert_eq!(plan.fates, [Fate::Keep]);
        assert!(plan.settled(), "nothing more to do while it leaves");
    }

    /// A folded pill whose reading changes (the art arrived, the title was
    /// translated, the shelf's centre moved) is rewritten in place: it was
    /// emptied of its words and picture and left a bare blob.
    #[test]
    fn a_pill_whose_reading_changed_is_rewritten_not_emptied() {
        let plan = reconcile(Asked::Pill, &[Hung::Pill]);
        assert_eq!(plan.fates, [Fate::Refill]);
        assert!(!plan.spawn, "the same pill stands, no second arrival");
        assert!(plan.settled());
    }

    /// Folding and opening swap the one for the other, and nothing of the
    /// other stays to take a press.
    #[test]
    fn a_fold_swaps_the_sheet_for_the_pill_and_back() {
        let plan = reconcile(Asked::Pill, &[Hung::Sheet]);
        assert_eq!(plan.fates, [Fate::Despawn]);
        assert!(plan.spawn);
        let plan = reconcile(Asked::Sheet, &[Hung::Pill]);
        assert_eq!(plan.fates, [Fate::Despawn]);
        assert!(plan.spawn);
    }

    /// A new question while the last one's panel is still leaving opens a
    /// fresh panel: a leaving one cannot be turned round. Of two of a kind
    /// only the first stands.
    #[test]
    fn exactly_one_thing_stands_for_a_question() {
        let plan = reconcile(Asked::Sheet, &[Hung::Leaving]);
        assert_eq!(plan.fates, [Fate::Despawn]);
        assert!(plan.spawn);
        let plan = reconcile(Asked::Sheet, &[Hung::Sheet, Hung::Pill, Hung::Sheet]);
        assert_eq!(plan.fates, [Fate::Refill, Fate::Despawn, Fate::Despawn]);
        assert!(!plan.spawn);
        assert!(!plan.settled(), "the extras still have to go");
        let plan = reconcile(Asked::Sheet, &[]);
        assert!(plan.spawn && !plan.settled(), "a fresh root is filled");
    }
}
