use super::{
    AbilityDef, AbilityLoc, ActivationTiming, CardLookup, Cause, Cost, CostPart, Engine,
    EngineError, GameEvent, GameObject, LegalActions, NameRef, ObjectId, ObjectKind, Pending,
    Phase, PlanKind, PlayerId, Resolution, SmallVec, Status, TypeSet, Zone, ZoneLocation,
    ZonePosition, casting, cost_wizard, eval, resolve,
};
use crate::choice::TargetPrompt;
use baylee_cards_dsl::{ActivationLimit, ActivationZone};

mod activation;
mod costs;
mod grants;
mod limits;
mod loyalty;
mod modes;
mod offers;
pub(crate) use modes::ActivationMode;
mod payment;
pub(crate) use payment::ActivationPayment;
mod prepared;
mod restrictions;
mod stack;
mod x_value;

/// A determined activation price keeps source permissions beside its mana.
#[derive(Clone, Copy, Debug, Hash)]
pub(crate) struct ActivationPrice {
    pub cost: Cost,
    pub allowance: casting::SourceAllowance,
}
impl std::ops::Deref for ActivationPrice {
    type Target = Cost;
    fn deref(&self) -> &Cost {
        &self.cost
    }
}
impl ActivationPrice {
    pub(super) fn with_mana(self, mana: baylee_core::mana::ManaCost) -> Self {
        Self {
            cost: Cost {
                mana,
                parts: self.cost.parts,
            },
            ..self
        }
    }
}

/// The cost parts an activation would leave unpaid, and think it had paid.
///
/// These are paid in `cast_wizard` — the pitch of Force of Will, the X of
/// Toxic Deluge — which is a path `pay_cost` is never shown, so it walks past
/// both. For a spell's alternative or additional cost that is exactly right.
/// For an **activated** ability there is no wizard, and the ability is
/// offered, pressed, and its part silently not paid — a pitch cost that
/// exiles nothing.
///
/// This used to have a twin, `choice_cost_unpayable`, which named the parts an
/// activation could not pay *at all* — a sacrifice or a discard, because
/// nothing could ask which one. `cost_wizard` asks now, so that half is gone
/// and what is left here is the opposite failure: a part that would be
/// *accepted and then skipped*. `can_afford` is no longer the other half of
/// the sentence either. It reads `ExileFromHand` for real (there has to be a
/// card to exile, or the offer is dead), and the wizard caps `PayLifeX` at
/// the caster's life.
///
/// That makes this the invisible one of the two failures. A cost that cannot
/// be paid ends in an error; this is offered and then granted for free, and
/// the activation *succeeds* — no test of an action can fail on it, and the
/// only evidence is the card still sitting in a hand that should have paid
/// it. Which is why
/// the reader that matters is a pool-wide guard,
/// `offer_tests::nothing_in_the_pool_carries_an_activated_cost_the_engine_would_skip`,
/// and why nothing here refuses: the parts are correct where they are used,
/// and the guard is what keeps them from being used anywhere else.
///
/// Exhaustive on purpose, and this is the enum where that matters most: a
/// `matches!` answers a *new* cost part with `false`, which here means "pay
/// it for real" and is the safe direction — but the same shape one function
/// down in `can_afford` answered `CostPart::TapOther` with silence and lost
/// the house AI a card. A part added to this enum should have to say which
/// side of this line it is on, in one word, at compile time.
pub(crate) const fn paid_by_the_casting_wizard(part: &CostPart) -> bool {
    match part {
        CostPart::ExileFromHand(_) | CostPart::PayLifeX => true,
        CostPart::TapSelf
        | CostPart::UntapSelf
        | CostPart::ExertSelf
        | CostPart::SacrificeSelf
        | CostPart::Sacrifice(_)
        | CostPart::ExilePermanent(_)
        | CostPart::SacrificeX(_)
        | CostPart::SacrificeAll(_)
        | CostPart::PayLife(_)
        | CostPart::PayCommanderColors
        | CostPart::Discard(_)
        | CostPart::DiscardRandom
        | CostPart::DiscardHand
        | CostPart::DiscardLastDrawnThisTurn
        | CostPart::PayEnergy(_)
        | CostPart::PayEnergyX
        | CostPart::TapN { .. }
        | CostPart::TapOther(_)
        | CostPart::TapX(_)
        | CostPart::Crew(_)
        | CostPart::Saddle(_)
        | CostPart::ReturnToHand(_)
        | CostPart::ExileFromGraveyard(_)
        | CostPart::DiscardSelf
        | CostPart::ExileSelf
        | CostPart::ReturnSelfToHand
        | CostPart::RemoveCounterSelf { .. }
        | CostPart::RemoveCounterSelfX { .. }
        | CostPart::PutCounterSelf { .. }
        | CostPart::PutCounterOnOther { .. }
        | CostPart::ExileFromSingleGraveyard { .. }
        | CostPart::ExileTopOfLibrary(_)
        | CostPart::PayAttachedManaCost => false,
    }
}

