//! The protocol-44 boundary: shadow labels written before it.
//!
//! From protocol 44 an offer names a suspend whose mana is still in the lands
//! (`LegalActions::unpaid_suspend`, CR 116.2f). A shadow label keeps the
//! SHA-256 of the question's JSON and a hash of the view and the question, so
//! a label written before, rebuilt on a later build at a position where the
//! offer names one, no longer matches. It moves neither `snapshot_hash` nor
//! an answer the engine takes, and neither the house nor an encoder reads it:
//! the records replay, and only these bindings differ. The conversion names
//! the boundary instead of a mismatch, and stops, because such a label is
//! converted by the runtime that wrote it (`docs/engine-internals.md`
//! §"The protocol-44 boundary"). Protocol 44's other two fields, a division
//! on the stack (`PlayerView::divided`) and a log line's step, are in no
//! observation a label binds.

use baylee_engine::choice::Pending;

/// How a refusal of a label from before protocol 44 begins.
pub const PROTOCOL44_REFUSAL: &str = "shadow label written before protocol 44:";

/// The rule a refusal names.
pub const PROTOCOL44_RULE: &str =
    "convert shadow labels written before protocol 44 with the runtime that wrote them";

/// `pending` as a build from before protocol 44 wrote it: the offer without
/// `unpaid_suspend`. `None` when the offer names none, which is nearly every
/// position.
pub(super) fn as_written_before(pending: &Pending) -> Option<Pending> {
    let Pending::Priority { player, legal } = pending else {
        return None;
    };
    if legal.unpaid_suspend.is_empty() {
        return None;
    }
    let mut before = (**legal).clone();
    before.unpaid_suspend.clear();
    Some(Pending::Priority {
        player: *player,
        legal: Box::new(before),
    })
}

/// Refuses, by name, a label `bound` does not bind to `pending` and does bind
/// to the question as a build before protocol 44 wrote it: one that build
/// wrote.
///
/// # Errors
/// That refusal, or what `bound` could not read.
pub(super) fn refuse_if_written_before(
    pending: &Pending,
    game: &str,
    n: u64,
    bound: &dyn Fn(&Pending) -> anyhow::Result<bool>,
) -> anyhow::Result<()> {
    if bound(pending)? {
        return Ok(());
    }
    if let Some(earlier) = as_written_before(pending)
        && bound(&earlier)?
    {
        anyhow::bail!(refusal(
            game,
            n,
            "the offer names an unpaid suspend it did not"
        ));
    }
    Ok(())
}

/// The refusal for a label of `game` at input `n`, naming what moved.
pub(super) fn refusal(game: &str, n: u64, what: &str) -> String {
    format!("{PROTOCOL44_REFUSAL} game {game}, input {n}: {what}; {PROTOCOL44_RULE}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::{ObjectId, PlayerId};
    use baylee_core::mana::ManaCost;
    use baylee_engine::choice::LegalActions;

    fn priority(legal: LegalActions) -> Pending {
        Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(legal),
        }
    }

    fn suspend() -> Pending {
        priority(LegalActions {
            can_pass: true,
            unpaid_suspend: vec![(ObjectId::new(61, 0), ManaCost::parse("{U}"))],
            ..LegalActions::default()
        })
    }

    /// An offer naming an unpaid suspend reads as protocol 43 wrote it
    /// without it; an offer naming none is no boundary's.
    #[test]
    fn the_question_as_a_build_before_protocol_44_wrote_it() {
        let Some(Pending::Priority { legal, .. }) = as_written_before(&suspend()) else {
            panic!("a priority");
        };
        assert!(legal.unpaid_suspend.is_empty());
        assert!(legal.can_pass);
        assert!(as_written_before(&priority(LegalActions::default())).is_none());
    }

    /// A label bound to the earlier JSON is refused by name; one bound to the
    /// question as it is passes; one bound to neither is left to the
    /// mismatch it is.
    #[test]
    fn a_label_from_before_is_refused_by_name_and_no_other() {
        let json = |pending: &Pending| serde_json::to_string(pending).expect("json");
        let question = suspend();
        let before = json(&as_written_before(&question).expect("named"));
        let written_before = |p: &Pending| Ok(json(p) == before);
        let said = refuse_if_written_before(&question, "run/game-0030", 322, &written_before)
            .expect_err("refused")
            .to_string();
        assert!(said.starts_with(PROTOCOL44_REFUSAL), "{said}");
        assert!(said.ends_with(PROTOCOL44_RULE), "{said}");
        let now = json(&question);
        let written_now = |p: &Pending| Ok(json(p) == now);
        assert!(refuse_if_written_before(&question, "g", 1, &written_now).is_ok());
        let neither = |_: &Pending| Ok(false);
        assert!(refuse_if_written_before(&question, "g", 1, &neither).is_ok());
    }
}
