//! Whether a spell in hand has anything to point at.
//!
//! The engine answers this properly — a spell with no legal target is not
//! offered in `LegalActions::castable` at all (CR 601.2c) — and that answer
//! is made with the mana already floating. [`crate::reachable`] is the other
//! half of the client's hand and asks the question a turn earlier, before any
//! land is tapped, which is exactly where the engine has nothing to say.
//!
//! It had nothing to say here either, and the hazard is named in `reachable`'s
//! own doc comment: *taking the offer taps the lands **first**, and the engine
//! then refuses the cast, which spends the turn's mana on nothing.* So a
//! Swords to Plowshares whose only creature had been exiled lit up, took the
//! click, tapped the lands, and left the card in hand with the mana gone.
//!
//! # Prove the absence, never assume it
//!
//! **Withhold the offer only when this client can positively prove that no
//! legal target exists; offer whenever it cannot tell.** That asymmetry is
//! the whole design and is easy to "simplify" away into a symmetric check, so
//! it is written here as a sentence rather than left in the shape of the code.
//!
//! [`crate::castmodes::parts_payable`] is the nearest neighbour and takes the
//! **opposite** default — anything it cannot answer from the view is refused
//! — and the difference is the consequence rather than a preference. A
//! refused alternative cost costs nothing: the card falls back to the price
//! printed on it. A refused cast darkens the card, and a card that is dark
//! for a reason the player cannot see is the whole of the fault this client
//! was already reported for once. Trading that for this one would be a poor
//! bargain, so an unreadable filter here means *offer it* and let the engine
//! be the authority it already is.
//!
//! # What it can prove
//!
//! Measured over the 208 spells in this pool that require at least one
//! target: 189 carry a filter every arm of [`matches`] can read, and the
//! other 19 can never be targetless at all — `AnyTarget`, `AnyOpponent` and
//! `Player` always have a player to point at, so the proof is refused for
//! them by construction rather than by a gap. The two together are the whole
//! population, which is why this is worth its code: the reported card
//! (`Object`, 152 of them) and a counterspell held with an empty stack
//! (`Spell`, 25) are both inside it.
//!
//! # Why it is here and not in `baylee-client-core`
//!
//! The same seam [`crate::manasources`] sits on. Reading a `TargetSpec` takes
//! the compiled card registry and a `Filter` takes `baylee-cards-dsl`, and
//! `baylee-client-core` deliberately links neither.

use baylee_cards_dsl::{AbilityDef, Filter, TargetSpec};
use baylee_view::{HandObject, PlayerView, PublicObject, StackItem};

/// Whether this client can **prove** the card has no legal target.
///
/// `false` is the answer for everything it cannot read, which includes every
/// card that needs no target at all — so a caller may ask it about any card
/// in hand without first deciding whether the question applies.
#[must_use]
pub fn provably_targetless(view: &PlayerView, hand: &HandObject) -> bool {
    let Some(def) = baylee_cards::by_index(hand.card.index) else {
        return false;
    };
    // The face that is up, the same one `manasources::hand_cost` prices: an
    // adventure or an MDFC in hand is cast as what it is showing, and the
    // other face's targets are a different card's.
    let abilities = def.abilities_for_face(hand.card.face as usize);
    let mut asked = false;
    for ability in abilities {
        // `ModalSpell` is deliberately not here. A modal spell is castable if
        // *any* mode is, so proving it targetless means proving every mode
        // targetless, and one unreadable mode would have to abandon the whole
        // proof — the honest answer until a card in this pool needs it.
        let AbilityDef::Spell {
            targets: Some(req), ..
        } = ability
        else {
            continue;
        };
        // "Up to one target" is cast with none (CR 601.2c), so a spell that
        // may decline is never the card this is looking for.
        if req.min == 0 {
            continue;
        }
        asked = true;
        match legal_targets(view, &req.spec) {
            // Not enough distinct objects to fill the minimum, which is the
            // count CR 601.2c holds the cast to rather than the maximum.
            Some(found) if found < usize::from(req.min) => {}
            _ => return false,
        }
    }
    asked
}

