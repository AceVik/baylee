//! The ways a card in hand could be cast, and what each would cost to reach.
//!
//! # Why this exists
//!
//! The engine counts a spell's ways against the mana that is **floating**
//! (`cast_wizard::cast_options`, whose `afford` closure reads the seat's own
//! pool), which is the right question for the moment it asks it: the engine
//! has no planner and cannot know which lands are about to be tapped. The
//! client does have one, and it used to float **exactly the printed cost**
//! and then send `CastSpell` — so by the time the engine counted, only one
//! way was payable and there was nothing left to ask.
//!
//! That silence went both directions. An alternative cost **dearer** than the
//! printed one could never be chosen by clicking at all — Reveillark's evoke
//! is `{5}{W}` against a printed `{4}{W}`, and seven open mana cast it the
//! cheap way without a word. A **free** alternative was chosen just as
//! quietly in the other direction: Solitude with an empty pool is already in
//! `LegalActions::castable`, so the click cast it for its evoke and the
//! printed `{3}{W}{W}` was unreachable for a player who would rather keep the
//! white card in their hand.
//!
//! So the question has to come **before** the floating, and this module is
//! what it is asked from: every way this client can both *read* and *pay
//! for*, each with the plan that pays for it. The engine stays exactly as it
//! is.
//!
//! # What it refuses, and why that is the honest half
//!
//! A way offered here that the engine will not offer is a row that lies: the
//! player picks it, the mana is floated, and the answer the client has
//! remembered fits nothing in the engine's list. So a way is offered only
//! when **every** part of its cost is one this reader can evaluate from the
//! view:
//!
//! - the mana part, through [`baylee_client_core::manaplan`] as everywhere
//!   else — which is also where `{X}`, `{S}` and restricted mana are refused;
//! - [`CostPart::PayLife`], against the seat's own life total;
//! - [`CostPart::ExileFromHand`] with a colour filter, against the hand — the
//!   pitch on Force of Will, Force of Negation, Misdirection and Solitude,
//!   and the one non-mana part in this pool that a seat's view can answer.
//!
//! Anything else — another filter shape, a sacrifice, a discard — takes the
//! way off the list rather than being guessed at.
//!
//! # And what it does not reach
//!
//! Only [`CastModeKind::Normal`] and [`CastModeKind::Alternative`], out of a
//! card in **hand**. The other four kinds are deliberately left to the
//! engine's own chooser:
//!
//! - [`CastModeKind::Mode`] — a modal spell — needs
//!   `casting::mode_has_a_legal_target`, which takes the `GameState` this
//!   client does not have and must not approximate. Damn's overload is
//!   therefore still cast the cheap way, and that is **AM1**'s to close.
//! - [`CastModeKind::Face`] and [`CastModeKind::PlayLandFace`] ask the same
//!   question about a back face; [`CastModeKind::Miracle`] is not a choice a
//!   click makes.
//!
//! Nothing here reads the commander tax, convoke, delve or a printed cost
//! reduction either — the same four things [`crate::mana_for`] and
//! [`crate::reachable`] already do not read. A way whose real price is lower
//! than this thinks is a way that stays off the list, which costs a click and
//! never a wrong cast.

use baylee_cards_dsl::{AltCondition, CostPart, Filter};
use baylee_client_core::manaplan::{self, Plan};
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaCost;
use baylee_engine::choice::{CastModeKind, LegalActions};
use baylee_view::PlayerView;

/// One way to cast a card, and the taps that would pay for it.
#[derive(Clone, Debug)]
pub struct ReachableMode {
    /// Which way this is.
    ///
    /// The **kind** and never the engine's option index: that index is a
    /// position in a list the engine builds against the pool at the instant
    /// it asks, and the pool at that instant is the one this client is about
    /// to change.
    pub kind: CastModeKind,
    /// The mana part to float for it.
    pub cost: ManaCost,
    /// The taps that float it — empty when the mana is already up, which is
    /// also what a free alternative cost plans to.
    pub plan: Plan,
}

