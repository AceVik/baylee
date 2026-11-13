//! Shared non-mana spell costs for offers, choices, and payment.
use super::{CastModeKind, CostPart, ObjectId, PlayerId, SmallVec, casting};

/// Mandatory additional parts, entwine, and the chosen kicker's non-mana parts.
pub(crate) fn wizard_mandatory_parts(
    face: &baylee_cards_dsl::FaceDef,
    choice: impl Into<CostChoice>,
) -> impl Iterator<Item = &CostPart> {
    let choice = choice.into();
    let entwine_parts = match (choice.option, face.entwine) {
        (Some(CastModeKind::Modes(_)), Some(entwine)) => entwine.parts,
        _ => &[],
    };
    face.mandatory_additional_costs
        .iter()
        .chain(entwine_parts.iter())
        .chain(
            face.additional_costs
                .iter()
                .filter(move |_| choice.kicked)
                .flat_map(|cost| cost.parts),
        )
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
    /// Kicker can accompany a face or mode without replacing that choice.
    pub kicked: bool,
}

impl From<Option<CastModeKind>> for CostChoice {
    fn from(option: Option<CastModeKind>) -> Self {
        Self {
            option,
            alternative_cost: None,
            buyback: false,
            kicked: matches!(
                option,
                Some(
                    CastModeKind::Kicked
                        | CastModeKind::LibraryTopLife { kicked: true, .. }
                        | CastModeKind::EscapeGranted { kicked: true, .. }
                )
            ),
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
    let choice = choice.into();
    wizard_alternative_parts(face, choice.alternative_cost.or(choice.option))
        .iter()
        .chain(wizard_mandatory_parts(face, choice))
        .chain(wizard_buyback_parts(face, choice.buyback))
}

/// Fixed life owed by every selected non-mana cost, before payment begins.
pub(crate) fn fixed_life_cost(
    face: &'static baylee_cards_dsl::FaceDef,
    choice: impl Into<CostChoice>,
) -> i32 {
    wizard_cost_parts(face, choice).fold(0_i32, |sum, part| match part {
        CostPart::PayLife(life) => sum.saturating_add(i32::from(*life)),
        _ => sum,
    })
}

impl super::CastWizard {
    /// Life already committed before an X or another optional cost is chosen.
    pub(super) fn reserved_life(&self, face: &'static baylee_cards_dsl::FaceDef) -> i32 {
        let base = match self.announced {
            Some(CastModeKind::LibraryTopLife { life, .. }) => {
                i32::try_from(life).unwrap_or(i32::MAX)
            }
            _ => 0,
        };
        base.saturating_add(fixed_life_cost(face, self.cost_choice()))
            .saturating_add(super::phyrexian_life(self))
    }

    /// Total life owed after X has been chosen, including every pay-X-life part.
    pub(super) fn total_life_cost(&self, face: &'static baylee_cards_dsl::FaceDef) -> i32 {
        wizard_cost_parts(face, self.cost_choice()).fold(self.reserved_life(face), |sum, part| {
            if matches!(part, CostPart::PayLifeX) {
                sum.saturating_add(i32::try_from(self.x).unwrap_or(i32::MAX))
            } else {
                sum
            }
        })
    }
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
