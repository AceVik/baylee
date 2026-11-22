//! Shared non-mana spell costs for offers, choices, and payment.
use super::{CastModeKind, CostPart, ObjectId, PlayerId, SmallVec, casting};

/// Permanents eligible to pay `Bargain` (`CR 702.166a`): an artifact,
/// enchantment, or token controlled by the caster.
static BARGAIN_FODDER: baylee_cards_dsl::Filter = baylee_cards_dsl::Filter::Or(&[
    baylee_cards_dsl::Filter::ARTIFACT,
    baylee_cards_dsl::Filter::ENCHANTMENT,
    baylee_cards_dsl::Filter::IsToken,
]);

static BARGAIN_PARTS: [CostPart; 1] = [CostPart::Sacrifice(&BARGAIN_FODDER)];

static CONSPIRE_CREATURE: baylee_cards_dsl::Filter = baylee_cards_dsl::Filter::And(&[
    baylee_cards_dsl::Filter::CREATURE,
    baylee_cards_dsl::Filter::SharesColorWithSource,
]);

static CONSPIRE_PARTS: [CostPart; 1] = [CostPart::TapN {
    count: 2,
    filter: &CONSPIRE_CREATURE,
}];

const CASUALTY_PARTS: [[CostPart; 1]; 256] = {
    let mut out = [[CostPart::Sacrifice(&baylee_cards_dsl::Filter::CREATURE)]; 256];
    let mut i = 0_usize;
    while i < 256 {
        #[allow(clippy::cast_possible_truncation)]
        {
            out[i] = [CostPart::Sacrifice(
                baylee_cards_dsl::casualty_creature_filter(i as u8),
            )];
        }
        i += 1;
    }
    out
};

fn expand_either_part(
    part: &'static CostPart,
    either_idx: &mut u8,
    choice: CostChoice,
) -> &'static [CostPart] {
    if let CostPart::Either(a, b) = part {
        let idx = *either_idx;
        *either_idx = either_idx.saturating_add(1);
        if idx < choice.either_decided {
            if idx < 8 && (choice.either_first_mask & (1_u8 << idx)) != 0 {
                a.parts
            } else {
                b.parts
            }
        } else {
            core::slice::from_ref(part)
        }
    } else {
        core::slice::from_ref(part)
    }
}

fn raw_mandatory_parts(
    face: &baylee_cards_dsl::FaceDef,
    choice: CostChoice,
) -> impl Iterator<Item = &'static CostPart> {
    let disguised = choice.option == Some(CastModeKind::Disguise);
    let mandatory: &'static [CostPart] = if disguised {
        &[]
    } else {
        face.mandatory_additional_costs
    };
    let route_parts: &'static [CostPart] = if disguised {
        &[]
    } else {
        choice.route_extra_parts
    };
    let entwine_parts: &'static [CostPart] = match (choice.option, face.entwine) {
        (Some(CastModeKind::Modes(_)), Some(entwine)) if !disguised => entwine.parts,
        _ => &[],
    };
    let jump_start_parts: &'static [CostPart] = if choice.option == Some(CastModeKind::JumpStart) {
        &[CostPart::Discard(&baylee_cards_dsl::Filter::Any)]
    } else {
        &[]
    };
    let retrace_parts: &'static [CostPart] = if choice.option == Some(CastModeKind::Retrace) {
        &[CostPart::Discard(&baylee_cards_dsl::Filter::LAND)]
    } else {
        &[]
    };
    let bargain_parts: &'static [CostPart] = if !disguised && choice.bargained && face.bargain {
        &BARGAIN_PARTS
    } else {
        &[]
    };
    let optional_parts: &'static [CostPart] = match face.optional_cost {
        Some(cost)
            if !disguised
                && (choice.optional_cost_paid
                    || choice.option == Some(CastModeKind::OptionalCost)) =>
        {
            cost.parts
        }
        _ => &[],
    };
    let offspring_parts: &'static [CostPart] = match face.offspring {
        Some(cost) if !disguised && choice.offspring => cost.parts,
        _ => &[],
    };
    let casualty_parts: &'static [CostPart] = match face.casualty {
        Some(min_power) if !disguised && choice.casualty => {
            &CASUALTY_PARTS[usize::from(min_power)][..]
        }
        _ => &[],
    };
    let squad_parts: &'static [CostPart] = match face.squad {
        Some(cost) if !disguised && choice.squad > 0 => cost.parts,
        _ => &[],
    };
    let additional_costs = face.additional_costs;
    mandatory
        .iter()
        .chain(route_parts.iter())
        .chain(jump_start_parts.iter())
        .chain(retrace_parts.iter())
        .chain(entwine_parts.iter())
        .chain(bargain_parts.iter())
        .chain(optional_parts.iter())
        .chain(offspring_parts.iter())
        .chain(casualty_parts.iter())
        .chain(std::iter::repeat_n(squad_parts, usize::from(choice.squad)).flatten())
        .chain(
            std::iter::repeat_n(
                if disguised {
                    &[][..]
                } else {
                    &CONSPIRE_PARTS[..]
                },
                usize::from(choice.conspired),
            )
            .flatten(),
        )
        .chain(
            choice
                .splice_parts
                .into_iter()
                .flatten()
                .flat_map(move |parts| if disguised { &[][..] } else { parts }),
        )
        .chain(
            additional_costs
                .iter()
                .enumerate()
                .filter(move |(idx, _)| {
                    !disguised
                        && choice.kicked
                        && (choice.kicked_mask == 0
                            || (*idx < 8 && (choice.kicked_mask & (1_u8 << *idx)) != 0))
                })
                .flat_map(|(_, cost)| cost.parts),
        )
}

