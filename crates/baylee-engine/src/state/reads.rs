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
        | Filter::FaceDown
        | Filter::Monstrous
        | Filter::Suspected
        | Filter::RingBearer
        | Filter::Modified
        | Filter::SharesAColorWith(_)
        | Filter::SharesCreatureTypeWith(_)
        | Filter::SharesColorWithExiledWithSource
        | Filter::SharesColorWithSource
        | Filter::CmcAtMostAmount(_)
        | Filter::CmcEqualsAmount(_)
        | Filter::PowerAtMostAmount(_)
        | Filter::PowerAtLeastAmount(_) => true,
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(filter_reads_board_state),
        Filter::Not(f) | Filter::ControllerControls(f) => filter_reads_board_state(f),
        _ => false,
    }
}

/// Whether a filter is relative to a resolution's reference object
/// (`Filter::Reference` and its family), which only a resolution or a target
/// choice can supply.
pub(crate) fn filter_reads_reference(filter: &baylee_cards_dsl::Filter) -> bool {
    filter_reads_reference_at(filter, 0)
}

fn filter_reads_reference_at(filter: &baylee_cards_dsl::Filter, depth: u8) -> bool {
    use baylee_cards_dsl::Filter;
    if depth >= 8 {
        return false;
    }
    match filter {
        Filter::Reference
        | Filter::BlockingReference
        | Filter::BlockedByReference
        | Filter::BlockedByReferenceThisTurn
        | Filter::BlockedReferenceThisTurn
        | Filter::NotControlledByReferenceController
        | Filter::SharesCreatureTypeWithOtherTarget
        | Filter::SharesTypeWithReference(_) => true,
        Filter::CmcAtMostAmount(amt)
        | Filter::CmcEqualsAmount(amt)
        | Filter::PowerAtMostAmount(amt)
        | Filter::PowerAtLeastAmount(amt) => amount_reads_reference_at(amt, depth + 1),
        Filter::And(parts) | Filter::Or(parts) => {
            parts.iter().any(|f| filter_reads_reference_at(f, depth))
        }
        Filter::Not(f)
        | Filter::AttachedTo(f)
        | Filter::BlockedThisTurnBy(f)
        | Filter::LowestManaValueOnBattlefield(f)
        | Filter::SharesAColorWith(f)
        | Filter::SharesCreatureTypeWith(f)
        | Filter::ControllerControls(f) => filter_reads_reference_at(f, depth),
        _ => false,
    }
}

