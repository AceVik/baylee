//! What a filter or modifier reads beyond characteristics: board state, a reference, combat, other zones.

use baylee_cards_dsl::CardDef;

/// Whether a DSL filter reads **board state** rather than a characteristic:
/// whether an object is tapped, or whether it is attacking.
///
/// The sibling of [`filter_reaches_other_zones`], and it exists for the same
/// reason: the projection cache is keyed on the *effect* generation, so an
/// input that is not an effect has to announce itself. These two are such
/// inputs — `Filter::Untapped` is Spectral Cloak's whole sentence
/// ("enchanted creature has shroud as long as it's untapped"), `Filter::
/// Attacking` is Orcish Oriflamme's — and a board where nothing reads them
/// must not pay for the announcement, because tapping a land is the most
/// frequent thing that happens in a game.
///
/// `Filter::EnteredThisTurn` reads the turn boundary. Arrival and `begin_turn`
/// both invalidate projections independently of any trigger resolving.
///
/// `Filter::IsAttached` is not here for a plainer reason: every write of
/// `attached_to` invalidates the projection itself — the attach in
/// `resolve`, the unattach in `sba`, and the zone move that clears it.
pub(crate) fn filter_reads_board_state(filter: &baylee_cards_dsl::Filter) -> bool {
    use baylee_cards_dsl::Filter;
    match filter {
        Filter::EnteredThisTurn
        | Filter::Tapped
        | Filter::Untapped
        | Filter::Attacking
        | Filter::Blocking
        | Filter::Blocked
        | Filter::BlockingSource
        | Filter::BlockedBySource
        | Filter::BlockingReference
        | Filter::BlockedByReference
        | Filter::Unblocked
        | Filter::SharesAColorWith(_)
        | Filter::SharesColorWithExiledWithSource => true,
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(filter_reads_board_state),
        Filter::Not(f) | Filter::ControllerControls(f) => filter_reads_board_state(f),
        _ => false,
    }
}

/// Whether a filter is relative to a resolution's reference object
/// (`Filter::Reference` and its family), which only a resolution or a target
/// choice can supply.
pub(crate) fn filter_reads_reference(filter: &baylee_cards_dsl::Filter) -> bool {
    use baylee_cards_dsl::Filter;
    match filter {
        Filter::Reference
        | Filter::BlockingReference
        | Filter::BlockedByReference
        | Filter::BlockedByReferenceThisTurn
        | Filter::BlockedReferenceThisTurn
        | Filter::NotControlledByReferenceController
        | Filter::SharesCreatureTypeWithOtherTarget
        | Filter::SharesTypeWithReference(_) => true,
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(filter_reads_reference),
        Filter::Not(f)
        | Filter::AttachedTo(f)
        | Filter::BlockedThisTurnBy(f)
        | Filter::LowestManaValueOnBattlefield(f)
        | Filter::SharesAColorWith(f)
        | Filter::ControllerControls(f) => filter_reads_reference(f),
        _ => false,
    }
}

/// Whether a modifier's number reads combat: a count of what the defending
/// player controls (`PtCount::DefendingPlayerControls`, CR 508.5), which
/// changes as an attack is declared and as combat ends while no effect
/// begins or ends — the input [`filter_reads_board_state`] announces for a
/// filter, here in the modifier.
pub(crate) fn modifier_reads_combat(modifier: baylee_cards_dsl::Modifier) -> bool {
    use baylee_cards_dsl::Modifier;
    if matches!(modifier, Modifier::ModifyPTByAmount { .. }) {
        return true;
    }
    if let Modifier::ModifyPTPerCount { filter, .. } = modifier {
        return filter_reads_board_state(filter);
    }
    if let Modifier::CharacteristicPTSeparate { power, toughness } = modifier {
        return pt_count_reads_combat(power) || pt_count_reads_combat(toughness);
    }
    let (Modifier::CharacteristicPT { count, .. }
    | Modifier::SetPTToCount(count)
    | Modifier::ModifyPTByCount(count)
    | Modifier::ReducePTByCount { count, .. }
    | Modifier::ModifyPTHalfCount(count)) = modifier
    else {
        return false;
    };
    pt_count_reads_combat(count)
}

fn pt_count_reads_combat(count: baylee_cards_dsl::PtCount) -> bool {
    use baylee_cards_dsl::PtCount;
    match count {
        PtCount::Plus { base, .. } | PtCount::Complement { count: base, .. } => {
            pt_count_reads_combat(*base)
        }
        PtCount::DefendingPlayerControls(_) | PtCount::ControllerOrDefenderControls(_) => true,
        PtCount::DistinctColorsAmong(f)
        | PtCount::YouControl(f)
        | PtCount::OnBattlefield(f)
        | PtCount::ChosenPlayerControls(f) => filter_reads_board_state(f),
        PtCount::CardTypesInAllGraveyards
        | PtCount::InAllGraveyards(_)
        | PtCount::ExiledWithThis
        | PtCount::CountersOnThis(_)
        | PtCount::EntrySacrifices
        | PtCount::ChosenNumber
        | PtCount::Fixed(_) => false,
    }
}

/// Whether a DSL filter mentions non-battlefield zones (then its effect
/// needs cross-zone projection).
/// The characteristic-defining power and toughness a card's front face
/// prints (CR 604.3a): a static on the card itself, unconditional, that
/// defines P/T in layer 7a. Front face only, because a card off the
/// battlefield and the stack has only its front face's characteristics
/// (CR 712.8a).
pub(super) fn printed_pt_cda(def: &CardDef) -> Option<baylee_cards_dsl::Modifier> {
    def.abilities_for_face(0)
        .iter()
        .find_map(|ability| match ability {
            baylee_cards_dsl::AbilityDef::Static(sa)
                if sa.filter == baylee_cards_dsl::Filter::This
                    && sa.condition.is_none()
                    && matches!(
                        sa.modifier,
                        baylee_cards_dsl::Modifier::CharacteristicPT { .. }
                            | baylee_cards_dsl::Modifier::CharacteristicPTSeparate { .. }
                    ) =>
            {
                Some(sa.modifier)
            }
            _ => None,
        })
}

pub(crate) fn filter_reaches_other_zones(filter: &baylee_cards_dsl::Filter) -> bool {
    use baylee_cards_dsl::{Filter, ZoneRef};
    match filter {
        Filter::InZone(z) => !matches!(z, ZoneRef::Battlefield),
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(filter_reaches_other_zones),
        Filter::Not(f) => filter_reaches_other_zones(f),
        _ => false,
    }
}