/// Mandatory additional parts, entwine, bargain, casualty, and the chosen kicker's non-mana parts,
/// expanding any settled [`CostPart::Either`] branches.
pub(crate) fn wizard_mandatory_parts(
    face: &baylee_cards_dsl::FaceDef,
    choice: impl Into<CostChoice>,
) -> impl Iterator<Item = &'static CostPart> {
    let choice = choice.into();
    let alt_eithers = wizard_alternative_parts_for_choice(face, choice)
        .iter()
        .filter(|p| matches!(p, CostPart::Either(_, _)))
        .count();
    let mut idx = u8::try_from(alt_eithers).unwrap_or(u8::MAX);
    raw_mandatory_parts(face, choice)
        .flat_map(move |part| expand_either_part(part, &mut idx, choice))
}

/// Alternative non-mana parts with settled [`CostPart::Either`] branches expanded.
pub(crate) fn wizard_alternative_parts_expanded(
    face: &baylee_cards_dsl::FaceDef,
    choice: impl Into<CostChoice>,
) -> SmallVec<[CostPart; 4]> {
    let choice = choice.into();
    let mut idx = 0_u8;
    wizard_alternative_parts_for_choice(face, choice)
        .iter()
        .flat_map(|part| expand_either_part(part, &mut idx, choice))
        .copied()
        .collect()
}

/// Buyback non-mana parts with settled [`CostPart::Either`] branches expanded.
pub(crate) fn wizard_buyback_parts_expanded(
    face: &baylee_cards_dsl::FaceDef,
    choice: impl Into<CostChoice>,
) -> SmallVec<[CostPart; 4]> {
    let choice = choice.into();
    let prior = wizard_alternative_parts_for_choice(face, choice)
        .iter()
        .chain(raw_mandatory_parts(face, choice))
        .filter(|p| matches!(p, CostPart::Either(_, _)))
        .count();
    let mut idx = u8::try_from(prior).unwrap_or(u8::MAX);
    wizard_buyback_parts(face, choice.buyback)
        .iter()
        .flat_map(|part| expand_either_part(part, &mut idx, choice))
        .copied()
        .collect()
}

/// Total `CostPart::Either` count across the cast's cost list and the next unsettled pair.
pub(crate) fn next_either_cost(
    face: &baylee_cards_dsl::FaceDef,
    choice: CostChoice,
) -> (
    u8,
    Option<(
        &'static baylee_cards_dsl::Cost,
        &'static baylee_cards_dsl::Cost,
    )>,
) {
    let mut total = 0_u8;
    let mut next = None;
    for part in wizard_alternative_parts_for_choice(face, choice)
        .iter()
        .chain(raw_mandatory_parts(face, choice))
        .chain(wizard_buyback_parts(face, choice.buyback))
    {
        if let CostPart::Either(a, b) = part {
            if total == choice.either_decided && next.is_none() {
                next = Some((*a, *b));
            }
            total = total.saturating_add(1);
        }
    }
    (total, next)
}

/// Combined mana added by all settled [`CostPart::Either`] branches of `choice`.
pub(crate) fn settled_either_mana(
    face: &baylee_cards_dsl::FaceDef,
    choice: CostChoice,
) -> baylee_core::mana::ManaCost {
    let mut mana = baylee_core::mana::ManaCost::ZERO;
    let mut idx = 0_u8;
    for part in wizard_alternative_parts_for_choice(face, choice)
        .iter()
        .chain(raw_mandatory_parts(face, choice))
        .chain(wizard_buyback_parts(face, choice.buyback))
    {
        if let CostPart::Either(a, b) = part {
            if idx < choice.either_decided {
                let chosen = if idx < 8 && (choice.either_first_mask & (1_u8 << idx)) != 0 {
                    a.mana
                } else {
                    b.mana
                };
                mana = mana.combine(&chosen);
            }
            idx = idx.saturating_add(1);
        }
    }
    mana
}

