//! The offer boundary (commit e1d85f5d0, with protocol 43 and 1e29094fb on
//! the same side): shadow labels written before it.
//!
//! From e1d85f5d0 on, an offer names the ways of casting a card that lies
//! outside the hand and the command zone (`LegalActions::cast_ways`), and from
//! protocol 43 what restricted mana may pay for (`restricted_taps`,
//! `restricted_floating`). A shadow label keeps the SHA-256 of the question's
//! JSON and a hash of the view and the question, so a label written before
//! either, rebuilt on a later build at a position where the offer names one,
//! no longer matches. From 1e29094fb the planner reads each `{2/C}` of a cost
//! on its own (CR 107.4e), so the house teacher asked again can answer
//! otherwise where a mix pays a card such a cost prints. None of it moves
//! `snapshot_hash` or an answer the engine takes: the records replay, and
//! only these bindings differ. The conversion names the boundary instead of a
//! mismatch, and stops, because such a label is converted by the runtime that
//! wrote it (`docs/engine-internals.md` §"The offer boundary").

use baylee_core::mana::ManaSymbol;
use baylee_engine::choice::Pending;
use baylee_view::PlayerView;

/// The commit from which an offer names the ways of a card cast from a
/// graveyard or from exile.
pub const OFFER_BOUNDARY: &str = "e1d85f5d0";

/// How a refusal of a label from before [`OFFER_BOUNDARY`] begins.
pub const OFFER_REFUSAL: &str = "shadow label written before offer boundary e1d85f5d0:";

/// The rule a refusal names.
pub const OFFER_RULE: &str =
    "convert shadow labels written before e1d85f5d0 with the runtime that wrote them";

/// `pending` as a build from before the boundary could have written it, for
/// each step of the boundary it may have been written before: without what
/// protocol 43 states, and also without the ways of e1d85f5d0. Empty when the
/// offer names none of it, which is most positions.
pub(super) fn as_written_before(view: &PlayerView, pending: &Pending) -> Vec<Pending> {
    let Pending::Priority { player, legal } = pending else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut before_43 = (**legal).clone();
    before_43.restricted_taps.clear();
    before_43.restricted_floating.clear();
    if before_43.restricted_taps != legal.restricted_taps
        || before_43.restricted_floating != legal.restricted_floating
    {
        out.push(Pending::Priority {
            player: *player,
            legal: Box::new(before_43.clone()),
        });
    }
    let mut before_ways = before_43;
    before_ways
        .cast_ways
        .retain(|(card, _)| named_before(view, *card));
    if before_ways.cast_ways != legal.cast_ways {
        out.push(Pending::Priority {
            player: *player,
            legal: Box::new(before_ways),
        });
    }
    out
}

/// Whether a build before e1d85f5d0 named the ways of `card`: one in the
/// seat's hand or command zone whose card prints a priced way
/// (`casting::has_priced_ways`), which is the only kind it named.
fn named_before(view: &PlayerView, card: baylee_core::ids::ObjectId) -> bool {
    let in_hand = held(view)
        .find(|held| held.id == card)
        .map(|held| held.card);
    let in_command = view
        .command
        .iter()
        .flatten()
        .find(|object| object.id == card)
        .and_then(|object| object.card);
    in_hand
        .or(in_command)
        .and_then(|identity| baylee_cards::by_index(identity.index))
        .is_some_and(baylee_engine::casting::has_priced_ways)
}

/// Whether the seat could cast a card whose cost prints two or more `{2/C}`
/// symbols: a position where the house teacher's answer may have moved at
/// 1e29094fb.
pub(super) fn reads_several_twobrids(view: &PlayerView, pending: &Pending) -> bool {
    let Pending::Priority { legal, .. } = pending else {
        return false;
    };
    legal
        .castable
        .iter()
        .chain(&legal.payable)
        .filter_map(|card| {
            held(view)
                .find(|held| held.id == *card)
                .map(|held| held.card)
                .or_else(|| view.object(*card).and_then(|object| object.card))
        })
        .filter_map(|identity| baylee_cards::by_index(identity.index))
        .any(|def| {
            def.faces.iter().any(|face| {
                face.mana_cost
                    .symbols()
                    .filter(|s| matches!(s, ManaSymbol::TwoOrColor(_)))
                    .count()
                    >= 2
            })
        })
}

/// Refuses, by name, a label `bound` does not bind to `pending` and does bind
/// to the question as a build before the boundary wrote it: one that build
/// wrote.
///
/// # Errors
/// That refusal, or what `bound` could not read.
pub(super) fn refuse_if_written_before(
    view: &PlayerView,
    pending: &Pending,
    game: &str,
    n: u64,
    bound: &dyn Fn(&Pending) -> anyhow::Result<bool>,
) -> anyhow::Result<()> {
    if bound(pending)? {
        return Ok(());
    }
    for earlier in as_written_before(view, pending) {
        if bound(&earlier)? {
            anyhow::bail!(refusal(
                game,
                n,
                "the offer names ways or restricted mana it did not"
            ));
        }
    }
    Ok(())
}

