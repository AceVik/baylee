//! Shared non-mana spell costs for offers, choices, and payment.
use super::{CastModeKind, CostPart, ObjectId, PlayerId, SmallVec, casting};

/// The mandatory additional cost parts for `face`, plus entwine's non-mana
/// cost parts when `option` chooses all modes (CR 702.42a).
pub(crate) fn wizard_mandatory_parts(
    face: &baylee_cards_dsl::FaceDef,
    option: Option<CastModeKind>,
) -> impl Iterator<Item = &CostPart> {
    let entwine_parts = match (option, face.entwine) {
        (Some(CastModeKind::Modes(_)), Some(entwine)) => entwine.parts,
        _ => &[],
    };
    face.mandatory_additional_costs
        .iter()
        .chain(entwine_parts.iter())
}

/// The non-mana parts of the alternative cost chosen for this cast.
/// Flashback is an alternative cost too, including one with no mana component.
pub(crate) fn wizard_alternative_parts(
    face: &'static baylee_cards_dsl::FaceDef,
    option: Option<CastModeKind>,
) -> &'static [CostPart] {
    match option {
        Some(CastModeKind::Alternative(index)) => face.alternative_costs[index].cost.parts,
        Some(CastModeKind::Flashback) => face.flashback_parts,
        Some(CastModeKind::Madness) => face.madness.expect("madness").parts,
        _ => &[],
    }
}

/// The non-mana parts of the buyback cost, when it is being paid.
pub(crate) fn wizard_buyback_parts(
    face: &'static baylee_cards_dsl::FaceDef,
    paid: bool,
) -> &'static [CostPart] {
    match face.buyback {
        Some(cost) if paid => cost.parts,
        _ => &[],
    }
}

/// Which of a face's non-mana costs a cast pays: the way it is cast, its
/// alternative cost when that is separate from its modes, and buyback.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CostChoice {
    /// The chosen way of casting.
    pub option: Option<CastModeKind>,
    /// Alternative cost paid independently of a modal choice (CR 118.9a).
    pub alternative_cost: Option<CastModeKind>,
    /// Whether the buyback cost is paid.
    pub buyback: bool,
}

impl From<Option<CastModeKind>> for CostChoice {
    fn from(option: Option<CastModeKind>) -> Self {
        Self {
            option,
            alternative_cost: None,
            buyback: false,
        }
    }
}

/// Every non-mana cost part the selected way of casting must pay. Alternative
/// parts precede mandatory additional parts and those precede buyback's,
/// matching the payment below.
pub(crate) fn wizard_cost_parts(
    face: &'static baylee_cards_dsl::FaceDef,
    choice: impl Into<CostChoice>,
) -> impl Iterator<Item = &'static CostPart> {
    let CostChoice {
        option,
        alternative_cost,
        buyback,
    } = choice.into();
    wizard_alternative_parts(face, alternative_cost.or(option))
        .iter()
        .chain(wizard_mandatory_parts(face, option))
        .chain(wizard_buyback_parts(face, buyback))
}

/// The object questions of a spell's cost, in payment order. `TapN` expands
/// into N questions so offer, choice and payment share one answer per object.
/// Externally imposed parts follow the card's printed and selected alternative parts.
pub(crate) fn additional_card_costs_for(
    state: &crate::state::GameState,
    player: PlayerId,
    card: ObjectId,
    face: &'static baylee_cards_dsl::FaceDef,
    choice: impl Into<CostChoice>,
) -> SmallVec<[CostPart; 4]> {
    let mut parts: SmallVec<[CostPart; 4]> = wizard_cost_parts(face, choice)
        .filter(|p| {
            matches!(
                p,
                CostPart::Sacrifice(_)
                    | CostPart::ExilePermanent(_)
                    | CostPart::ExileFromGraveyard(_)
                    | CostPart::Discard(_)
                    | CostPart::ReturnToHand(_)
                    | CostPart::TapN { .. }
            )
        })
        .flat_map(|part| {
            let count = match part {
                CostPart::TapN { count, .. } => usize::try_from(*count).unwrap_or(usize::MAX),
                _ => 1,
            };
            std::iter::repeat_n(*part, count)
        })
        .collect();
    parts.extend(casting::additional_spell_costs(state, player, card, face));
    parts
}