/// Whether paying `part` can refuse after something of the cost was already
/// paid, which is when [`Engine::pay_cost`] keeps a checkpoint to cancel the
/// payment with (CR 733.1).
///
/// The mana is paid first and all at once or not at all, so a cost whose
/// every part answers `false` here can refuse only before it has written
/// anything: a land's `{T}: Add {G}`, which is most of what is ever paid,
/// takes no copy of the game. Exhaustive for the reason the function above
/// is: a new part says which side it is on.
const fn can_refuse_part_way(part: &CostPart) -> bool {
    match part {
        // A move of an object that may no longer be there, a counter that
        // may already be gone, an answer that may be missing.
        CostPart::PayCommanderColors
        | CostPart::SacrificeSelf
        | CostPart::DiscardSelf
        | CostPart::ExileSelf
        | CostPart::ReturnSelfToHand
        | CostPart::RemoveCounterSelf { .. }
        | CostPart::RemoveCounterSelfX { .. }
        | CostPart::Sacrifice(_)
        | CostPart::ExilePermanent(_)
        | CostPart::SacrificeX(_)
        | CostPart::SacrificeAll(_)
        | CostPart::Discard(_)
        | CostPart::DiscardRandom
        | CostPart::DiscardHand
        | CostPart::DiscardLastDrawnThisTurn
        | CostPart::PayEnergy(_)
        | CostPart::PayEnergyX
        | CostPart::TapN { .. }
        | CostPart::TapOther(_)
        | CostPart::TapX(_)
        | CostPart::Crew(_)
        | CostPart::Saddle(_)
        | CostPart::ReturnToHand(_)
        | CostPart::ExileFromGraveyard(_)
        | CostPart::ExileFromSingleGraveyard { .. }
        | CostPart::ExileTopOfLibrary(_)
        | CostPart::PutCounterOnOther { .. } => true,
        CostPart::TapSelf
        | CostPart::UntapSelf
        | CostPart::ExertSelf
        | CostPart::PayLife(_)
        | CostPart::PutCounterSelf { .. }
        | CostPart::ExileFromHand(_)
        | CostPart::PayLifeX
        | CostPart::PayAttachedManaCost => false,
    }
}

/// Whether an ability's effects ask how often it has been activated this
/// turn (`Effect::IfActivatedThisTurnAtLeast`, Dragon Whelp), anywhere in
/// the list: then its activations are counted as they happen.
fn asks_its_activations(effects: &'static [baylee_cards_dsl::Effect]) -> bool {
    let mut asks = false;
    baylee_cards_dsl::Effect::walk(effects, &mut 0, &mut |effect| {
        asks |= matches!(
            effect,
            baylee_cards_dsl::Effect::IfActivatedThisTurnAtLeast { .. }
        );
    });
    asks
}

impl<L: CardLookup> Engine<L> {
    /// Whether the offered activation `(source, index)` is a mana ability,
    /// asked of the object's own list or, for a granted slot, of the grants
    /// — the one reading both narrowings share.
    pub(super) fn is_mana_offer(&self, source: ObjectId, index: u32) -> bool {
        let Some(obj) = self.state.object(source) else {
            return false;
        };
        if crate::choice::granted_slot(index).is_some() {
            return crate::effects::granted_activated(&self.state, source)
                .find(|grant| grant.id == index)
                .is_some_and(|granted| granted.mana_ability);
        }
        obj.abilities(&self.lookup)
            .get(index as usize)
            .is_some_and(AbilityDef::is_mana_ability)
    }

    /// Mana abilities usable during another cost's payment, without priority.
    pub(super) fn is_payment_mana_offer(&self, source: ObjectId, index: u32) -> bool {
        self.is_mana_offer(source, index)
            && !matches!(
                self.activation_definition(source, index),
                Some(
                    AbilityDef::Activated {
                        timing: ActivationTiming::PriorityOnly,
                        ..
                    } | AbilityDef::ModalActivated {
                        timing: ActivationTiming::PriorityOnly,
                        ..
                    } | AbilityDef::ActivatedConditional {
                        timing: ActivationTiming::PriorityOnly,
                        ..
                    }
                )
            )
    }

    // ---------------------------------------------------- S3: abilities
}