/// How many legal targets this spec has, or `None` where the view cannot say.
///
/// The player specs answer `None` and not a number, which reads oddly and is
/// the point: a player is always there, so a spell pointing at one can never
/// be the targetless case and the proof is refused rather than computed.
fn legal_targets(view: &PlayerView, spec: &TargetSpec) -> Option<usize> {
    match spec {
        // "Battlefield, or stack for spells" — and the battlefield is the
        // half a spell in hand is pointed at. A permanent spell already on
        // the stack is somebody else's problem and is counted by `Spell`.
        TargetSpec::Object(filter) => count(view, view.battlefield.iter(), filter),
        // The stack holds both kinds and these three specs want different
        // halves of it, which `PublicObject::stack_item` is the field to ask.
        // Counting the whole stack for all three would over-count and so err
        // in the safe direction — but "a counterspell has a target" and
        // "there is an ability on the stack" are different sentences, and a
        // reading that conflates them is one nobody can later trust.
        TargetSpec::Spell(filter) => count(
            view,
            view.stack
                .iter()
                .filter(|o| matches!(o.stack_item, Some(StackItem::Spell))),
            filter,
        ),
        TargetSpec::AbilityOnStack(filter) => count(
            view,
            view.stack
                .iter()
                .filter(|o| matches!(o.stack_item, Some(StackItem::Ability { .. }))),
            filter,
        ),
        TargetSpec::SpellOrAbility(filter) => count(view, view.stack.iter(), filter),
        TargetSpec::StackOrBattlefield(filter) => {
            let stack = count(view, view.stack.iter(), filter)?;
            let field = count(view, view.battlefield.iter(), filter)?;
            Some(stack + field)
        }
        // Every other spec, including all four that name a player. A refusal
        // here is the safe direction by construction.
        _ => None,
    }
}

/// Counts the objects a filter matches, or `None` if any of them is a
/// question this view cannot answer.
///
/// One unreadable candidate abandons the whole count, because the claim being
/// built is a *negative*: "none of these matches" is only true if every one of
/// them was actually looked at.
fn count<'a>(
    view: &PlayerView,
    objects: impl Iterator<Item = &'a PublicObject>,
    filter: &Filter,
) -> Option<usize> {
    let mut found = 0;
    for object in objects {
        if matches(view, object, filter)? {
            found += 1;
        }
    }
    Some(found)
}