/// The refusal for a label of `game` at input `n`, naming what moved.
pub(super) fn refusal(game: &str, n: u64, what: &str) -> String {
    format!("{OFFER_REFUSAL} game {game}, input {n}: {what}; {OFFER_RULE}")
}

/// The cards this seat holds as the view shows them: its hand, and a hand it
/// decides for (CR 724).
fn held(view: &PlayerView) -> impl Iterator<Item = &baylee_view::HandObject> {
    view.hand
        .iter()
        .chain(view.controlled_hands.iter().flat_map(|h| &h.cards))
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::{ViewBuilder, token};
    use baylee_core::ids::{ObjectId, PlayerId, PrintRef};
    use baylee_engine::choice::{CastModeDesc, CastModeKind, LegalActions, RestrictedTaps, Spend};
    use baylee_view::{CardIdentity, HandObject};

    fn identity(name: &str) -> CardIdentity {
        CardIdentity {
            index: baylee_cards::decks::by_name(name).expect(name),
            print: PrintRef::new(0),
            face: 0,
        }
    }

    fn in_hand(slot: u32, name: &str) -> HandObject {
        HandObject {
            id: ObjectId::new(slot, 0),
            card: identity(name),
            name: name.to_owned(),
            mana_value: 0,
            colors: baylee_core::color::ColorSet::EMPTY,
            types: baylee_core::types::TypeSet::EMPTY,
            commander: false,
        }
    }

    fn way(kind: CastModeKind) -> CastModeDesc {
        CastModeDesc {
            index: 0,
            kind,
            cost: baylee_core::mana::ManaCost::parse("{1}{U}"),
        }
    }

    fn priority(legal: LegalActions) -> Pending {
        Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(legal),
        }
    }

    /// An offer naming restricted mana reads as protocol 42 wrote it without
    /// the two lists; one naming a graveyard card's way reads as e1d85f5d0's
    /// parent wrote it without that way, and an adventure in hand keeps its
    /// own, which that build named too.
    #[test]
    fn the_question_as_a_build_before_the_boundary_wrote_it() {
        let mut buried = token(60, 0, "Malevolent Hermit", 2, 1);
        buried.card = Some(identity("Malevolent Hermit"));
        let mut view = ViewBuilder::new(2).with_graveyard(0, vec![buried]).build();
        view.hand.push(in_hand(61, "Brazen Borrower"));
        let hermit = ObjectId::new(60, 0);
        let borrower = ObjectId::new(61, 0);
        let legal = LegalActions {
            payable: vec![hermit, borrower],
            cast_ways: vec![
                (hermit, way(CastModeKind::Face(1))),
                (borrower, way(CastModeKind::Normal)),
                (borrower, way(CastModeKind::Face(1))),
            ],
            restricted_taps: vec![RestrictedTaps {
                taps: vec![(ObjectId::new(1, 0), 0)],
                spends: vec![Spend::Cast(borrower, CastModeKind::Normal)],
            }],
            ..LegalActions::default()
        };
        let earlier = as_written_before(&view, &priority(legal));
        assert_eq!(earlier.len(), 2);
        let Pending::Priority {
            legal: before_43, ..
        } = &earlier[0]
        else {
            panic!("a priority");
        };
        assert!(before_43.restricted_taps.is_empty());
        assert_eq!(before_43.cast_ways.len(), 3);
        let Pending::Priority { legal: before, .. } = &earlier[1] else {
            panic!("a priority");
        };
        assert_eq!(
            before
                .cast_ways
                .iter()
                .map(|(id, _)| *id)
                .collect::<Vec<_>>(),
            vec![borrower, borrower],
            "the Hermit's disturb way is the boundary's, the Borrower's are not"
        );
        let plain = LegalActions {
            payable: vec![borrower],
            cast_ways: vec![(borrower, way(CastModeKind::Face(1)))],
            ..LegalActions::default()
        };
        assert!(as_written_before(&view, &priority(plain)).is_empty());
    }

    #[test]
    fn a_cost_of_several_twobrids_is_where_the_house_may_have_moved() {
        let mut view = ViewBuilder::new(2).build();
        view.hand.push(in_hand(70, "Temur Tawnyback"));
        view.hand.push(in_hand(71, "Grizzly Bears"));
        let tawnyback = LegalActions {
            payable: vec![ObjectId::new(70, 0)],
            ..LegalActions::default()
        };
        assert!(reads_several_twobrids(&view, &priority(tawnyback)));
        let bears = LegalActions {
            payable: vec![ObjectId::new(71, 0)],
            ..LegalActions::default()
        };
        assert!(!reads_several_twobrids(&view, &priority(bears)));
    }

    #[test]
    fn a_refusal_names_the_boundary_and_the_rule() {
        let said = refusal("run/game-0016", 85, "what moved");
        assert!(said.starts_with(OFFER_REFUSAL), "{said}");
        assert!(said.ends_with(OFFER_RULE), "{said}");
        assert!(said.contains(OFFER_BOUNDARY));
    }
}