/// The non-mana parts of the alternative cost chosen for this cast.
/// Flashback is an alternative cost too, including one with no mana component.
pub(crate) fn wizard_alternative_parts(
    face: &baylee_cards_dsl::FaceDef,
    option: Option<CastModeKind>,
) -> &'static [CostPart] {
    match option {
        Some(CastModeKind::Alternative(index)) => face.alternative_costs[index].cost.parts,
        Some(CastModeKind::Flashback) => face.flashback_parts,
        Some(CastModeKind::Madness) => face.madness.expect("madness").parts,
        Some(CastModeKind::Bestow) => face.bestow.expect("bestow").parts,
        Some(CastModeKind::Blitz) => face.blitz.expect("blitz").parts,
        Some(CastModeKind::Warp) => face.warp.expect("warp").parts,
        _ => &[],
    }
}

/// The non-mana parts of the buyback cost, when it is being paid.
pub(crate) fn wizard_buyback_parts(
    face: &baylee_cards_dsl::FaceDef,
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
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct CostChoice {
    /// The chosen way of casting.
    pub option: Option<CastModeKind>,
    /// Alternative cost paid independently of a modal choice (CR 118.9a).
    pub alternative_cost: Option<CastModeKind>,
    /// Whether the buyback cost is paid.
    pub buyback: bool,
    /// Whether the bargain additional cost is paid (`CR 702.166a`).
    pub bargained: bool,
    /// Whether `FaceDef::optional_cost` is paid (`CR 118.8b`).
    pub optional_cost_paid: bool,
    /// Whether the offspring additional cost is paid (`CR 702.175a`).
    pub offspring: bool,
    /// Whether the casualty additional cost is paid (`CR 702.153a`).
    pub casualty: bool,
    /// How many times the squad additional cost is paid (`CR 702.157a`).
    pub squad: u8,
    /// Kicker can accompany a face or mode without replacing that choice.
    pub kicked: bool,
    /// Bitmask of accepted multi-kicker entries; zero means all when `kicked`.
    pub kicked_mask: u8,
    /// Permanent already chosen for `Emerge` (`CR 702.119c`), excluded from
    /// satisfying any other sacrifice part of the same cast.
    pub emerge_sacrifice: Option<ObjectId>,
    /// Number of `CostPart::Either` parts decided so far (`CR 118.8`).
    pub either_decided: u8,
    /// Bitmask of decided `CostPart::Either` parts (`1` = first branch, `0` = second branch).
    pub either_first_mask: u8,
    /// Creature already chosen to return to hand (`BeamMeUp` / `Sneak`),
    /// excluded from satisfying any mandatory additional cost part.
    pub returned: Option<ObjectId>,
    /// Non-mana parts of a `Sneak` cost (`CR 702.190a`) beyond the implicit
    /// `ReturnUnblockedAttacker`.
    pub sneak_parts: &'static [CostPart],
    /// Number of conspire instances paid (`CR 702.78a–b`).
    pub conspired: u8,
    /// Non-mana parts of `AbilityDef::Awaken` when `option == Some(CastModeKind::Awaken)`.
    pub awaken_parts: &'static [CostPart],
    /// Additional non-mana cost parts imposed by `AltCastPrice::NormalPlus` (`CR 118.8`).
    pub route_extra_parts: &'static [CostPart],
    /// Non-mana parts of spliced cards in chosen order (`CR 702.47a–c`).
    pub splice_parts: [Option<&'static [CostPart]>; 8],
}

impl From<Option<CastModeKind>> for CostChoice {
    fn from(option: Option<CastModeKind>) -> Self {
        Self {
            option,
            alternative_cost: None,
            buyback: false,
            bargained: false,
            optional_cost_paid: option == Some(CastModeKind::OptionalCost),
            offspring: false,
            casualty: false,
            squad: 0,
            kicked: matches!(
                option,
                Some(
                    CastModeKind::Kicked
                        | CastModeKind::LibraryTopLife { kicked: true, .. }
                        | CastModeKind::EscapeGranted { kicked: true, .. }
                )
            ),
            kicked_mask: 0,
            emerge_sacrifice: None,
            either_decided: 0,
            either_first_mask: 0,
            returned: None,
            sneak_parts: &[],
            conspired: 0,
            awaken_parts: &[],
            route_extra_parts: &[],
            splice_parts: [None; 8],
        }
    }
}

pub(crate) fn wizard_alternative_parts_for_choice(
    face: &baylee_cards_dsl::FaceDef,
    choice: CostChoice,
) -> &'static [CostPart] {
    let opt = choice.alternative_cost.or(choice.option);
    if opt == Some(CastModeKind::Sneak) {
        choice.sneak_parts
    } else if opt == Some(CastModeKind::Awaken) {
        choice.awaken_parts
    } else {
        wizard_alternative_parts(face, opt)
    }
}

/// Every non-mana cost part the selected way of casting must pay. Alternative
/// parts precede mandatory additional parts and those precede buyback's,
/// matching the payment below.
pub(crate) fn wizard_cost_parts(
    face: &baylee_cards_dsl::FaceDef,
    choice: impl Into<CostChoice>,
) -> impl Iterator<Item = &'static CostPart> {
    let choice = choice.into();
    let mut idx = 0_u8;
    wizard_alternative_parts_for_choice(face, choice)
        .iter()
        .chain(raw_mandatory_parts(face, choice))
        .chain(wizard_buyback_parts(face, choice.buyback))
        .flat_map(move |part| expand_either_part(part, &mut idx, choice))
}