/// Evaluates a [`Filter`] against one object in a view.
///
/// A mirror of `baylee_engine::eval::matches_projected` and held to the
/// stricter rule that mirror is always held to here: the engine may answer
/// from the `GameState`, and anything this cannot answer from the **view**
/// answers `None`. Every arm is named for the reason the engine's own version
/// names them, so a new [`Filter`] variant has to be looked at here too
/// rather than falling into a wildcard and being quietly declared absent.
fn matches(view: &PlayerView, object: &PublicObject, filter: &Filter) -> Option<bool> {
    // Neither a card nor a registry token, which is **two** kinds of object
    // and the reason this is a bail rather than a reading. A face-down
    // permanent is one: it has no characteristics in the view worth reading,
    // and it is a 2/2 creature and a perfectly legal target for "target
    // creature" (CR 708.2), so trusting its empty fields would conclude "no
    // match" about the object most likely to be one. A token a copy effect
    // made is the other (`PublicObject::token` is `None` for those as well) —
    // an ordinary visible permanent this simply declines to reason about.
    //
    // Only the first is a hazard; the second costs coverage. Both answer
    // `None`, because the proof being built is a negative and the safe
    // direction of a refusal here is to offer the spell.
    if object.card.is_none() && object.token.is_none() {
        return None;
    }
    Some(match filter {
        // `Another` is `Any` here and not by coincidence: both are asked
        // *about a spell still in hand*, so the source is in no zone this
        // walks and no candidate can be it. `This` is the same fact from the
        // other side — nothing on the battlefield or the stack is the card
        // being cast, so the one filter that would name it matches nothing.
        Filter::Any | Filter::Another => true,
        Filter::This => false,
        Filter::And(parts) => {
            for part in *parts {
                if !matches(view, object, part)? {
                    return Some(false);
                }
            }
            true
        }
        Filter::Or(parts) => {
            let mut any = false;
            for part in *parts {
                any |= matches(view, object, part)?;
            }
            any
        }
        Filter::Not(f) => !matches(view, object, f)?,
        Filter::HasType(t) => object.types.intersects(*t),
        Filter::LacksType(t) => !object.types.intersects(*t),
        Filter::HasSupertype(t) => object.supertypes.contains(*t),
        Filter::HasSubtype(s) => object.subtypes.contains(*s),
        Filter::HasColor(c) => object.colors.intersects(*c),
        Filter::IsColorless => object.colors.is_colorless(),
        Filter::Monocolored => object.colors.len() == 1,
        // The engine reads `card.is_none()` and a view must not: `card` is
        // also `None` for a permanent this seat is not entitled to look at,
        // which is a different fact wearing the same shape. `token` is the
        // field the view carries for exactly this question — and the
        // face-down bail above is what keeps the two apart.
        Filter::IsToken => object.token.is_some(),
        Filter::ControlledByYou => object.controller == view.seat,
        // Exact at a duel and refused above it. The engine asks
        // `state.is_opponent`, which knows about teams; a `PlayerView` does
        // not — `SeatIdentity::team` rides in `GameStatic` and never here —
        // so at three seats or more a teammate's creature and an opponent's
        // are the same shape from this side. "Not me" is *not* a safe
        // approximation for it either, because a `Not` around this arm turns
        // an over-count into an under-count and the proof being built is a
        // negative. With exactly two seats there is no third answer.
        Filter::ControlledByOpponent => {
            if view.seats.len() != 2 {
                return None;
            }
            object.controller != view.seat
        }
        Filter::OwnedByYou => object.owner == view.seat,
        Filter::Tapped => object.status.is_tapped(),
        Filter::Untapped => !object.status.is_tapped(),
        Filter::Attacking => view
            .combat
            .attackers
            .iter()
            .any(|attacker| attacker.creature == object.id),
        Filter::CmcAtMost(n) => object.mana_value <= *n,
        Filter::CmcAtLeast(n) => object.mana_value >= *n,
        Filter::ToughnessAtMost(n) => object.toughness.is_some_and(|t| t <= *n),
        // Five the view cannot answer, named rather than swept up. The first
        // three need the *source* object, which is a card in hand that has
        // not been cast and so has no chosen subtype, no attachment and no
        // identity on the battlefield to share. `HasKeyword` is a
        // `KeywordSet` against the view's flat `u128` and is left until
        // something needs it. `InZone` is answerable only for the zone the
        // caller already chose to iterate, which makes it a tautology here
        // rather than a reading.
        Filter::MatchesChosenTypeOfSource
        | Filter::SharesSubtypeWithCommander
        | Filter::AttachedToBySource
        | Filter::HasKeyword(_)
        | Filter::InZone(_) => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::{ViewBuilder, printed};
    use baylee_core::ids::{ObjectId, PrintRef};
    use baylee_core::types::TypeSet;
    use baylee_view::CardIdentity;

    /// A card in hand, named the way a player names it.
    ///
    /// Built from the registry rather than from `ViewBuilder::with_hand`,
    /// which numbers a card's `CardIndex` from its object slot — fine for a
    /// layout test and useless here, where the whole question is what the
    /// printed card says it targets.
    fn in_hand(slot: u32, name: &str) -> baylee_view::HandObject {
        let index =
            baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("`{name}` is in the pool"));
        baylee_view::HandObject {
            id: ObjectId::new(slot, 0),
            card: CardIdentity {
                index,
                print: PrintRef::new(0),
                face: 0,
            },
            name: name.to_string(),
            mana_value: 1,
            colors: baylee_core::color::ColorSet::default(),
            types: TypeSet::INSTANT,
            commander: false,
        }
    }

    /// The reported card, and the whole of #139 in one assertion: Swords to
    /// Plowshares is "destroy target creature" and there is no creature.
    #[test]
    fn a_removal_spell_with_nothing_to_remove_is_proved_targetless() {
        let view = ViewBuilder::new(2).build();
        assert!(provably_targetless(
            &view,
            &in_hand(7, "Swords to Plowshares")
        ));
    }

    /// And one creature is enough to end the proof, whoever controls it —
    /// `Filter::CREATURE` names no controller, so an opponent's is a target.
    #[test]
    fn one_creature_anywhere_ends_the_proof() {
        for seat in [0, 1] {
            let view = ViewBuilder::new(2)
                .with_battlefield(seat, vec![printed(3, seat, "Grizzly Bears", 3)])
                .build();
            assert!(
                !provably_targetless(&view, &in_hand(7, "Swords to Plowshares")),
                "seat {seat}'s creature is a legal target"
            );
        }
    }

    /// A counterspell held over an empty stack is the same fault wearing a
    /// different spec, and it is the one a player meets most often — passing
    /// with a counter in hand is an ordinary thing to do.
    #[test]
    fn a_counterspell_over_an_empty_stack_is_proved_targetless() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![printed(3, 0, "Grizzly Bears", 3)])
            .build();
        assert!(
            provably_targetless(&view, &in_hand(7, "Counterspell")),
            "a creature on the table is not a spell on the stack"
        );
    }

    /// A card that targets nothing is never proved, which is what lets a
    /// caller ask about every card in hand without sorting them first.
    #[test]
    fn a_card_that_needs_no_target_is_never_proved() {
        let view = ViewBuilder::new(2).build();
        assert!(!provably_targetless(&view, &in_hand(7, "Llanowar Elves")));
    }

    /// A permanent nobody may look at abandons the proof.
    ///
    /// The object is a **land** as far as the view can describe it, so a
    /// reader that trusted those fields would count no creature and prove the
    /// spell dead — while a face-down permanent is a 2/2 creature and a
    /// perfectly legal target (CR 708.2). The bail is the difference between
    /// reading a hidden object and reading *through* it.
    ///
    /// The same condition also catches a token a copy effect made, which is
    /// not hidden at all and merely goes unread. That is a cost and not a
    /// fault: both answers are a refusal to prove, and a refusal here means
    /// the spell is offered.
    #[test]
    fn a_permanent_this_seat_may_not_look_at_abandons_the_proof() {
        let mut hidden = printed(3, 1, "", 3);
        hidden.card = None;
        hidden.token = None;
        hidden.types = TypeSet::LAND;
        let view = ViewBuilder::new(2)
            .with_battlefield(1, vec![hidden])
            .build();
        assert!(!provably_targetless(
            &view,
            &in_hand(7, "Swords to Plowshares")
        ));
    }
}