/// Every way `card` could be cast right now that this client can read and pay
/// for, in the order a chooser should draw them.
///
/// Printed cost first, then each alternative in the order the card prints
/// them, because that is the order the engine offers and the order the card
/// reads.
///
/// An empty list is an ordinary answer and so is a list of one: the caller
/// asks a question only when there is more than one way, and everything else
/// takes the path it always took.
#[must_use]
pub fn reachable_modes(
    view: &PlayerView,
    legal: &LegalActions,
    card: ObjectId,
) -> Vec<ReachableMode> {
    let Some(hand) = view.hand.iter().find(|c| c.id == card) else {
        return Vec::new();
    };
    let Some(def) = baylee_cards::by_index(hand.card.index) else {
        return Vec::new();
    };
    // The same face `manasources::hand_cost` reads, for the same reason: an
    // adventure or an MDFC in hand is shown by whichever face is up, and a
    // cost read off the other one is a price nobody is being quoted.
    let Some(face) = def
        .faces
        .get(hand.card.face as usize)
        .or_else(|| def.faces.first())
    else {
        return Vec::new();
    };
    let Some(pool) = view.seat(view.seat).map(|s| s.mana_pool) else {
        return Vec::new();
    };
    let sources = crate::manasources::sources(view, legal);
    let mut out = Vec::new();
    let mut offer = |kind, cost: ManaCost| {
        if let Some(plan) = manaplan::plan(&cost, &pool, &sources) {
            out.push(ReachableMode { kind, cost, plan });
        }
    };
    // CR 202.1a: a face with no printed cost has no printed way to be cast,
    // which is the rule `casting::has_a_printed_cost` states engine-side and
    // the one that keeps a suspend-only card off this list.
    if face.mana_cost.symbols().next().is_some() {
        offer(CastModeKind::Normal, face.mana_cost);
    }
    for (i, alt) in face.alternative_costs.iter().enumerate() {
        if condition_holds(view, alt.condition) && parts_payable(view, card, alt.cost.parts) {
            offer(CastModeKind::Alternative(i), alt.cost.mana);
        }
    }
    out
}

/// Whether an alternative cost's condition holds, read off the view.
///
/// All three are readable, which is why none of them takes a way off the list
/// the way an unreadable cost part does.
fn condition_holds(view: &PlayerView, condition: AltCondition) -> bool {
    match condition {
        AltCondition::Always => true,
        AltCondition::NotYourTurn => view.active != view.seat,
        // `casting::controls_a_commander`, in the words a view has: a
        // commander handle follows its card through every zone (CR 400.7), so
        // the battlefield is asked rather than the handle believed.
        AltCondition::CommanderControlled => view.seat(view.seat).is_some_and(|seat| {
            seat.commanders.iter().any(|c| {
                view.battlefield
                    .iter()
                    .any(|o| o.id == c.object && o.controller == view.seat)
            })
        }),
    }
}