fn amount_reads_reference_at(amount: &baylee_cards_dsl::Amount, depth: u8) -> bool {
    use baylee_cards_dsl::Amount;
    if depth >= 8 {
        return false;
    }
    match amount {
        Amount::DistinctColorsAmong(f)
        | Amount::BasicLandTypesAmong(f)
        | Amount::CountOf { filter: f, .. }
        | Amount::ControlledCount { filter: f, .. }
        | Amount::DamageThisTurn { by: f, .. }
        | Amount::GreatestPowerAmong(f)
        | Amount::GreatestManaValueAmong(f) => filter_reads_reference_at(f, depth),
        Amount::Half { base: a, .. }
        | Amount::Divided { base: a, .. }
        | Amount::Times { base: a, .. }
        | Amount::Plus { base: a, .. }
        | Amount::Negated(a)
        | Amount::SaturatingSub { base: a, .. }
        | Amount::ConstantMinus { subtract: a, .. } => amount_reads_reference_at(a, depth + 1),
        Amount::Sum { left, right } | Amount::Difference { left, right } => {
            amount_reads_reference_at(left, depth + 1)
                || amount_reads_reference_at(right, depth + 1)
        }
        Amount::IfCondition {
            then, otherwise, ..
        } => {
            amount_reads_reference_at(then, depth + 1)
                || amount_reads_reference_at(otherwise, depth + 1)
        }
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
    if let Modifier::ModifyPTPerCount { filter, .. }
    | Modifier::GainActivatedAbilitiesOf { filter, .. } = modifier
    {
        return filter_reads_board_state(filter);
    }
    if let Modifier::CharacteristicPTSeparate { power, toughness }
    | Modifier::SetPTSeparateToCount { power, toughness } = modifier
    {
        return pt_count_reads_combat(power) || pt_count_reads_combat(toughness);
    }
    let (Modifier::CharacteristicPT { count, .. }
    | Modifier::SetPTToCount(count)
    | Modifier::SetPowerToCount(count)
    | Modifier::SetToughnessToCount(count)
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
        PtCount::Plus { base, .. }
        | PtCount::Complement { count: base, .. }
        | PtCount::Twice(base) => pt_count_reads_combat(*base),
        PtCount::DefendingPlayerControls(_) | PtCount::ControllerOrDefenderControls(_) => true,
        PtCount::DistinctColorsAmong(f)
        | PtCount::DistinctNamesAmong(f)
        | PtCount::GreatestManaValueAmong(f)
        | PtCount::YouControl(f)
        | PtCount::OnBattlefield(f)
        | PtCount::ChosenPlayerControls(f) => filter_reads_board_state(f),
        PtCount::CardsInHand(who)
        | PtCount::InGraveyards { who, .. }
        | PtCount::Domain(who)
        | PtCount::LifeTotal(who)
        | PtCount::PartySize(who) => matches!(who, baylee_cards_dsl::PlayerRel::DefendingPlayer),
        PtCount::CardTypesInAllGraveyards
        | PtCount::CardTypesInOpponentsGraveyards
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
/// (CR 712.6a).
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

pub(crate) fn card_reads_crime(def: &CardDef) -> bool {
    (0..def.faces.len().max(1))
        .any(|face| def.abilities_for_face(face).iter().any(ability_reads_crime))
}

pub(crate) fn ability_reads_crime(ability: &baylee_cards_dsl::AbilityDef) -> bool {
    use baylee_cards_dsl::{AbilityDef, Trigger};
    match ability {
        AbilityDef::Triggered {
            trigger,
            effects,
            condition,
            ..
        } => {
            matches!(trigger, Trigger::CommittedCrime { .. })
                || condition.as_ref().is_some_and(condition_reads_crime)
                || effects_read_crime(effects)
        }
        AbilityDef::ModalTriggered {
            trigger,
            modes,
            condition,
            ..
        } => {
            matches!(trigger, Trigger::CommittedCrime { .. })
                || condition.as_ref().is_some_and(condition_reads_crime)
                || modes.iter().any(|m| effects_read_crime(m.effects))
        }
        AbilityDef::Spell {
            effects, condition, ..
        } => condition.as_ref().is_some_and(condition_reads_crime) || effects_read_crime(effects),
        AbilityDef::Activated {
            effects,
            cost_reduction,
            ..
        } => cost_reduction_reads_crime(cost_reduction.as_ref()) || effects_read_crime(effects),
        AbilityDef::ActivatedConditional {
            effects,
            condition,
            cost_reduction,
            ..
        } => {
            condition_reads_crime(condition)
                || cost_reduction_reads_crime(cost_reduction.as_ref())
                || effects_read_crime(effects)
        }
        AbilityDef::ModalSpell { modes, .. }
        | AbilityDef::Modal { modes, .. }
        | AbilityDef::ModalActivated { modes, .. } => {
            modes.iter().any(|m| effects_read_crime(m.effects))
        }
        AbilityDef::SagaChapter { effects, .. } | AbilityDef::Loyalty { effects, .. } => {
            effects_read_crime(effects)
        }
        AbilityDef::Static(sa) => {
            sa.condition.as_ref().is_some_and(condition_reads_crime)
                || filter_reads_crime(&sa.filter, 0)
                || modifier_reads_crime(&sa.modifier)
        }
        AbilityDef::Replacement(
            baylee_cards_dsl::ReplacementRule::AdditionalTokenCreation {
                condition: Some(c), ..
            }
            | baylee_cards_dsl::ReplacementRule::SubstituteTokenCreation {
                condition: Some(c), ..
            }
            | baylee_cards_dsl::ReplacementRule::ReplaceDrawWithDraws {
                condition: Some(c), ..
            },
        ) => condition_reads_crime(c),
        _ => false,
    }
}

fn cost_reduction_reads_crime(cr: Option<&baylee_cards_dsl::CostReduction>) -> bool {
    matches!(
        cr,
        Some(baylee_cards_dsl::CostReduction::If { condition, .. })
            if condition_reads_crime(condition)
    )
}

fn condition_reads_crime(cond: &baylee_cards_dsl::Condition) -> bool {
    condition_reads_crime_at(cond, 0)
}

fn condition_reads_crime_at(cond: &baylee_cards_dsl::Condition, depth: u8) -> bool {
    use baylee_cards_dsl::Condition;
    if depth >= 8 {
        return false;
    }
    match cond {
        Condition::CommittedCrimeThisTurn(_) => true,
        Condition::AmountAtLeast(amount, _) | Condition::AmountIsEven(amount) => {
            amount_reads_crime_at(amount, depth + 1)
        }
        Condition::AmountCompare { left, right, .. } => {
            amount_reads_crime_at(left, depth + 1) || amount_reads_crime_at(right, depth + 1)
        }
        Condition::ControlCount(f, _)
        | Condition::ControlCountExceptEvent(f, _)
        | Condition::ControlCountAtMost(f, _)
        | Condition::BattlefieldCount(f, _)
        | Condition::BattlefieldCountAtMost(f, _)
        | Condition::ControlDistinctNames(f, _)
        | Condition::OpponentControlCount(f, _)
        | Condition::OpponentControlCountAtMost(f, _)
        | Condition::OpponentControlsMore(f)
        | Condition::OpponentControlsMoreBy(f, _)
        | Condition::ControlMoreThanEachOtherPlayer(f)
        | Condition::SomeDefendingPlayerControlsNone(f)
        | Condition::ChosenOpponentControls(f)
        | Condition::ChosenColorIsUniqueMostCommon(f)
        | Condition::TargetSpellWouldDestroy(f)
        | Condition::GraveyardCardsAbove(f, _)
        | Condition::SourceMatches(f)
        | Condition::AttachedMatches(f)
        | Condition::CanSacrifice(f)
        | Condition::SacrificedForCost(f)
        | Condition::DiscardedForCost(f)
        | Condition::ObjectMatches { filter: f, .. } => filter_reads_crime(f, depth + 1),
        Condition::EventBlockedByOthers { some, none } => {
            filter_reads_crime(some, depth + 1) || filter_reads_crime(none, depth + 1)
        }
        Condition::Not(inner) | Condition::TriggerOnly(inner) => {
            condition_reads_crime_at(inner, depth + 1)
        }
        Condition::All(parts) | Condition::Any(parts) => {
            parts.iter().any(|p| condition_reads_crime_at(p, depth + 1))
        }
        _ => false,
    }
}

fn filter_reads_crime(filter: &baylee_cards_dsl::Filter, depth: u8) -> bool {
    use baylee_cards_dsl::Filter;
    if depth >= 8 {
        return false;
    }
    match filter {
        Filter::CmcAtMostAmount(a)
        | Filter::CmcEqualsAmount(a)
        | Filter::PowerAtMostAmount(a)
        | Filter::PowerAtLeastAmount(a) => amount_reads_crime_at(a, depth + 1),
        Filter::And(parts) | Filter::Or(parts) => {
            parts.iter().any(|f| filter_reads_crime(f, depth))
        }
        Filter::Not(f)
        | Filter::AttachedTo(f)
        | Filter::BlockedThisTurnBy(f)
        | Filter::LowestManaValueOnBattlefield(f)
        | Filter::SharesAColorWith(f)
        | Filter::SharesCreatureTypeWith(f)
        | Filter::ControllerControls(f) => filter_reads_crime(f, depth),
        _ => false,
    }
}

fn amount_reads_crime_at(amount: &baylee_cards_dsl::Amount, depth: u8) -> bool {
    use baylee_cards_dsl::Amount;
    if depth >= 8 {
        return false;
    }
    match amount {
        Amount::IfCondition {
            condition,
            then,
            otherwise,
        } => {
            condition_reads_crime_at(condition, depth + 1)
                || amount_reads_crime_at(then, depth + 1)
                || amount_reads_crime_at(otherwise, depth + 1)
        }
        Amount::DistinctColorsAmong(f)
        | Amount::BasicLandTypesAmong(f)
        | Amount::CountOf { filter: f, .. }
        | Amount::ControlledCount { filter: f, .. }
        | Amount::DamageThisTurn { by: f, .. }
        | Amount::GreatestPowerAmong(f)
        | Amount::GreatestManaValueAmong(f) => filter_reads_crime(f, depth + 1),
        Amount::Negated(a)
        | Amount::Plus { base: a, .. }
        | Amount::SaturatingSub { base: a, .. }
        | Amount::ConstantMinus { subtract: a, .. }
        | Amount::Half { base: a, .. }
        | Amount::Divided { base: a, .. }
        | Amount::Times { base: a, .. } => amount_reads_crime_at(a, depth + 1),
        Amount::Sum { left: a, right: b } | Amount::Difference { left: a, right: b } => {
            amount_reads_crime_at(a, depth + 1) || amount_reads_crime_at(b, depth + 1)
        }
        _ => false,
    }
}

fn modifier_reads_crime(modifier: &baylee_cards_dsl::Modifier) -> bool {
    use baylee_cards_dsl::Modifier;
    match modifier {
        Modifier::GrantTriggered {
            trigger, effects, ..
        } => {
            matches!(trigger, baylee_cards_dsl::Trigger::CommittedCrime { .. })
                || effects_read_crime(effects)
        }
        Modifier::GrantTriggeredDefinition(ability) => ability_reads_crime(ability),
        Modifier::GrantActivated { effects, .. } => effects_read_crime(effects),
        Modifier::GrantFlash { filter, .. } => filter_reads_crime(filter, 0),
        Modifier::MayCastWithAltCost {
            max_mana_value: Some(amt),
            ..
        } => amount_reads_crime_at(amt, 0),
        Modifier::SpellsCostLessByAmount(amount) => amount_reads_crime_at(amount, 0),
        Modifier::AdjustSpellCostByAmount { amount, .. } => amount_reads_crime_at(amount, 0),
        _ => false,
    }
}

fn effects_read_crime(effects: &'static [baylee_cards_dsl::Effect]) -> bool {
    let mut seen = 0;
    let mut found = false;
    baylee_cards_dsl::Effect::walk(effects, &mut seen, &mut |effect| {
        use baylee_cards_dsl::{DelayedTriggerWhen, Effect, Trigger};
        found |= match effect {
            Effect::IfCondition { condition, .. } | Effect::RepeatWhile { condition, .. } => {
                condition_reads_crime(condition)
            }
            Effect::RepeatProcess {
                pre_condition,
                post_condition,
                ..
            } => {
                pre_condition.as_ref().is_some_and(condition_reads_crime)
                    || post_condition.as_ref().is_some_and(condition_reads_crime)
            }
            Effect::ReflexiveTrigger { condition, .. } => {
                condition.is_some_and(condition_reads_crime)
            }
            Effect::RegisterDelayedTrigger {
                when, condition, ..
            } => {
                matches!(
                    when,
                    DelayedTriggerWhen::OnTrigger {
                        trigger: Trigger::CommittedCrime { .. },
                        ..
                    }
                ) || condition.is_some_and(condition_reads_crime)
            }
            Effect::CreateEmblem { abilities } => abilities.iter().any(ability_reads_crime),
            Effect::Backup {
                grant_abilities, ..
            } => grant_abilities.iter().any(ability_reads_crime),
            Effect::MoveAndModify { modifiers, .. } => modifiers.iter().any(modifier_reads_crime),
            _ => false,
        };
    });
    found
}

pub(crate) fn card_reads_expend(def: &CardDef) -> bool {
    (0..def.faces.len().max(1)).any(|face| {
        def.abilities_for_face(face)
            .iter()
            .any(ability_reads_expend)
    })
}

pub(crate) fn ability_reads_expend(ability: &baylee_cards_dsl::AbilityDef) -> bool {
    use baylee_cards_dsl::{AbilityDef, Trigger};
    match ability {
        AbilityDef::Triggered {
            trigger,
            effects,
            condition,
            ..
        } => {
            matches!(trigger, Trigger::Expended { .. })
                || condition.as_ref().is_some_and(condition_reads_expend)
                || effects_read_expend(effects)
        }
        AbilityDef::ModalTriggered {
            trigger,
            modes,
            condition,
            ..
        } => {
            matches!(trigger, Trigger::Expended { .. })
                || condition.as_ref().is_some_and(condition_reads_expend)
                || modes.iter().any(|m| effects_read_expend(m.effects))
        }
        AbilityDef::Spell {
            effects, condition, ..
        } => condition.as_ref().is_some_and(condition_reads_expend) || effects_read_expend(effects),
        AbilityDef::Activated { effects, .. } => effects_read_expend(effects),
        AbilityDef::ActivatedConditional {
            effects, condition, ..
        } => condition_reads_expend(condition) || effects_read_expend(effects),
        AbilityDef::ModalSpell { modes, .. }
        | AbilityDef::Modal { modes, .. }
        | AbilityDef::ModalActivated { modes, .. } => {
            modes.iter().any(|m| effects_read_expend(m.effects))
        }
        AbilityDef::SagaChapter { effects, .. } | AbilityDef::Loyalty { effects, .. } => {
            effects_read_expend(effects)
        }
        AbilityDef::Static(sa) => {
            sa.condition.as_ref().is_some_and(condition_reads_expend)
                || filter_reads_expend(&sa.filter, 0)
                || modifier_reads_expend(&sa.modifier)
        }
        AbilityDef::Replacement(
            baylee_cards_dsl::ReplacementRule::AdditionalTokenCreation {
                condition: Some(c), ..
            }
            | baylee_cards_dsl::ReplacementRule::SubstituteTokenCreation {
                condition: Some(c), ..
            }
            | baylee_cards_dsl::ReplacementRule::ReplaceDrawWithDraws {
                condition: Some(c), ..
            },
        ) => condition_reads_expend(c),
        _ => false,
    }
}

fn condition_reads_expend(cond: &baylee_cards_dsl::Condition) -> bool {
    condition_reads_expend_at(cond, 0)
}

fn condition_reads_expend_at(cond: &baylee_cards_dsl::Condition, depth: u8) -> bool {
    use baylee_cards_dsl::Condition;
    if depth >= 8 {
        return false;
    }
    match cond {
        Condition::ExpendedThisTurn(_) => true,
        Condition::AmountAtLeast(amount, _) | Condition::AmountIsEven(amount) => {
            amount_reads_expend_at(amount, depth + 1)
        }
        Condition::AmountCompare { left, right, .. } => {
            amount_reads_expend_at(left, depth + 1) || amount_reads_expend_at(right, depth + 1)
        }
        Condition::ControlCount(f, _)
        | Condition::ControlCountExceptEvent(f, _)
        | Condition::ControlCountAtMost(f, _)
        | Condition::BattlefieldCount(f, _)
        | Condition::BattlefieldCountAtMost(f, _)
        | Condition::ControlDistinctNames(f, _)
        | Condition::OpponentControlCount(f, _)
        | Condition::OpponentControlCountAtMost(f, _)
        | Condition::OpponentControlsMore(f)
        | Condition::OpponentControlsMoreBy(f, _)
        | Condition::ControlMoreThanEachOtherPlayer(f)
        | Condition::SomeDefendingPlayerControlsNone(f)
        | Condition::ChosenOpponentControls(f)
        | Condition::ChosenColorIsUniqueMostCommon(f)
        | Condition::TargetSpellWouldDestroy(f)
        | Condition::GraveyardCardsAbove(f, _)
        | Condition::SourceMatches(f)
        | Condition::AttachedMatches(f)
        | Condition::CanSacrifice(f)
        | Condition::SacrificedForCost(f)
        | Condition::DiscardedForCost(f)
        | Condition::ObjectMatches { filter: f, .. } => filter_reads_expend(f, depth + 1),
        Condition::EventBlockedByOthers { some, none } => {
            filter_reads_expend(some, depth + 1) || filter_reads_expend(none, depth + 1)
        }
        Condition::Not(inner) | Condition::TriggerOnly(inner) => {
            condition_reads_expend_at(inner, depth + 1)
        }
        Condition::All(parts) | Condition::Any(parts) => parts
            .iter()
            .any(|p| condition_reads_expend_at(p, depth + 1)),
        _ => false,
    }
}

fn filter_reads_expend(filter: &baylee_cards_dsl::Filter, depth: u8) -> bool {
    use baylee_cards_dsl::Filter;
    if depth >= 8 {
        return false;
    }
    match filter {
        Filter::CmcAtMostAmount(a)
        | Filter::CmcEqualsAmount(a)
        | Filter::PowerAtMostAmount(a)
        | Filter::PowerAtLeastAmount(a) => amount_reads_expend_at(a, depth + 1),
        Filter::And(parts) | Filter::Or(parts) => {
            parts.iter().any(|f| filter_reads_expend(f, depth))
        }
        Filter::Not(f)
        | Filter::AttachedTo(f)
        | Filter::BlockedThisTurnBy(f)
        | Filter::LowestManaValueOnBattlefield(f)
        | Filter::SharesAColorWith(f)
        | Filter::SharesCreatureTypeWith(f)
        | Filter::ControllerControls(f) => filter_reads_expend(f, depth),
        _ => false,
    }
}

fn amount_reads_expend(amount: &baylee_cards_dsl::Amount) -> bool {
    amount_reads_expend_at(amount, 0)
}

fn amount_reads_expend_at(amount: &baylee_cards_dsl::Amount, depth: u8) -> bool {
    use baylee_cards_dsl::Amount;
    if depth >= 8 {
        return false;
    }
    match amount {
        Amount::ManaExpendedOnSpellsThisTurn(_) => true,
        Amount::IfCondition {
            condition,
            then,
            otherwise,
        } => {
            condition_reads_expend_at(condition, depth + 1)
                || amount_reads_expend_at(then, depth + 1)
                || amount_reads_expend_at(otherwise, depth + 1)
        }
        Amount::DistinctColorsAmong(f)
        | Amount::BasicLandTypesAmong(f)
        | Amount::CountOf { filter: f, .. }
        | Amount::ControlledCount { filter: f, .. }
        | Amount::DamageThisTurn { by: f, .. }
        | Amount::GreatestPowerAmong(f)
        | Amount::GreatestManaValueAmong(f) => filter_reads_expend(f, depth + 1),
        Amount::Negated(a)
        | Amount::Plus { base: a, .. }
        | Amount::SaturatingSub { base: a, .. }
        | Amount::ConstantMinus { subtract: a, .. }
        | Amount::Half { base: a, .. }
        | Amount::Divided { base: a, .. }
        | Amount::Times { base: a, .. } => amount_reads_expend_at(a, depth + 1),
        Amount::Sum { left: a, right: b } | Amount::Difference { left: a, right: b } => {
            amount_reads_expend_at(a, depth + 1) || amount_reads_expend_at(b, depth + 1)
        }
        _ => false,
    }
}

fn modifier_reads_expend(modifier: &baylee_cards_dsl::Modifier) -> bool {
    use baylee_cards_dsl::Modifier;
    match modifier {
        Modifier::GrantTriggered {
            trigger, effects, ..
        } => {
            matches!(trigger, baylee_cards_dsl::Trigger::Expended { .. })
                || effects_read_expend(effects)
        }
        Modifier::GrantTriggeredDefinition(ability) => ability_reads_expend(ability),
        Modifier::GrantActivated { effects, .. } => effects_read_expend(effects),
        Modifier::ModifyPTByAmount { power, toughness } => {
            amount_reads_expend(power) || amount_reads_expend(toughness)
        }
        Modifier::GrantFlash { filter, .. } => filter_reads_expend(filter, 0),
        Modifier::MayCastWithAltCost {
            max_mana_value: Some(amt),
            ..
        } => amount_reads_expend_at(amt, 0),
        Modifier::SpellsCostLessByAmount(amount) => amount_reads_expend_at(amount, 0),
        Modifier::AdjustSpellCostByAmount { amount, .. } => amount_reads_expend_at(amount, 0),
        _ => false,
    }
}

fn effects_read_expend(effects: &'static [baylee_cards_dsl::Effect]) -> bool {
    let mut seen = 0;
    let mut found = false;
    baylee_cards_dsl::Effect::walk(effects, &mut seen, &mut |effect| {
        use baylee_cards_dsl::{DelayedTriggerWhen, Effect, Trigger};
        found |= match effect {
            Effect::IfCondition { condition, .. } | Effect::RepeatWhile { condition, .. } => {
                condition_reads_expend(condition)
            }
            Effect::RepeatProcess {
                max_times,
                pre_condition,
                post_condition,
                ..
            } => {
                max_times.as_ref().is_some_and(amount_reads_expend)
                    || pre_condition.as_ref().is_some_and(condition_reads_expend)
                    || post_condition.as_ref().is_some_and(condition_reads_expend)
            }
            Effect::ReflexiveTrigger { condition, .. } => {
                condition.is_some_and(condition_reads_expend)
            }
            Effect::RegisterDelayedTrigger {
                when, condition, ..
            } => {
                matches!(
                    when,
                    DelayedTriggerWhen::OnTrigger {
                        trigger: Trigger::Expended { .. },
                        ..
                    }
                ) || condition.is_some_and(condition_reads_expend)
            }
            Effect::DrawCards { amount }
            | Effect::GainLife { amount }
            | Effect::LoseLife { amount, .. }
            | Effect::DealDamage { amount, .. }
            | Effect::AddCounter { amount, .. }
            | Effect::PutCountersDivided { amount, .. }
            | Effect::PutCounterChoice { amount, .. }
            | Effect::AddCounterSource { amount, .. }
            | Effect::CreateTokenN { amount, .. }
            | Effect::NextSpellThisTurnCostsLess(amount)
            | Effect::ReduceNextSpellCost {
                generic: amount, ..
            } => amount_reads_expend(amount),
            Effect::CreateTokenSized {
                amount,
                power,
                toughness,
                ..
            } => {
                amount_reads_expend(amount)
                    || amount_reads_expend(power)
                    || amount_reads_expend(toughness)
            }
            Effect::DamagePermanentAndController {
                permanent_amount,
                controller_amount,
                ..
            } => amount_reads_expend(permanent_amount) || amount_reads_expend(controller_amount),
            Effect::PumpTarget {
                power, toughness, ..
            }
            | Effect::PumpFilter {
                power, toughness, ..
            }
            | Effect::PumpEventObject {
                power, toughness, ..
            } => amount_reads_expend(power) || amount_reads_expend(toughness),
            Effect::CreateEmblem { abilities } => abilities.iter().any(ability_reads_expend),
            Effect::Backup {
                grant_abilities, ..
            } => grant_abilities.iter().any(ability_reads_expend),
            Effect::MoveAndModify {
                counters,
                modifiers,
                ..
            } => {
                counters.iter().any(|(_, amt)| amount_reads_expend(amt))
                    || modifiers.iter().any(modifier_reads_expend)
            }
            _ => false,
        };
    });
    found
}

pub(crate) fn card_reads_discard_count(def: &CardDef) -> bool {
    card_has_spell_cast_metric(def, |metric| {
        matches!(
            metric,
            baylee_cards_dsl::SpellCastMetric::CardsDiscardedThisTurn
        )
    })
}

pub(crate) fn card_reads_sacrificed_permanents(def: &CardDef) -> bool {
    card_has_spell_cast_metric(def, |metric| {
        matches!(
            metric,
            baylee_cards_dsl::SpellCastMetric::PermanentsSacrificedThisTurn(_)
        )
    })
}

fn card_has_spell_cast_metric(
    def: &CardDef,
    predicate: impl Fn(baylee_cards_dsl::SpellCastMetric) -> bool + Copy,
) -> bool {
    (0..def.faces.len().max(1)).any(|face| {
        def.abilities_for_face(face)
            .iter()
            .any(|ability| ability_has_spell_cast_metric(ability, predicate))
    })
}

fn ability_has_spell_cast_metric(
    ability: &baylee_cards_dsl::AbilityDef,
    predicate: impl Fn(baylee_cards_dsl::SpellCastMetric) -> bool + Copy,
) -> bool {
    use baylee_cards_dsl::{AbilityDef, Modifier};
    match ability {
        AbilityDef::Static(sa) => matches!(
            sa.modifier,
            Modifier::AdjustSpellCostByCast { metric, .. } if predicate(metric)
        ),
        AbilityDef::Spell { effects, .. }
        | AbilityDef::Triggered { effects, .. }
        | AbilityDef::Activated { effects, .. }
        | AbilityDef::ActivatedConditional { effects, .. }
        | AbilityDef::SagaChapter { effects, .. }
        | AbilityDef::Loyalty { effects, .. } => effects_have_spell_cast_metric(effects, predicate),
        AbilityDef::ModalSpell { modes, .. }
        | AbilityDef::Modal { modes, .. }
        | AbilityDef::ModalActivated { modes, .. }
        | AbilityDef::ModalTriggered { modes, .. } => modes
            .iter()
            .any(|m| effects_have_spell_cast_metric(m.effects, predicate)),
        _ => false,
    }
}

fn effects_have_spell_cast_metric(
    effects: &'static [baylee_cards_dsl::Effect],
    predicate: impl Fn(baylee_cards_dsl::SpellCastMetric) -> bool + Copy,
) -> bool {
    let mut seen = 0;
    let mut found = false;
    baylee_cards_dsl::Effect::walk(effects, &mut seen, &mut |effect| {
        use baylee_cards_dsl::{Effect, Modifier};
        found |= match effect {
            Effect::CreateContinuousEffect {
                modifier: Modifier::AdjustSpellCostByCast { metric, .. },
                ..
            } => predicate(*metric),
            Effect::MoveAndModify { modifiers, .. } => modifiers.iter().any(|m| {
                matches!(
                    m,
                    Modifier::AdjustSpellCostByCast { metric, .. } if predicate(*metric)
                )
            }),
            Effect::CreateEmblem { abilities } => abilities
                .iter()
                .any(|a| ability_has_spell_cast_metric(a, predicate)),
            _ => false,
        };
    });
    found
}
