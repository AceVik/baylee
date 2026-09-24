//! Paying an activation cost that has to ask the player a question.
//!
//! "{T}, Sacrifice a creature: Add {B}{B}" names no creature, and until this
//! module existed the engine had nowhere to ask which one. `can_afford`
//! refused `CostPart::Sacrifice` and `CostPart::Discard` outright — not for
//! anything it could see on the board, but because `pay_cost` would answer
//! them with "choice costs are not supported yet (M2)" *after* emptying the
//! mana pool, and without rewinding. Five cards in the pool wrote the cost
//! and carried `Coverage::Partial` for exactly that reason, and fifty more
//! print such a line in their `//! Oracle:` header.
//!
//! The question is asked on the seam that already exists. `start_activation`
//! suspends for targets by writing a [`super::PlanKind`] and a `Pending`, and
//! `apply` re-enters it once the answer is in; the cost stage is the same
//! move one step later, in the order CR 601.2 gives and CR 602.2b applies to
//! an activation — targets at 601.2c, costs at 601.2h.
//!
//! **Choosing what to sacrifice is not targeting** (CR 115.1: only an effect
//! that uses the word "target" targets). A creature with hexproof may be
//! sacrificed to its controller's own Viscera Seer, and sacrificing does not
//! make anything "become the target of" an ability. The answer therefore
//! arrives as [`crate::choice::Pending::ChooseCards`] with its own
//! [`ChoicePrompt`], which is where a cost belongs: `ChoicePrompt::Delve`
//! was already there for the same reason, and the convoke question next door
//! is labelled `TargetPrompt::Convoke` on the same argument.
//!
//! One function answers what may be chosen, and both readers use it. A board
//! question in `can_afford` that disagreed with the list put in front of the
//! player is the offer/apply contradiction this engine treats as its worst
//! kind: an ability offered and then refused, or worse, a list holding
//! something the payment will not accept.
//!
//! One thing the pair is honestly not exact about, and no card in the pool
//! reaches it. `can_afford` counts **equal** parts against the menu — Time
//! Sieve's five sacrifices, Mines of Moria's three exiles — but two
//! *different* asking parts whose filters overlap on one narrow board ("sacrifice
//! a creature, sacrifice an artifact" over a single artifact creature) are each
//! read on their own, so the ability is offered, asks its first question, and
//! is refused at the second for want of anything left to name. Nothing has
//! been paid at the moment of the refusal, because `start_activation` asks
//! every question before `pay_cost` runs. Making the offer exact is a matching
//! problem, and not one a card is asking yet.

use super::{
    Cause, Cost, CostPart, EngineError, GameEvent, ObjectId, PlayerId, Status, ZoneLocation,
    ZonePosition,
};
use crate::choice::ChoicePrompt;
use crate::eval;
use crate::state::GameState;

/// Whether this part cannot be paid until somebody names an object.
///
/// The successor to `choice_cost_unpayable`, and the opposite of it: that
/// predicate said "refuse this", this one says "ask about this". Every
/// reader that used to relax when the old one answered `false` now asks
/// [`options`] instead, which is a question about the board rather than
/// about the engine's own limits.
pub(crate) const fn needs_an_answer(part: &CostPart) -> bool {
    matches!(
        part,
        CostPart::Sacrifice(_)
            | CostPart::Discard(_)
            | CostPart::TapOther(_)
            | CostPart::ReturnToHand(_)
            | CostPart::ExileFromGraveyard(_)
    )
}

/// How many answers a cost needs before it can be paid.
pub(crate) fn answers_wanted(cost: &Cost) -> usize {
    cost.parts.iter().filter(|p| needs_an_answer(p)).count()
}

/// The parts that need an answer, in the order the cost prints them.
///
/// The order is the contract between the question and the payment: the nth
/// answer pays the nth asking part, so a cost with a sacrifice and a discard
/// cannot pay one with the other's card.
pub(crate) fn asking_parts(cost: &Cost) -> impl Iterator<Item = &CostPart> {
    cost.parts.iter().filter(|p| needs_an_answer(p))
}