/// Fixed life owed by every selected non-mana cost, before payment begins.
pub(crate) fn fixed_life_cost(
    face: &baylee_cards_dsl::FaceDef,
    choice: impl Into<CostChoice>,
) -> i32 {
    wizard_cost_parts(face, choice).fold(0_i32, |sum, part| match part {
        CostPart::PayLife(life) => sum.saturating_add(i32::from(*life)),
        _ => sum,
    })
}

/// Fixed energy owed by every selected non-mana cost, before payment begins.
pub(crate) fn fixed_energy_cost(
    face: &baylee_cards_dsl::FaceDef,
    choice: impl Into<CostChoice>,
) -> u32 {
    wizard_cost_parts(face, choice).fold(0_u32, |sum, part| match part {
        CostPart::PayEnergy(n) => sum.saturating_add(u32::from(*n)),
        _ => sum,
    })
}

/// Whether a mandatory additional cost part scales with X.
pub(crate) fn mandatory_scales_with_x(face: &baylee_cards_dsl::FaceDef) -> bool {
    face.mandatory_additional_costs
        .iter()
        .any(part_scales_with_x)
}

pub(crate) fn part_scales_with_x(part: &CostPart) -> bool {
    match part {
        CostPart::PayLifeX
        | CostPart::PayEnergyX
        | CostPart::SacrificeX(_)
        | CostPart::TapX(_)
        | CostPart::DiscardX(_)
        | CostPart::ExileFromGraveyardX(_)
        | CostPart::RemoveAnyCounterX { .. } => true,
        CostPart::Either(a, b) => {
            a.parts.iter().any(part_scales_with_x) || b.parts.iter().any(part_scales_with_x)
        }
        _ => false,
    }
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
/// into N questions (and `SacrificeX` / `TapX` into X questions) so offer,
/// choice and payment share one answer per object. Externally imposed parts
/// follow the card's printed and selected alternative parts.
pub(crate) fn additional_card_costs_for_x(
    state: &crate::state::GameState,
    player: PlayerId,
    card: ObjectId,
    face: &'static baylee_cards_dsl::FaceDef,
    choice: impl Into<CostChoice>,
    x: u32,
) -> SmallVec<[CostPart; 4]> {
    let choice = choice.into();
    let alt_expanded = wizard_alternative_parts_expanded(face, choice);
    let buyback_expanded = wizard_buyback_parts_expanded(face, choice);
    let alt_iter = alt_expanded.into_iter().filter(|p| {
        matches!(
            p,
            CostPart::Sacrifice(_)
                | CostPart::ReturnToHand(_)
                | CostPart::TapOther(_)
                | CostPart::TapN { .. }
                | CostPart::Forage
                | CostPart::Behold(_)
        )
    });
    let add_iter = wizard_mandatory_parts(face, choice)
        .copied()
        .chain(buyback_expanded)
        .filter(|p| {
            matches!(
                p,
                CostPart::Sacrifice(_)
                    | CostPart::SacrificeX(_)
                    | CostPart::ExilePermanent(_)
                    | CostPart::ExileFromHand(_)
                    | CostPart::ExileFromGraveyard(_)
                    | CostPart::ExileFromGraveyardX(_)
                    | CostPart::Discard(_)
                    | CostPart::DiscardX(_)
                    | CostPart::ReturnToHand(_)
                    | CostPart::TapOther(_)
                    | CostPart::TapX(_)
                    | CostPart::TapN { .. }
                    | CostPart::Forage
                    | CostPart::Behold(_)
            )
        });
    let mut parts: SmallVec<[CostPart; 4]> = alt_iter
        .chain(add_iter)
        .flat_map(|part| {
            let count = match &part {
                CostPart::TapN { count, .. } => usize::try_from(*count).unwrap_or(usize::MAX),
                CostPart::SacrificeX(_)
                | CostPart::TapX(_)
                | CostPart::DiscardX(_)
                | CostPart::ExileFromGraveyardX(_) => usize::try_from(x).unwrap_or(usize::MAX),
                _ => 1,
            };
            std::iter::repeat_n(part, count)
        })
        .collect();
    parts.extend(casting::additional_spell_costs(state, player, card, face));
    parts
}
