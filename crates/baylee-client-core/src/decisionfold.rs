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
}