/// What `part` may be paid with, on this board.
///
/// The single reader of that question. `can_afford` asks whether this is
/// empty and the prompt hands the same list to the player, so the two cannot
/// drift — which is the whole reason it is a function and not two loops.
///
/// Ownership is enforced here rather than left to the filter, because it is
/// the rule and not the card: CR 701.21a only lets a player sacrifice a
/// permanent **they control**, and CR 701.9a discards from their own hand.
/// Survival of the Fittest prints `Discard(&Filter::CREATURE)` with no
/// "you control" in it at all, and reading that filter alone over every hand
/// at the table would have offered an opponent's card.
///
/// [`CostPart::TapOther`] is the one where no rule says whose — the card
/// prints "a creature **you control**" and all thirteen in this pool do — so
/// the same line is drawn here anyway, deliberately narrower than the rules
/// require. A cost paid by tapping something across the table is not a thing
/// Magic prints, and being wrong in this direction offers a player less than
/// the card allows rather than handing them an opponent's permanent.
///
/// [`CostPart::ReturnToHand`] is the second of that kind and the measurement
/// behind it is stronger: 52 `Cost$` lines in the card-script reference print
/// a return cost naming something other than the source, and **all 52** print
/// "you control". The filter says it too — the transcoder writes
/// `Filter::ControlledByYou` into every one it emits — and this line is the
/// second half of the same answer rather than a substitute for it, which is
/// Earthcraft's arrangement one variant up.
///
/// [`CostPart::ExileFromGraveyard`] reads the payer's own graveyard and no
/// other, which is [`CostPart::Discard`]'s arrangement: the zone says whose,
/// so there is nothing for a controller check to add — a card in a
/// graveyard has no controller (CR 108.4), and the only cards in a
/// player's graveyard are that player's own (CR 400.3).
///
/// What it does *not* borrow from `TapOther` is "untapped". A Forest tapped
/// for `{G}` is the cost Quirion Ranger was printed to pay, so the word is
/// absent here and lives in the filter on the six costs that print it.
///
/// What the rule does supply there is "untapped": CR 118.3 says a permanent
/// that is already tapped cannot be tapped to pay a cost, whether or not the
/// card thought to say so. Summoning sickness is deliberately *not* read —
/// CR 302.6 restricts a creature's own `{T}` ability and says nothing about
/// a creature being tapped to pay for somebody else's, which is why a
/// freshly cast Bird can pay Earthcraft the turn it arrives.
pub(crate) fn options(
    state: &GameState,
    player: PlayerId,
    source: ObjectId,
    part: &CostPart,
) -> Vec<ObjectId> {
    let (zone, controlled, untapped) = match part {
        CostPart::Sacrifice(_) | CostPart::ReturnToHand(_) => {
            (ZoneLocation::Battlefield, true, false)
        }
        CostPart::TapOther(_) => (ZoneLocation::Battlefield, true, true),
        CostPart::Discard(_) => (ZoneLocation::Hand(player), false, false),
        CostPart::ExileFromGraveyard(_) => (ZoneLocation::Graveyard(player), false, false),
        _ => return Vec::new(),
    };
    let (CostPart::Sacrifice(filter)
    | CostPart::Discard(filter)
    | CostPart::TapOther(filter)
    | CostPart::ReturnToHand(filter)
    | CostPart::ExileFromGraveyard(filter)) = part
    else {
        return Vec::new();
    };
    // The battlefield as the rules see it, not as the zone lists it: a
    // phased-out permanent is treated as though it does not exist
    // (CR 702.26b), so it can neither be sacrificed, tapped nor returned to
    // pay anything.
    let listed = if zone == ZoneLocation::Battlefield {
        state.battlefield_view()
    } else {
        state.zones.list(zone).clone()
    };
    listed
        .iter()
        .filter(|id| {
            state.object(**id).is_some_and(|o| {
                (!controlled || o.controller == player)
                    && (!untapped || !o.status.contains(Status::TAPPED))
                    && eval::matches(filter, state, o, player, source)
            })
        })
        .copied()
        .collect()
}