/// Whether the non-mana parts of an alternative cost could be paid right now.
///
/// `Engine::can_afford` is the function this mirrors, and it is a mirror
/// rather than a copy because it is held to a stricter rule: the engine may
/// answer from the `GameState`, and anything this cannot answer from the
/// **view** is refused. Every arm is named for the reason the engine's own
/// version names them — a twelfth [`CostPart`] has to be looked at here too,
/// rather than quietly falling into a wildcard and being declared payable.
fn parts_payable(view: &PlayerView, card: ObjectId, parts: &[CostPart]) -> bool {
    parts.iter().all(|part| match part {
        CostPart::PayLife(n) => view
            .seat(view.seat)
            .is_some_and(|seat| seat.life >= i32::from(*n)),
        // `casting::pitchable`, and the card being cast is not a candidate
        // for its own pitch — it is what is being paid for. A `HandObject`
        // carries its own colours, so the one filter shape this pool prints
        // here is answered exactly rather than approximately; any other shape
        // takes the way off the list.
        CostPart::ExileFromHand(filter) => match filter {
            Filter::HasColor(colors) => view
                .hand
                .iter()
                .any(|c| c.id != card && c.colors.intersects(*colors)),
            _ => false,
        },
        // Refused, not because they cannot be paid, but because nothing in
        // this pool prints one on an alternative cost and a reader that
        // guessed would be guessing about a card nobody can test against.
        // `Sacrifice` and `Discard` the engine refuses outright
        // (`choice_cost_unpayable`); the rest are paid off the source and
        // would be safe to admit the day a card wants them.
        CostPart::TapSelf
        | CostPart::UntapSelf
        | CostPart::SacrificeSelf
        | CostPart::Sacrifice(_)
        | CostPart::Discard(_)
        | CostPart::DiscardSelf
        | CostPart::ExileSelf
        | CostPart::ReturnSelfToHand
        | CostPart::PayLifeX => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::{ViewBuilder, token};
    use baylee_core::color::{Color, ColorSet};
    use baylee_core::generated::subtypes::land;
    use baylee_core::ids::{CardIndex, PlayerId};
    use baylee_core::types::{SubtypeSet, TypeSet};
    use baylee_view::{HandObject, PlayerView, PublicObject};

    /// The registry index of a card, by the name its front face prints.
    fn index_of(name: &str) -> CardIndex {
        baylee_cards::all()
            .find(|def| def.faces[0].name == name)
            .unwrap_or_else(|| panic!("{name} is not in the pool"))
            .index
    }

    /// A basic land, as the engine offers it: the CR 305.6 shortcut on a land
    /// with exactly one basic type.
    fn basic(slot: u32, name: &str, subtype: baylee_core::ids::SubtypeId) -> PublicObject {
        let mut o = token(slot, 0, name, 0, 0);
        o.types = TypeSet::LAND;
        o.subtypes = SubtypeSet::from_slice(&[subtype]);
        o.power = None;
        o.toughness = None;
        o
    }

    /// One card in hand, named and coloured for real.
    fn card_in_hand(slot: u32, name: &str, colors: &[Color]) -> HandObject {
        HandObject {
            id: ObjectId::new(slot, 0),
            card: baylee_view::CardIdentity {
                index: index_of(name),
                print: baylee_core::ids::PrintRef::new(slot as u16),
                face: 0,
            },
            name: name.to_string(),
            mana_value: 0,
            colors: ColorSet::from_slice(colors),
            types: TypeSet::CREATURE,
            commander: false,
        }
    }

    /// `count` Plains on the table, a hand, and the engine offering every
    /// land's shortcut.
    fn table(hand: Vec<HandObject>, count: u32) -> (PlayerView, LegalActions) {
        lands_of(hand, count, "Plains", land::PLAINS)
    }

    /// The same with Islands, for the blue half of the pool.
    fn blue_table(hand: Vec<HandObject>, count: u32) -> (PlayerView, LegalActions) {
        lands_of(hand, count, "Island", land::ISLAND)
    }

    fn lands_of(
        hand: Vec<HandObject>,
        count: u32,
        name: &str,
        subtype: baylee_core::ids::SubtypeId,
    ) -> (PlayerView, LegalActions) {
        let lands: Vec<PublicObject> = (0..count).map(|i| basic(100 + i, name, subtype)).collect();
        let ids: Vec<ObjectId> = lands.iter().map(|l| l.id).collect();
        let mut view = ViewBuilder::new(2).with_battlefield(0, lands).build();
        view.hand = hand;
        let legal = LegalActions {
            mana_abilities: ids,
            ..LegalActions::default()
        };
        (view, legal)
    }

    fn kinds(modes: &[ReachableMode]) -> Vec<CastModeKind> {
        modes.iter().map(|m| m.kind).collect()
    }

    /// The measurement in the report: seven open mana and Reveillark's two
    /// ways both payable, which is the moment nothing asked.
    #[test]
    fn reveillark_with_six_open_mana_has_two_ways_to_be_cast() {
        let hand = vec![card_in_hand(1, "Reveillark", &[Color::White])];
        let (view, legal) = table(hand, 6);
        let modes = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(
            kinds(&modes),
            vec![CastModeKind::Normal, CastModeKind::Alternative(0)],
            "printed {{4}}{{W}} and evoke {{5}}{{W}}, both reachable with six lands"
        );
        assert_eq!(modes[0].plan.steps.len(), 5, "the printed cost taps five");
        assert_eq!(modes[1].plan.steps.len(), 6, "evoke taps every one of them");
    }

    /// Five is enough for the printed cost and not for the evoke, so there is
    /// nothing to ask and the click keeps the behaviour it always had.
    #[test]
    fn reveillark_with_five_open_mana_has_only_the_printed_way() {
        let hand = vec![card_in_hand(1, "Reveillark", &[Color::White])];
        let (view, legal) = table(hand, 5);
        let modes = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(kinds(&modes), vec![CastModeKind::Normal]);
    }

    /// The other direction of the same defect: the free way is reachable with
    /// no mana at all, so a seat with five lands has **two** ways and was
    /// being given the free one in silence.
    #[test]
    fn solitude_with_five_open_mana_offers_the_printed_cost_as_well() {
        let hand = vec![
            card_in_hand(1, "Solitude", &[Color::White]),
            card_in_hand(2, "Reveillark", &[Color::White]),
        ];
        let (view, legal) = table(hand, 5);
        let modes = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(
            kinds(&modes),
            vec![CastModeKind::Normal, CastModeKind::Alternative(0)]
        );
        assert!(
            modes[1].plan.is_empty(),
            "a pitch costs no mana, so it plans no taps"
        );
    }

    /// The pitch is a cost like any other, and a hand with nothing white in
    /// it cannot pay it. Offering the row anyway would be a chooser that
    /// lies: the engine would not offer the way, and the answer this client
    /// had remembered would fit nothing in its list.
    #[test]
    fn solitude_alone_in_a_hand_cannot_pitch_to_itself() {
        let hand = vec![card_in_hand(1, "Solitude", &[Color::White])];
        let (view, legal) = table(hand, 5);
        let modes = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(kinds(&modes), vec![CastModeKind::Normal]);

        // …and a hand whose other card is the wrong colour is the same
        // answer, which is what says the filter is being read rather than
        // the hand merely counted.
        let hand = vec![
            card_in_hand(1, "Solitude", &[Color::White]),
            card_in_hand(2, "Force of Will", &[Color::Blue]),
        ];
        let (view, legal) = table(hand, 5);
        let modes = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(kinds(&modes), vec![CastModeKind::Normal]);
    }

    /// Force of Negation's pitch is only legal on somebody else's turn
    /// (`AltCondition::NotYourTurn`), and the view is what says whose turn it
    /// is.
    #[test]
    fn a_condition_on_an_alternative_cost_is_read_off_the_view() {
        let hand = vec![
            card_in_hand(1, "Force of Negation", &[Color::Blue]),
            card_in_hand(2, "Force of Will", &[Color::Blue]),
        ];
        let (mut view, legal) = blue_table(hand, 3);

        view.active = PlayerId::new(0);
        let mine = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(kinds(&mine), vec![CastModeKind::Normal], "my own turn");

        view.active = PlayerId::new(1);
        let theirs = reachable_modes(&view, &legal, ObjectId::new(1, 0));
        assert_eq!(
            kinds(&theirs),
            vec![CastModeKind::Normal, CastModeKind::Alternative(0)],
            "not my turn, so the pitch is on offer"
        );
    }

    /// The pool-wide claim this reader rests on, nailed down rather than
    /// written in a comment: every alternative cost in the registry is built
    /// out of the parts [`parts_payable`] reads. The day a card prints one
    /// that is not, this fails — which is the point, because the honest
    /// answer then is to decide what that card's row should say, not to let
    /// it quietly vanish from a chooser.
    #[test]
    fn every_alternative_cost_in_the_pool_is_one_this_reader_can_evaluate() {
        let mut unreadable = Vec::new();
        for def in baylee_cards::all() {
            for face in def.faces {
                for alt in face.alternative_costs {
                    let readable = alt.cost.parts.iter().all(|part| match part {
                        CostPart::PayLife(_) => true,
                        CostPart::ExileFromHand(filter) => {
                            matches!(filter, Filter::HasColor(_))
                        }
                        _ => false,
                    });
                    if !readable {
                        unreadable.push(face.name);
                    }
                }
            }
        }
        assert!(
            unreadable.is_empty(),
            "these cards print an alternative cost this reader refuses: {unreadable:?}"
        );
    }
}