/// What `part` may be paid with **as part of `cost`**: [`options`], less
/// the source when the same cost already spends it the same way.
///
/// "{T}, Tap an untapped creature you control" is Selesnya Evangel's price,
/// and the Evangel is an untapped creature its controller controls — so
/// [`options`] lists it, and a menu of one would pay both taps with one
/// permanent. It cannot: the `{T}` taps it, and a permanent that is already
/// tapped cannot be tapped to pay a cost (CR 118.3, CR 701.26a). The same
/// holds for each pair where the cost names the source and then asks for
/// "another" of the same kind of payment — a sacrifice beside
/// `SacrificeSelf`, a return beside `ReturnSelfToHand`, a discard beside
/// `DiscardSelf`, an exile beside `ExileSelf` — because an object moved by
/// one part is not there to be moved by the other.
///
/// A different kind of payment leaves the source on the menu: "{T},
/// Sacrifice a creature" may sacrifice the creature that tapped, and
/// Viscera Seer, which prints no `{T}`, may sacrifice itself.
///
/// `can_afford` counts this and the prompt shows it, so the offer and the
/// question cannot disagree. `PlayerMayPayCostOr` asks [`options`] instead:
/// its price is one part with no source-spending part beside it.
pub(crate) fn menu(
    state: &GameState,
    player: PlayerId,
    source: ObjectId,
    cost: &Cost,
    part: &CostPart,
) -> Vec<ObjectId> {
    let mut listed = options(state, player, source, part);
    if cost.parts.iter().any(|own| spends_the_source_as(own, part)) {
        listed.retain(|id| *id != source);
    }
    listed
}

/// Whether `own` pays with the source the way `part` would pay with a
/// chosen object — the pairs [`menu`] keeps the source off.
const fn spends_the_source_as(own: &CostPart, part: &CostPart) -> bool {
    matches!(
        (own, part),
        (CostPart::TapSelf, CostPart::TapOther(_))
            | (CostPart::SacrificeSelf, CostPart::Sacrifice(_))
            | (CostPart::ReturnSelfToHand, CostPart::ReturnToHand(_))
            | (CostPart::DiscardSelf, CostPart::Discard(_))
            | (CostPart::ExileSelf, CostPart::ExileFromGraveyard(_))
    )
}

/// What the player is being asked for.
pub(crate) const fn prompt(part: &CostPart) -> ChoicePrompt {
    match part {
        CostPart::Discard(_) => ChoicePrompt::CostDiscard,
        CostPart::TapOther(_) => ChoicePrompt::CostTap,
        CostPart::ReturnToHand(_) => ChoicePrompt::CostReturn,
        CostPart::ExileFromGraveyard(_) => ChoicePrompt::CostExile,
        _ => ChoicePrompt::CostSacrifice,
    }
}

/// Pays one asking part with the object the player named.
///
/// A sacrifice and a discard put a card in its owner's graveyard through
/// [`GameState::move_object`] under [`Cause::Cost`], which is what the
/// `SacrificeSelf` and `DiscardSelf` arms of `pay_cost` already do — the same
/// door, so a sacrifice chosen by a player and a sacrifice printed on the
/// card cannot come out as two different events. A tap goes through the
/// `TapSelf` arm's door for the same reason, down to the
/// [`GameEvent::ObjectTapped`] it records: an ability that triggers on a
/// creature becoming tapped may not see one of the two and miss the other.
///
/// A return goes through the same door to a different zone, which is the
/// `ReturnSelfToHand` arm of `pay_cost` one file over: a permanent bounced
/// to pay a cost and one that bounced itself are the same event. An exile
/// from the graveyard is the `ExileSelf` arm's door the same way, into the
/// owner's exile (CR 406.2).
///
/// The part is passed in rather than inferred from the object, because the
/// object cannot say it. A creature on the battlefield is a legal answer to
/// a sacrifice, a tap and a return alike, and the three are different
/// outcomes.
///
/// The legality of the answer is checked by `apply` against the very list
/// [`options`] produced, before this is ever reached. This re-reads the owner
/// and nothing else.
pub(crate) fn pay(
    state: &mut GameState,
    player: PlayerId,
    part: &CostPart,
    chosen: ObjectId,
) -> Result<(), EngineError> {
    if matches!(part, CostPart::TapOther(_)) {
        state.set_tapped(chosen, true);
        state.journal.record(GameEvent::ObjectTapped {
            object: chosen,
            cause: Cause::Cost,
        });
        return Ok(());
    }
    let owner = state.object(chosen).map_or(player, |o| o.owner);
    // Owner's hand, never the payer's: CR 400.3 puts a returned card in the
    // zone of the player who owns it, and a Forest borrowed off somebody
    // else's battlefield goes home rather than joining the borrower's hand.
    let to = match part {
        CostPart::ReturnToHand(_) => ZoneLocation::Hand(owner),
        CostPart::ExileFromGraveyard(_) => ZoneLocation::Exile(owner),
        _ => ZoneLocation::Graveyard(owner),
    };
    state.move_object(chosen, to, ZonePosition::Top, Cause::Cost)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::{ObjectKind, Status};
    use crate::state::CardLookup;
    use baylee_cards_dsl::Filter;
    use baylee_core::ids::CardIndex;
    use baylee_core::preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
        SeatController, SeatSpec,
    };
    use baylee_core::types::TypeSet;

    struct RegistryLookup;
    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn me() -> PlayerId {
        PlayerId::new(0)
    }
    fn them() -> PlayerId {
        PlayerId::new(1)
    }

    fn state() -> GameState {
        let forest = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
            .expect("registry contains Forest")
            .index;
        let entry = DeckEntry {
            card: forest,
            print: baylee_core::ids::PrintRef::new(0),
        };
        let seat = || SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: SeatCapabilities::default(),
            deck: (0..60).map(|_| entry).collect(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            format: FormatId::Freeform,
            seed: 14,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: baylee_core::preset::Finish::Normal,
            }],
            seats: vec![seat(), seat()],
        };
        GameState::from_preset(&preset, &RegistryLookup).expect("game starts")
    }

    /// A creature for `owner`, in `zone`.
    fn creature(
        state: &mut GameState,
        owner: PlayerId,
        zone: ZoneLocation,
        name: &str,
    ) -> ObjectId {
        let name = state.names.intern(name);
        let id = state.create_bare(owner, ObjectKind::Permanent, name, zone);
        state.object_mut(id).expect("just made it").base_mut().types = TypeSet::CREATURE;
        id
    }

    /// Ownership is the **rule** and not the card: CR 701.21a only lets a
    /// player sacrifice a permanent they control, and Survival of the
    /// Fittest prints `Discard(&Filter::CREATURE)` with no "you control" in
    /// it at all — read over every hand at the table, that filter would
    /// have offered an opponent's card.
    #[test]
    fn a_cost_is_paid_from_the_payers_own_side_whatever_the_filter_says() {
        let mut state = state();
        let source = creature(&mut state, me(), ZoneLocation::Battlefield, "Seer");
        let mine = creature(&mut state, me(), ZoneLocation::Battlefield, "Mine");
        creature(&mut state, them(), ZoneLocation::Battlefield, "Theirs");
        let in_my_hand = creature(&mut state, me(), ZoneLocation::Hand(me()), "In Hand");
        creature(&mut state, them(), ZoneLocation::Hand(them()), "Their Hand");

        let anything = &Filter::CREATURE;
        assert_eq!(
            options(&state, me(), source, &CostPart::Sacrifice(anything)),
            vec![source, mine],
            "the source is a creature it controls too, and sacrificing it \
             is a thing cards print"
        );
        assert_eq!(
            options(&state, me(), source, &CostPart::Discard(anything)),
            vec![in_my_hand],
            "the discard comes out of the payer's own hand"
        );
    }

    /// A graveyard exile reads **the payer's** graveyard and nothing else,
    /// through its filter: a creature card of mine is on the menu, a land
    /// card of mine is not, and neither is a creature card in the other
    /// graveyard or a creature on the battlefield. The zone is the whole of
    /// the "your" — the filter here says only "creature", as the cards
    /// write it.
    #[test]
    fn a_graveyard_exile_reads_only_the_payers_graveyard_through_its_filter() {
        let mut state = state();
        let source = creature(&mut state, me(), ZoneLocation::Battlefield, "Haunt");
        let buried = creature(&mut state, me(), ZoneLocation::Graveyard(me()), "Buried");
        let land = state.names.intern("Buried Land");
        let land = state.create_bare(me(), ObjectKind::Card, land, ZoneLocation::Graveyard(me()));
        state
            .object_mut(land)
            .expect("just made it")
            .base_mut()
            .types = TypeSet::LAND;
        creature(
            &mut state,
            them(),
            ZoneLocation::Graveyard(them()),
            "Theirs",
        );
        creature(&mut state, me(), ZoneLocation::Battlefield, "Standing");

        let part = CostPart::ExileFromGraveyard(&Filter::CREATURE);
        assert!(needs_an_answer(&part), "a card has to be named to pay it");
        assert_eq!(prompt(&part), ChoicePrompt::CostExile);
        assert_eq!(
            options(&state, me(), source, &part),
            vec![buried],
            "my creature card, and not my land card, their creature card or \
             a creature on the battlefield"
        );
        assert_eq!(
            options(
                &state,
                me(),
                source,
                &CostPart::ExileFromGraveyard(&Filter::Any)
            ),
            vec![buried, land],
            "and the filter is what kept the land out"
        );
    }

    /// The card goes to its owner's **exile** (CR 406.2), through the door
    /// every cost uses and under [`Cause::Cost`] — not to the graveyard it
    /// came from, which is where the other parts' fall-through would put it
    /// and where it already is, so a payment that forgot the arm would move
    /// nothing and report success.
    #[test]
    fn a_card_exiled_to_pay_goes_to_its_owners_exile_under_cause_cost() {
        let mut state = state();
        let buried = creature(&mut state, me(), ZoneLocation::Graveyard(me()), "Buried");

        pay(
            &mut state,
            me(),
            &CostPart::ExileFromGraveyard(&Filter::CREATURE),
            buried,
        )
        .expect("a card in the payer's graveyard pays it");

        assert!(state.zones.contains(buried, ZoneLocation::Exile(me())));
        assert!(!state.zones.contains(buried, ZoneLocation::Graveyard(me())));
        assert!(
            state.journal.entries().iter().any(|e| matches!(
                e.event,
                GameEvent::ZoneChanged {
                    object,
                    from: crate::zone::Zone::Graveyard,
                    to: crate::zone::Zone::Exile,
                    cause: Cause::Cost,
                } if object == buried
            )),
            "the move is journaled as a cost paid out of the graveyard"
        );
    }

    /// CR 118.3: a permanent that is already tapped cannot be tapped to pay
    /// a cost, whether or not the card thought to say so. The word is
    /// supplied by the rule for `TapOther` and by nothing else — a Forest
    /// tapped for `{G}` is the cost Quirion Ranger was printed to pay, so
    /// `ReturnToHand` must keep offering it.
    #[test]
    fn only_a_tap_cost_is_refused_a_tapped_permanent() {
        let mut state = state();
        let source = creature(&mut state, me(), ZoneLocation::Battlefield, "Earthcraft");
        let untapped = creature(&mut state, me(), ZoneLocation::Battlefield, "Untapped");
        let tapped = creature(&mut state, me(), ZoneLocation::Battlefield, "Tapped");
        state
            .object_mut(tapped)
            .expect("just made it")
            .status
            .insert(Status::TAPPED);

        let anything = &Filter::CREATURE;
        assert_eq!(
            options(&state, me(), source, &CostPart::TapOther(anything)),
            vec![source, untapped]
        );
        assert_eq!(
            options(&state, me(), source, &CostPart::ReturnToHand(anything)),
            vec![source, untapped, tapped],
            "Quirion Ranger returns a Forest that is already tapped"
        );
        assert_eq!(
            options(&state, me(), source, &CostPart::Sacrifice(anything)),
            vec![source, untapped, tapped]
        );
    }

    /// A phased-out permanent is treated as though it does not exist
    /// (CR 702.26b), so it pays no cost: not by being sacrificed, tapped or
    /// returned. The zone still lists it, which is why the reader has to be
    /// the battlefield as the rules see it.
    #[test]
    fn a_phased_out_permanent_pays_no_cost() {
        let mut state = state();
        let source = creature(&mut state, me(), ZoneLocation::Battlefield, "Outlet");
        let present = creature(&mut state, me(), ZoneLocation::Battlefield, "Present");
        let away = creature(&mut state, me(), ZoneLocation::Battlefield, "Away");
        state
            .object_mut(away)
            .expect("just made it")
            .status
            .insert(Status::PHASED_OUT);

        let anything = &Filter::CREATURE;
        for part in [
            CostPart::Sacrifice(anything),
            CostPart::TapOther(anything),
            CostPart::ReturnToHand(anything),
        ] {
            assert_eq!(
                options(&state, me(), source, &part),
                vec![source, present],
                "{part:?} is not paid with a permanent that has phased out"
            );
        }
    }

    /// A cost that spends its source one way does not offer the source
    /// again for the same kind of payment, and a cost that spends it some
    /// other way does. Selesnya Evangel's `{T}` beside "tap an untapped
    /// creature you control" is the pool's case; "{T}, sacrifice a
    /// creature" may still sacrifice the creature that tapped.
    #[test]
    fn a_source_spent_by_name_is_not_on_the_menu_for_the_same_payment() {
        let mut state = state();
        let source = creature(&mut state, me(), ZoneLocation::Battlefield, "Evangel");
        let other = creature(&mut state, me(), ZoneLocation::Battlefield, "Other");

        let tap = CostPart::TapOther(&Filter::CREATURE);
        let evangel = baylee_cards_dsl::cost!("{1}", TapSelf, TapOther(&Filter::CREATURE));
        assert_eq!(menu(&state, me(), source, &evangel, &tap), vec![other]);

        let sacrifice = CostPart::Sacrifice(&Filter::CREATURE);
        let tap_and_sacrifice = baylee_cards_dsl::cost!(TapSelf, Sacrifice(&Filter::CREATURE));
        assert_eq!(
            menu(&state, me(), source, &tap_and_sacrifice, &sacrifice),
            vec![source, other],
            "a tap and a sacrifice are two ways, and one permanent may pay both"
        );
        let both = baylee_cards_dsl::cost!(SacrificeSelf, Sacrifice(&Filter::CREATURE));
        assert_eq!(
            menu(&state, me(), source, &both, &sacrifice),
            vec![other],
            "a source sacrificed by name is not there to be sacrificed again"
        );
    }

    /// The parts that ask, in the order the cost prints them, because the
    /// nth answer pays the nth asking part: a cost with a sacrifice and a
    /// discard must not pay one with the other's card. And a part that asks
    /// nothing is not in the list — `options` answers it with nothing, so a
    /// reader that asked anyway would offer an empty choice.
    #[test]
    fn the_asking_parts_keep_the_order_the_cost_prints_them_in() {
        let mut state = state();
        let source = creature(&mut state, me(), ZoneLocation::Battlefield, "Source");
        let cost = baylee_cards_dsl::cost!(
            "{1}",
            TapSelf,
            Sacrifice(&Filter::CREATURE),
            PayLife(1),
            Discard(&Filter::CREATURE)
        );

        assert_eq!(answers_wanted(&cost), 2);
        let asking: Vec<&CostPart> = asking_parts(&cost).collect();
        assert!(matches!(asking[0], CostPart::Sacrifice(_)));
        assert!(matches!(asking[1], CostPart::Discard(_)));
        assert_eq!(
            answers_wanted(&baylee_cards_dsl::cost!("{2}", TapSelf, PayLife(3))),
            0,
            "a cost that names no object asks nothing"
        );

        assert!(!needs_an_answer(&CostPart::TapSelf));
        assert!(
            options(&state, me(), source, &CostPart::TapSelf).is_empty(),
            "and the board has nothing to offer for it"
        );
    }
}
