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
        | Filter::SurveilledByYouThisTurn
        | Filter::Tapped
        | Filter::Untapped
        | Filter::Attacking
        | Filter::AttackingYouOrYourPlaneswalker
        | Filter::DefendingPlayerIsYou
        | Filter::FirstMatchingSpellThisTurn(_)
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
        | Filter::Saddled
        | Filter::RingBearer
        | Filter::Modified
        | Filter::SharesAColorWith(_)
        | Filter::SharesColorWithMostCommon(_)
        | Filter::SharesCreatureTypeWith(_)
        | Filter::SharesColorWithExiledWithSource
        | Filter::SharesColorWithSource
        | Filter::CmcAtMostAmount(_)
        | Filter::CmcEqualsAmount(_)
        | Filter::PowerAtMostAmount(_)
        | Filter::PowerAtLeastAmount(_)
        | Filter::PowerEqualsAmount(_)
        | Filter::ToughnessEqualsAmount(_)
        | Filter::CmcEqualsChosenNumber
        | Filter::PowerEqualsChosenNumber
        | Filter::ToughnessEqualsChosenNumber
        | Filter::IsRemembered
        | Filter::NotRemembered
        | Filter::SharesCardTypeWithRemembered
        | Filter::SharesNameWithRemembered
        | Filter::AttachedHostOfRemembered
        | Filter::SharesNameWithAnotherPermanent => true,
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(filter_reads_board_state),
        Filter::Not(f) | Filter::ControllerControls(f) | Filter::TargetsYouControl(f) => {
            filter_reads_board_state(f)
        }
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
        | Filter::SharesTypeWithReference(_)
        | Filter::SharesCardTypeWithReference => true,
        Filter::CmcAtMostAmount(amt)
        | Filter::CmcEqualsAmount(amt)
        | Filter::PowerAtMostAmount(amt)
        | Filter::PowerAtLeastAmount(amt)
        | Filter::PowerEqualsAmount(amt)
        | Filter::ToughnessEqualsAmount(amt) => amount_reads_reference_at(amt, depth + 1),
        Filter::And(parts) | Filter::Or(parts) => {
            parts.iter().any(|f| filter_reads_reference_at(f, depth))
        }
        Filter::Not(f)
        | Filter::AttachedTo(f)
        | Filter::BlockedThisTurnBy(f)
        | Filter::LowestManaValueOnBattlefield(f)
        | Filter::SharesAColorWith(f)
        | Filter::SharesColorWithMostCommon(f)
        | Filter::SharesCreatureTypeWith(f)
        | Filter::ControllerControls(f)
        | Filter::TargetsYouControl(f) => filter_reads_reference_at(f, depth),
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
        | Amount::GreatestManaValueAmong(f)
        | Amount::TotalPowerAmong(f)
        | Amount::EnteredThisTurn { filter: f, .. }
        | Amount::TokensCreatedThisTurn { filter: f, .. }
        | Amount::LeftZoneThisTurn { filter: f, .. }
        | Amount::PermanentsSacrificedThisTurn { filter: f, .. }
        | Amount::SpellsCastMatchingThisTurn { filter: f, .. }
        | Amount::CreaturesAttackedThisTurn { filter: f, .. }
        | Amount::DistinctManaValuesInZone { filter: f, .. }
        | Amount::DistinctNames { filter: f, .. }
        | Amount::DistinctPowersAmong(f)
        | Amount::HighestControlledCount { filter: f, .. }
        | Amount::TotalManaValueInZone { filter: f, .. }
        | Amount::DistinctExactManaCostsInZone { filter: f, .. } => {
            filter_reads_reference_at(f, depth)
        }
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
        Filter::ExiledByYou => true,
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
            }
            | baylee_cards_dsl::ReplacementRule::TriggerMultiplierMatching {
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
        | Filter::PowerAtLeastAmount(a)
        | Filter::PowerEqualsAmount(a)
        | Filter::ToughnessEqualsAmount(a) => amount_reads_crime_at(a, depth + 1),
        Filter::And(parts) | Filter::Or(parts) => {
            parts.iter().any(|f| filter_reads_crime(f, depth))
        }
        Filter::Not(f)
        | Filter::AttachedTo(f)
        | Filter::BlockedThisTurnBy(f)
        | Filter::LowestManaValueOnBattlefield(f)
        | Filter::SharesAColorWith(f)
        | Filter::SharesColorWithMostCommon(f)
        | Filter::SharesCreatureTypeWith(f)
        | Filter::ControllerControls(f)
        | Filter::TargetsYouControl(f) => filter_reads_crime(f, depth),
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
        | Amount::GreatestManaValueAmong(f)
        | Amount::TotalPowerAmong(f)
        | Amount::EnteredThisTurn { filter: f, .. }
        | Amount::TokensCreatedThisTurn { filter: f, .. }
        | Amount::LeftZoneThisTurn { filter: f, .. }
        | Amount::PermanentsSacrificedThisTurn { filter: f, .. }
        | Amount::SpellsCastMatchingThisTurn { filter: f, .. }
        | Amount::CreaturesAttackedThisTurn { filter: f, .. }
        | Amount::DistinctManaValuesInZone { filter: f, .. }
        | Amount::DistinctNames { filter: f, .. }
        | Amount::DistinctPowersAmong(f)
        | Amount::HighestControlledCount { filter: f, .. }
        | Amount::TotalManaValueInZone { filter: f, .. }
        | Amount::DistinctExactManaCostsInZone { filter: f, .. } => {
            filter_reads_crime(f, depth + 1)
        }
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
        Modifier::GrantFlash { filter, .. }
        | Modifier::FirstSpellEachTurnCostAdjust { filter, .. } => filter_reads_crime(filter, 0),
        Modifier::MayCastWithAltCost {
            max_mana_value: Some(amt),
            ..
        } => amount_reads_crime_at(amt, 0),
        Modifier::SpellsCostLessByAmount(amount) => amount_reads_crime_at(amount, 0),
        Modifier::SpellsCostLessManaByAmount { filter, amount, .. } => {
            filter_reads_crime(filter, 0) || amount_reads_crime_at(amount, 0)
        }
        Modifier::AdjustSpellCostByAmount { amount, .. } => amount_reads_crime_at(amount, 0),
        Modifier::ActivatedAbilitiesCostLess { filter, amount, .. } => {
            filter_reads_crime(filter, 0) || amount_reads_crime_at(amount, 0)
        }
        Modifier::PreventDamageRemovingCounters {
            then: Some(rider), ..
        } => effects_read_crime(rider.instruction()),
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
            }
            | baylee_cards_dsl::ReplacementRule::TriggerMultiplierMatching {
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
        | Filter::PowerAtLeastAmount(a)
        | Filter::PowerEqualsAmount(a)
        | Filter::ToughnessEqualsAmount(a) => amount_reads_expend_at(a, depth + 1),
        Filter::And(parts) | Filter::Or(parts) => {
            parts.iter().any(|f| filter_reads_expend(f, depth))
        }
        Filter::Not(f)
        | Filter::AttachedTo(f)
        | Filter::BlockedThisTurnBy(f)
        | Filter::LowestManaValueOnBattlefield(f)
        | Filter::SharesAColorWith(f)
        | Filter::SharesColorWithMostCommon(f)
        | Filter::SharesCreatureTypeWith(f)
        | Filter::ControllerControls(f)
        | Filter::TargetsYouControl(f) => filter_reads_expend(f, depth),
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
        | Amount::GreatestManaValueAmong(f)
        | Amount::TotalPowerAmong(f)
        | Amount::EnteredThisTurn { filter: f, .. }
        | Amount::TokensCreatedThisTurn { filter: f, .. }
        | Amount::LeftZoneThisTurn { filter: f, .. }
        | Amount::PermanentsSacrificedThisTurn { filter: f, .. }
        | Amount::SpellsCastMatchingThisTurn { filter: f, .. }
        | Amount::CreaturesAttackedThisTurn { filter: f, .. }
        | Amount::DistinctManaValuesInZone { filter: f, .. }
        | Amount::DistinctNames { filter: f, .. }
        | Amount::DistinctPowersAmong(f)
        | Amount::HighestControlledCount { filter: f, .. }
        | Amount::TotalManaValueInZone { filter: f, .. }
        | Amount::DistinctExactManaCostsInZone { filter: f, .. } => {
            filter_reads_expend(f, depth + 1)
        }
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
        Modifier::GrantFlash { filter, .. }
        | Modifier::FirstSpellEachTurnCostAdjust { filter, .. } => filter_reads_expend(filter, 0),
        Modifier::MayCastWithAltCost {
            max_mana_value: Some(amt),
            ..
        } => amount_reads_expend_at(amt, 0),
        Modifier::SpellsCostLessByAmount(amount) => amount_reads_expend_at(amount, 0),
        Modifier::SpellsCostLessManaByAmount { filter, amount, .. } => {
            filter_reads_expend(filter, 0) || amount_reads_expend_at(amount, 0)
        }
        Modifier::AdjustSpellCostByAmount { amount, .. } => amount_reads_expend_at(amount, 0),
        Modifier::ActivatedAbilitiesCostLess { filter, amount, .. } => {
            filter_reads_expend(filter, 0) || amount_reads_expend_at(amount, 0)
        }
        Modifier::PreventDamageRemovingCounters {
            then: Some(rider), ..
        } => effects_read_expend(rider.instruction()),
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
            }
            | Effect::CreateTokenAttached { amount, .. }
            | Effect::ChooseCardConstrained { max: amount, .. }
            | Effect::SearchLibraryDistinctNames { count: amount, .. }
            | Effect::AttachChosen { max: amount, .. } => amount_reads_expend(amount),
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
            Effect::ChooseNumberEx { min, max, .. } => {
                amount_reads_expend(min) || amount_reads_expend(max)
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
    }) || card_reads_pred(def, &|_| false, &|amt| {
        matches!(amt, baylee_cards_dsl::Amount::CardsDiscardedThisTurn(_))
    })
}

pub(crate) fn card_reads_sacrificed_permanents(def: &CardDef) -> bool {
    card_has_spell_cast_metric(def, |metric| {
        matches!(
            metric,
            baylee_cards_dsl::SpellCastMetric::PermanentsSacrificedThisTurn(_)
        )
    }) || card_reads_pred(def, &|_| false, &|amt| {
        matches!(
            amt,
            baylee_cards_dsl::Amount::PermanentsSacrificedThisTurn { .. }
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
            }
            | Effect::AddTempModifier {
                modifier: Modifier::AdjustSpellCostByCast { metric, .. },
                ..
            }
            | Effect::AddGlobalTempModifier {
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

/// Whether `def` reads per-turn frozen zone-change snapshots
/// (`Amount::EnteredThisTurn`, `Amount::TokensCreatedThisTurn`, or
/// `Amount::LeftZoneThisTurn`, or `Filter::EnteredCurrentZoneThisTurn`).
pub(crate) fn card_reads_zone_changes(def: &CardDef) -> bool {
    card_reads_pred(def, &|_| false, &|amt| {
        matches!(
            amt,
            baylee_cards_dsl::Amount::EnteredThisTurn { .. }
                | baylee_cards_dsl::Amount::TokensCreatedThisTurn { .. }
                | baylee_cards_dsl::Amount::LeftZoneThisTurn { .. }
        )
    }) || card_reads_filter_or_modifier(
        def,
        &|f| matches!(f, baylee_cards_dsl::Filter::EnteredCurrentZoneThisTurn(_)),
        &|_| false,
    )
}

/// Whether `def` reads who exiled an exile-zone card (`Filter::ExiledByYou` or
/// `AltCastZones::ExiledByYou`).
pub(crate) fn card_reads_exiled_by(def: &CardDef) -> bool {
    card_reads_filter_or_modifier(
        def,
        &|f| matches!(f, baylee_cards_dsl::Filter::ExiledByYou),
        &|m| {
            matches!(
                m,
                baylee_cards_dsl::Modifier::MayCastWithAltCost {
                    zones: baylee_cards_dsl::AltCastZones::ExiledByYou,
                    ..
                } | baylee_cards_dsl::Modifier::ZonePlayGrant(baylee_cards_dsl::ZonePlayGrant {
                    zones: baylee_cards_dsl::AltCastZones::ExiledByYou,
                    ..
                })
            )
        },
    )
}

/// Whether `def` reads per-turn cast spell logs (`SpellsCastMatchingThisTurn`
/// or `DistinctCardTypesCastThisTurn`).
pub(crate) fn card_reads_spells_cast_log(def: &CardDef) -> bool {
    card_reads_pred(def, &|_| false, &|amt| {
        matches!(
            amt,
            baylee_cards_dsl::Amount::SpellsCastMatchingThisTurn { .. }
                | baylee_cards_dsl::Amount::DistinctCardTypesCastThisTurn(_)
        )
    })
}

/// Whether `def` reads per-turn who placed counters on a permanent
/// (`Condition::CountersPlacedOnSourceThisTurn { by_you: true, .. }`).
pub(crate) fn card_reads_counter_placers(def: &CardDef) -> bool {
    card_reads_pred(
        def,
        &|cond| {
            matches!(
                cond,
                baylee_cards_dsl::Condition::CountersPlacedOnSourceThisTurn { by_you: true, .. }
            )
        },
        &|_| false,
    )
}

/// Whether `def` reads the exact mana colors, color count, or triggering spell
/// mana spent (`ManaColorSpentToCast`, `ColorsOfManaSpentToCast`, or
/// `EventManaSpentToCast`).
pub(crate) fn card_reads_mana_types_spent(def: &CardDef) -> bool {
    card_reads_pred(def, &|_| false, &|amt| {
        matches!(
            amt,
            baylee_cards_dsl::Amount::ManaColorSpentToCast(_)
                | baylee_cards_dsl::Amount::ColorsOfManaSpentToCast
                | baylee_cards_dsl::Amount::EventManaSpentToCast
        )
    })
}

fn card_reads_pred(
    def: &CardDef,
    cond_pred: &impl Fn(&baylee_cards_dsl::Condition) -> bool,
    amt_pred: &impl Fn(&baylee_cards_dsl::Amount) -> bool,
) -> bool {
    let face_reads = def.faces.iter().any(|face| {
        let cr = face.cost_reduction.as_ref().is_some_and(|c| match c {
            baylee_cards_dsl::CostReduction::If { condition, .. } => {
                cond_matches(condition, cond_pred, amt_pred, 0)
            }
            baylee_cards_dsl::CostReduction::PerCount { amount, .. } => {
                amt_matches(amount, cond_pred, amt_pred, 0)
            }
            _ => false,
        });
        let alt = face.alternative_costs.iter().any(|a| {
            matches!(
                a.condition,
                baylee_cards_dsl::AltCondition::If(c) if cond_matches(c, cond_pred, amt_pred, 0)
            )
        });
        let em = face.enter_modifiers.iter().any(|m| match m {
            baylee_cards_dsl::EnterModifier::TappedUnlessCondition(c) => {
                cond_matches(c, cond_pred, amt_pred, 0)
            }
            baylee_cards_dsl::EnterModifier::WithCounters { amount, .. }
            | baylee_cards_dsl::EnterModifier::PayLifeUpTo(amount)
            | baylee_cards_dsl::EnterModifier::Bloodthirst(amount) => {
                amt_matches(amount, cond_pred, amt_pred, 0)
            }
            baylee_cards_dsl::EnterModifier::WithCountersIf {
                count, condition, ..
            } => {
                amt_matches(count, cond_pred, amt_pred, 0)
                    || cond_matches(condition, cond_pred, amt_pred, 0)
            }
            _ => false,
        });
        cr || alt || em
    });
    face_reads
        || (0..def.faces.len().max(1)).any(|face| {
            def.abilities_for_face(face)
                .iter()
                .any(|ability| ability_matches(ability, cond_pred, amt_pred))
        })
}

#[allow(clippy::too_many_lines)]
fn ability_matches(
    ability: &baylee_cards_dsl::AbilityDef,
    cond_pred: &impl Fn(&baylee_cards_dsl::Condition) -> bool,
    amt_pred: &impl Fn(&baylee_cards_dsl::Amount) -> bool,
) -> bool {
    use baylee_cards_dsl::{AbilityDef, ActivationLimit, ActivationX, CostReduction};
    let cr_hit = |cr: Option<&CostReduction>| {
        cr.is_some_and(|c| match c {
            CostReduction::If { condition, .. } => cond_matches(condition, cond_pred, amt_pred, 0),
            CostReduction::PerCount { amount, .. } => amt_matches(amount, cond_pred, amt_pred, 0),
            _ => false,
        })
    };
    let lim_hit = |limit: ActivationLimit| matches!(limit, ActivationLimit::PerTurnAmount(a) if amt_matches(&a, cond_pred, amt_pred, 0));
    let x_hit = |x_val: ActivationX| matches!(x_val, ActivationX::Defined(a) if amt_matches(&a, cond_pred, amt_pred, 0));
    match ability {
        AbilityDef::Triggered {
            effects, condition, ..
        } => {
            condition
                .as_ref()
                .is_some_and(|c| cond_matches(c, cond_pred, amt_pred, 0))
                || effects_match(effects, cond_pred, amt_pred)
        }
        AbilityDef::ModalTriggered {
            modes, condition, ..
        } => {
            condition
                .as_ref()
                .is_some_and(|c| cond_matches(c, cond_pred, amt_pred, 0))
                || modes
                    .iter()
                    .any(|m| effects_match(m.effects, cond_pred, amt_pred))
        }
        AbilityDef::Spell {
            effects, condition, ..
        } => {
            condition
                .as_ref()
                .is_some_and(|c| cond_matches(c, cond_pred, amt_pred, 0))
                || effects_match(effects, cond_pred, amt_pred)
        }
        AbilityDef::Activated {
            effects,
            cost_reduction,
            limit,
            x_value,
            ..
        } => {
            cr_hit(cost_reduction.as_ref())
                || lim_hit(*limit)
                || x_hit(*x_value)
                || effects_match(effects, cond_pred, amt_pred)
        }
        AbilityDef::ActivatedConditional {
            effects,
            condition,
            cost_reduction,
            limit,
            x_value,
            ..
        } => {
            cond_matches(condition, cond_pred, amt_pred, 0)
                || cr_hit(cost_reduction.as_ref())
                || lim_hit(*limit)
                || x_hit(*x_value)
                || effects_match(effects, cond_pred, amt_pred)
        }
        AbilityDef::ModalActivated {
            modes, condition, ..
        } => {
            condition
                .as_ref()
                .is_some_and(|c| cond_matches(c, cond_pred, amt_pred, 0))
                || modes
                    .iter()
                    .any(|m| effects_match(m.effects, cond_pred, amt_pred))
        }
        AbilityDef::ModalSpell { modes, .. } | AbilityDef::Modal { modes, .. } => modes
            .iter()
            .any(|m| effects_match(m.effects, cond_pred, amt_pred)),
        AbilityDef::SagaChapter { effects, .. } | AbilityDef::Loyalty { effects, .. } => {
            effects_match(effects, cond_pred, amt_pred)
        }
        AbilityDef::Static(sa) => {
            sa.condition
                .as_ref()
                .is_some_and(|c| cond_matches(c, cond_pred, amt_pred, 0))
                || filter_matches(&sa.filter, cond_pred, amt_pred, 0)
                || modifier_matches(&sa.modifier, cond_pred, amt_pred)
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
            }
            | baylee_cards_dsl::ReplacementRule::TriggerMultiplierMatching {
                condition: Some(c), ..
            },
        ) => cond_matches(c, cond_pred, amt_pred, 0),
        _ => false,
    }
}

fn cond_matches(
    cond: &baylee_cards_dsl::Condition,
    cond_pred: &impl Fn(&baylee_cards_dsl::Condition) -> bool,
    amt_pred: &impl Fn(&baylee_cards_dsl::Amount) -> bool,
    depth: u8,
) -> bool {
    use baylee_cards_dsl::Condition;
    if depth >= 8 {
        return false;
    }
    if cond_pred(cond) {
        return true;
    }
    match cond {
        Condition::AmountAtLeast(amount, _) | Condition::AmountIsEven(amount) => {
            amt_matches(amount, cond_pred, amt_pred, depth + 1)
        }
        Condition::AmountCompare { left, right, .. } => {
            amt_matches(left, cond_pred, amt_pred, depth + 1)
                || amt_matches(right, cond_pred, amt_pred, depth + 1)
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
        | Condition::ObjectMatches { filter: f, .. } => {
            filter_matches(f, cond_pred, amt_pred, depth + 1)
        }
        Condition::EventBlockedByOthers { some, none } => {
            filter_matches(some, cond_pred, amt_pred, depth + 1)
                || filter_matches(none, cond_pred, amt_pred, depth + 1)
        }
        Condition::Not(inner) | Condition::TriggerOnly(inner) => {
            cond_matches(inner, cond_pred, amt_pred, depth + 1)
        }
        Condition::All(parts) | Condition::Any(parts) => parts
            .iter()
            .any(|p| cond_matches(p, cond_pred, amt_pred, depth + 1)),
        _ => false,
    }
}

fn filter_matches(
    filter: &baylee_cards_dsl::Filter,
    cond_pred: &impl Fn(&baylee_cards_dsl::Condition) -> bool,
    amt_pred: &impl Fn(&baylee_cards_dsl::Amount) -> bool,
    depth: u8,
) -> bool {
    use baylee_cards_dsl::Filter;
    if depth >= 8 {
        return false;
    }
    match filter {
        Filter::CmcAtMostAmount(a)
        | Filter::CmcEqualsAmount(a)
        | Filter::PowerAtMostAmount(a)
        | Filter::PowerAtLeastAmount(a)
        | Filter::PowerEqualsAmount(a)
        | Filter::ToughnessEqualsAmount(a) => amt_matches(a, cond_pred, amt_pred, depth + 1),
        Filter::And(parts) | Filter::Or(parts) => parts
            .iter()
            .any(|f| filter_matches(f, cond_pred, amt_pred, depth)),
        Filter::Not(f)
        | Filter::AttachedTo(f)
        | Filter::BlockedThisTurnBy(f)
        | Filter::LowestManaValueOnBattlefield(f)
        | Filter::SharesAColorWith(f)
        | Filter::SharesColorWithMostCommon(f)
        | Filter::SharesCreatureTypeWith(f)
        | Filter::ControllerControls(f)
        | Filter::TargetsYouControl(f) => filter_matches(f, cond_pred, amt_pred, depth),
        _ => false,
    }
}

fn amt_matches(
    amount: &baylee_cards_dsl::Amount,
    cond_pred: &impl Fn(&baylee_cards_dsl::Condition) -> bool,
    amt_pred: &impl Fn(&baylee_cards_dsl::Amount) -> bool,
    depth: u8,
) -> bool {
    use baylee_cards_dsl::Amount;
    if depth >= 8 {
        return false;
    }
    if amt_pred(amount) {
        return true;
    }
    match amount {
        Amount::IfCondition {
            condition,
            then,
            otherwise,
        } => {
            cond_matches(condition, cond_pred, amt_pred, depth + 1)
                || amt_matches(then, cond_pred, amt_pred, depth + 1)
                || amt_matches(otherwise, cond_pred, amt_pred, depth + 1)
        }
        Amount::DistinctColorsAmong(f)
        | Amount::BasicLandTypesAmong(f)
        | Amount::CountOf { filter: f, .. }
        | Amount::ControlledCount { filter: f, .. }
        | Amount::DamageThisTurn { by: f, .. }
        | Amount::GreatestPowerAmong(f)
        | Amount::GreatestManaValueAmong(f)
        | Amount::TotalPowerAmong(f)
        | Amount::EnteredThisTurn { filter: f, .. }
        | Amount::TokensCreatedThisTurn { filter: f, .. }
        | Amount::LeftZoneThisTurn { filter: f, .. }
        | Amount::PermanentsSacrificedThisTurn { filter: f, .. }
        | Amount::SpellsCastMatchingThisTurn { filter: f, .. }
        | Amount::CreaturesAttackedThisTurn { filter: f, .. }
        | Amount::DistinctManaValuesInZone { filter: f, .. }
        | Amount::DistinctNames { filter: f, .. }
        | Amount::DistinctPowersAmong(f)
        | Amount::HighestControlledCount { filter: f, .. }
        | Amount::TotalManaValueInZone { filter: f, .. }
        | Amount::DistinctExactManaCostsInZone { filter: f, .. } => {
            filter_matches(f, cond_pred, amt_pred, depth + 1)
        }
        Amount::Negated(a)
        | Amount::Plus { base: a, .. }
        | Amount::SaturatingSub { base: a, .. }
        | Amount::ConstantMinus { subtract: a, .. }
        | Amount::Half { base: a, .. }
        | Amount::Divided { base: a, .. }
        | Amount::Times { base: a, .. } => amt_matches(a, cond_pred, amt_pred, depth + 1),
        Amount::Sum { left: a, right: b } | Amount::Difference { left: a, right: b } => {
            amt_matches(a, cond_pred, amt_pred, depth + 1)
                || amt_matches(b, cond_pred, amt_pred, depth + 1)
        }
        _ => false,
    }
}

fn modifier_matches(
    modifier: &baylee_cards_dsl::Modifier,
    cond_pred: &impl Fn(&baylee_cards_dsl::Condition) -> bool,
    amt_pred: &impl Fn(&baylee_cards_dsl::Amount) -> bool,
) -> bool {
    use baylee_cards_dsl::Modifier;
    match modifier {
        Modifier::GrantTriggered { effects, .. } | Modifier::GrantActivated { effects, .. } => {
            effects_match(effects, cond_pred, amt_pred)
        }
        Modifier::GrantTriggeredDefinition(ability) => {
            ability_matches(ability, cond_pred, amt_pred)
        }
        Modifier::ModifyPTByAmount { power, toughness } => {
            amt_matches(power, cond_pred, amt_pred, 0)
                || amt_matches(toughness, cond_pred, amt_pred, 0)
        }
        Modifier::GrantFlash { filter, .. } => filter_matches(filter, cond_pred, amt_pred, 0),
        Modifier::MayCastWithAltCost {
            max_mana_value: Some(amt),
            ..
        }
        | Modifier::SpellsCostLessByAmount(amt) => amt_matches(amt, cond_pred, amt_pred, 0),
        Modifier::SpellsCostLessManaByAmount { filter, amount, .. } => {
            filter_matches(filter, cond_pred, amt_pred, 0)
                || amt_matches(amount, cond_pred, amt_pred, 0)
        }
        Modifier::AdjustSpellCostByAmount { amount, .. } => {
            amt_matches(amount, cond_pred, amt_pred, 0)
        }
        Modifier::ActivatedAbilitiesCostLess { filter, amount, .. } => {
            filter_matches(filter, cond_pred, amt_pred, 0)
                || amt_matches(amount, cond_pred, amt_pred, 0)
        }
        Modifier::FirstSpellEachTurnCostAdjust { filter, .. } => {
            filter_matches(filter, cond_pred, amt_pred, 0)
        }
        Modifier::PreventDamageRemovingCounters {
            then: Some(rider), ..
        } => effects_match(rider.instruction(), cond_pred, amt_pred),
        _ => false,
    }
}

#[allow(clippy::too_many_lines)] // One arm per Effect variant carrying an Amount or Condition.
fn effects_match(
    effects: &'static [baylee_cards_dsl::Effect],
    cond_pred: &impl Fn(&baylee_cards_dsl::Condition) -> bool,
    amt_pred: &impl Fn(&baylee_cards_dsl::Amount) -> bool,
) -> bool {
    let mut seen = 0;
    let mut found = false;
    baylee_cards_dsl::Effect::walk(effects, &mut seen, &mut |effect| {
        use baylee_cards_dsl::Effect;
        found |= match effect {
            Effect::IfCondition { condition, .. } | Effect::RepeatWhile { condition, .. } => {
                cond_matches(condition, cond_pred, amt_pred, 0)
            }
            Effect::RepeatProcess {
                max_times,
                pre_condition,
                post_condition,
                ..
            } => {
                max_times
                    .as_ref()
                    .is_some_and(|a| amt_matches(a, cond_pred, amt_pred, 0))
                    || pre_condition
                        .as_ref()
                        .is_some_and(|c| cond_matches(c, cond_pred, amt_pred, 0))
                    || post_condition
                        .as_ref()
                        .is_some_and(|c| cond_matches(c, cond_pred, amt_pred, 0))
            }
            Effect::ReflexiveTrigger { condition, .. }
            | Effect::RegisterDelayedTrigger { condition, .. } => {
                condition.is_some_and(|c| cond_matches(c, cond_pred, amt_pred, 0))
            }
            Effect::DrawCards { amount }
            | Effect::DrawCardsFor { amount, .. }
            | Effect::GainLife { amount }
            | Effect::GainLifeFor { amount, .. }
            | Effect::LoseLife { amount, .. }
            | Effect::DealDamage { amount, .. }
            | Effect::DealDamageEvenly { amount, .. }
            | Effect::AddCounter { amount, .. }
            | Effect::AddCounterFilter { amount, .. }
            | Effect::AddCounterSource { amount, .. }
            | Effect::PutCountersDivided { amount, .. }
            | Effect::PutCounterChoice { amount, .. }
            | Effect::CreateTokenN { amount, .. }
            | Effect::CreateTokensFor { amount, .. }
            | Effect::Mill { amount, .. }
            | Effect::Scry { amount, .. }
            | Effect::Surveil { amount, .. }
            | Effect::AddMana { amount, .. }
            | Effect::NextSpellThisTurnCostsLess(amount)
            | Effect::ReduceNextSpellCost {
                generic: amount, ..
            }
            | Effect::Bolster(amount)
            | Effect::Monstrosity(amount)
            | Effect::Firebend(amount)
            | Effect::Mobilize(amount)
            | Effect::Endure(amount) => amt_matches(amount, cond_pred, amt_pred, 0),
            Effect::Repeat { times, .. } => amt_matches(times, cond_pred, amt_pred, 0),
            Effect::CreateTokenSized {
                amount,
                power,
                toughness,
                ..
            } => {
                amt_matches(amount, cond_pred, amt_pred, 0)
                    || amt_matches(power, cond_pred, amt_pred, 0)
                    || amt_matches(toughness, cond_pred, amt_pred, 0)
            }
            Effect::DamagePermanentAndController {
                permanent_amount,
                controller_amount,
                ..
            } => {
                amt_matches(permanent_amount, cond_pred, amt_pred, 0)
                    || amt_matches(controller_amount, cond_pred, amt_pred, 0)
            }
            Effect::PumpTarget {
                power, toughness, ..
            }
            | Effect::PumpFilter {
                power, toughness, ..
            }
            | Effect::PumpEventObject {
                power, toughness, ..
            } => {
                amt_matches(power, cond_pred, amt_pred, 0)
                    || amt_matches(toughness, cond_pred, amt_pred, 0)
            }
            Effect::CreateContinuousEffect { modifier, .. }
            | Effect::AddTempModifier { modifier, .. }
            | Effect::AddGlobalTempModifier { modifier, .. } => {
                modifier_matches(modifier, cond_pred, amt_pred)
            }
            Effect::CreateEmblem { abilities } => abilities
                .iter()
                .any(|a| ability_matches(a, cond_pred, amt_pred)),
            Effect::Backup {
                grant_abilities, ..
            } => grant_abilities
                .iter()
                .any(|a| ability_matches(a, cond_pred, amt_pred)),
            Effect::MoveAndModify {
                counters,
                modifiers,
                ..
            } => {
                counters
                    .iter()
                    .any(|(_, amt)| amt_matches(amt, cond_pred, amt_pred, 0))
                    || modifiers
                        .iter()
                        .any(|m| modifier_matches(m, cond_pred, amt_pred))
            }
            Effect::ChooseNumberEx { min, max, .. } => {
                amt_matches(min, cond_pred, amt_pred, 0) || amt_matches(max, cond_pred, amt_pred, 0)
            }
            Effect::CreateTokenAttached { amount, host, .. } => {
                amt_matches(amount, cond_pred, amt_pred, 0)
                    || matches!(
                        host,
                        baylee_cards_dsl::AttachHost::ChosenPermanent(filter)
                            if filter_matches(filter, cond_pred, amt_pred, 0)
                    )
            }
            Effect::PutOntoBattlefieldAttached { subject, host, .. } => {
                matches!(
                    subject,
                    baylee_cards_dsl::DefinedObjects::AllInZone { filter, .. }
                        if filter_matches(filter, cond_pred, amt_pred, 0)
                ) || matches!(
                    host,
                    baylee_cards_dsl::AttachHost::ChosenPermanent(filter)
                        if filter_matches(filter, cond_pred, amt_pred, 0)
                )
            }
            Effect::PutOntoBattlefieldAttacking { subject, .. }
            | Effect::DiscardDefined { cards: subject } => matches!(
                subject,
                baylee_cards_dsl::DefinedObjects::AllInZone { filter, .. }
                    if filter_matches(filter, cond_pred, amt_pred, 0)
            ),
            Effect::ChooseCardConstrained { filter, max, .. }
            | Effect::AttachChosen { filter, max, .. } => {
                filter_matches(filter, cond_pred, amt_pred, 0)
                    || amt_matches(max, cond_pred, amt_pred, 0)
            }
            Effect::SearchLibraryDistinctNames { filter, count, .. } => {
                filter_matches(filter, cond_pred, amt_pred, 0)
                    || amt_matches(count, cond_pred, amt_pred, 0)
            }
            Effect::ForEachRemembered { filter, .. } => {
                filter_matches(filter, cond_pred, amt_pred, 0)
            }
            _ => false,
        };
    });
    found
}

pub(crate) fn card_reads_first_spell_each_turn(def: &CardDef) -> bool {
    (0..def.faces.len().max(1)).any(|face| {
        def.abilities_for_face(face)
            .iter()
            .any(ability_reads_first_spell_each_turn)
    })
}

fn filter_reads_first_spell_each_turn(filter: &baylee_cards_dsl::Filter, depth: u8) -> bool {
    use baylee_cards_dsl::Filter;
    if depth >= 8 {
        return false;
    }
    match filter {
        Filter::FirstMatchingSpellThisTurn(_) => true,
        Filter::And(parts) | Filter::Or(parts) => parts
            .iter()
            .any(|p| filter_reads_first_spell_each_turn(p, depth + 1)),
        Filter::Not(f) | Filter::ControllerControls(f) | Filter::TargetsYouControl(f) => {
            filter_reads_first_spell_each_turn(f, depth + 1)
        }
        _ => false,
    }
}

fn ability_reads_first_spell_each_turn(ability: &baylee_cards_dsl::AbilityDef) -> bool {
    use baylee_cards_dsl::{AbilityDef, Modifier};
    match ability {
        AbilityDef::Static(sa) => {
            matches!(sa.modifier, Modifier::FirstSpellEachTurnCostAdjust { .. })
                || filter_reads_first_spell_each_turn(&sa.filter, 0)
                || sa.condition.as_ref().is_some_and(|c| {
                    matches!(
                        c,
                        baylee_cards_dsl::Condition::FirstMatchingSpellThisTurn(_)
                    )
                })
        }
        AbilityDef::Spell { effects, .. }
        | AbilityDef::Triggered { effects, .. }
        | AbilityDef::Activated { effects, .. }
        | AbilityDef::ActivatedConditional { effects, .. }
        | AbilityDef::SagaChapter { effects, .. }
        | AbilityDef::Loyalty { effects, .. } => effects_read_first_spell_each_turn(effects),
        AbilityDef::ModalSpell { modes, .. }
        | AbilityDef::Modal { modes, .. }
        | AbilityDef::ModalActivated { modes, .. }
        | AbilityDef::ModalTriggered { modes, .. } => modes
            .iter()
            .any(|m| effects_read_first_spell_each_turn(m.effects)),
        _ => false,
    }
}

fn effects_read_first_spell_each_turn(effects: &'static [baylee_cards_dsl::Effect]) -> bool {
    let mut seen = 0;
    let mut found = false;
    baylee_cards_dsl::Effect::walk(effects, &mut seen, &mut |effect| {
        use baylee_cards_dsl::{Effect, Modifier};
        found |= match effect {
            Effect::CreateContinuousEffect {
                modifier: Modifier::FirstSpellEachTurnCostAdjust { .. },
                ..
            }
            | Effect::AddTempModifier {
                modifier: Modifier::FirstSpellEachTurnCostAdjust { .. },
                ..
            }
            | Effect::AddGlobalTempModifier {
                modifier: Modifier::FirstSpellEachTurnCostAdjust { .. },
                ..
            } => true,
            Effect::MoveAndModify { modifiers, .. } => modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FirstSpellEachTurnCostAdjust { .. })),
            Effect::CreateEmblem { abilities } => {
                abilities.iter().any(ability_reads_first_spell_each_turn)
            }
            _ => false,
        };
    });
    found
}

/// Whether `def` reads non-hand, non-graveyard cast origin provenance
/// (`Filter::CastFromZone` / `Condition::SourceCastFromZone` for `Exile`,
/// `Library`, or `Command`).
pub(crate) fn card_reads_cast_origin_zone(def: &CardDef) -> bool {
    let zone_needs_rider = |z: baylee_cards_dsl::ZoneRef| {
        matches!(
            z,
            baylee_cards_dsl::ZoneRef::Exile
                | baylee_cards_dsl::ZoneRef::Library
                | baylee_cards_dsl::ZoneRef::Command
        )
    };
    if card_reads_pred(
        def,
        &|cond| matches!(cond, baylee_cards_dsl::Condition::SourceCastFromZone(z) if zone_needs_rider(*z)),
        &|_| false,
    ) {
        return true;
    }
    (0..def.faces.len().max(1)).any(|face| {
        def.abilities_for_face(face)
            .iter()
            .any(ability_reads_cast_origin_zone)
    })
}

fn filter_reads_cast_origin_zone(filter: &baylee_cards_dsl::Filter, depth: u8) -> bool {
    use baylee_cards_dsl::{Filter, ZoneRef};
    if depth >= 8 {
        return false;
    }
    match filter {
        Filter::CastFromZone(ZoneRef::Exile | ZoneRef::Library | ZoneRef::Command) => true,
        Filter::And(parts) | Filter::Or(parts) => parts
            .iter()
            .any(|f| filter_reads_cast_origin_zone(f, depth + 1)),
        Filter::Not(f)
        | Filter::AttachedTo(f)
        | Filter::BlockedThisTurnBy(f)
        | Filter::LowestManaValueOnBattlefield(f)
        | Filter::SharesAColorWith(f)
        | Filter::SharesColorWithMostCommon(f)
        | Filter::SharesCreatureTypeWith(f)
        | Filter::ControllerControls(f)
        | Filter::TargetsYouControl(f)
        | Filter::ActivatedAbilityFrom(f) => filter_reads_cast_origin_zone(f, depth + 1),
        Filter::CmcAtMostAmount(a)
        | Filter::CmcEqualsAmount(a)
        | Filter::PowerAtMostAmount(a)
        | Filter::PowerAtLeastAmount(a)
        | Filter::PowerEqualsAmount(a)
        | Filter::ToughnessEqualsAmount(a) => amt_reads_cast_origin_zone(a, depth + 1),
        _ => false,
    }
}

fn cond_reads_cast_origin_zone(cond: &baylee_cards_dsl::Condition, depth: u8) -> bool {
    use baylee_cards_dsl::{Condition, ZoneRef};
    if depth >= 8 {
        return false;
    }
    match cond {
        Condition::SourceCastFromZone(ZoneRef::Exile | ZoneRef::Library | ZoneRef::Command) => true,
        Condition::AmountAtLeast(a, _) | Condition::AmountIsEven(a) => {
            amt_reads_cast_origin_zone(a, depth + 1)
        }
        Condition::AmountCompare { left, right, .. } => {
            amt_reads_cast_origin_zone(left, depth + 1)
                || amt_reads_cast_origin_zone(right, depth + 1)
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
        | Condition::ObjectMatches { filter: f, .. } => filter_reads_cast_origin_zone(f, depth + 1),
        Condition::EventBlockedByOthers { some, none } => {
            filter_reads_cast_origin_zone(some, depth + 1)
                || filter_reads_cast_origin_zone(none, depth + 1)
        }
        Condition::Not(inner) | Condition::TriggerOnly(inner) => {
            cond_reads_cast_origin_zone(inner, depth + 1)
        }
        Condition::All(parts) | Condition::Any(parts) => parts
            .iter()
            .any(|p| cond_reads_cast_origin_zone(p, depth + 1)),
        _ => false,
    }
}

fn amt_reads_cast_origin_zone(amount: &baylee_cards_dsl::Amount, depth: u8) -> bool {
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
            cond_reads_cast_origin_zone(condition, depth + 1)
                || amt_reads_cast_origin_zone(then, depth + 1)
                || amt_reads_cast_origin_zone(otherwise, depth + 1)
        }
        Amount::DistinctColorsAmong(f)
        | Amount::BasicLandTypesAmong(f)
        | Amount::CountOf { filter: f, .. }
        | Amount::ControlledCount { filter: f, .. }
        | Amount::DamageThisTurn { by: f, .. }
        | Amount::GreatestPowerAmong(f)
        | Amount::GreatestManaValueAmong(f)
        | Amount::TotalPowerAmong(f)
        | Amount::EnteredThisTurn { filter: f, .. }
        | Amount::TokensCreatedThisTurn { filter: f, .. }
        | Amount::LeftZoneThisTurn { filter: f, .. }
        | Amount::PermanentsSacrificedThisTurn { filter: f, .. }
        | Amount::SpellsCastMatchingThisTurn { filter: f, .. }
        | Amount::CreaturesAttackedThisTurn { filter: f, .. }
        | Amount::DistinctManaValuesInZone { filter: f, .. }
        | Amount::DistinctNames { filter: f, .. } => filter_reads_cast_origin_zone(f, depth + 1),
        Amount::Negated(a)
        | Amount::Plus { base: a, .. }
        | Amount::SaturatingSub { base: a, .. }
        | Amount::ConstantMinus { subtract: a, .. }
        | Amount::Half { base: a, .. }
        | Amount::Divided { base: a, .. }
        | Amount::Times { base: a, .. } => amt_reads_cast_origin_zone(a, depth + 1),
        Amount::Sum { left: a, right: b } | Amount::Difference { left: a, right: b } => {
            amt_reads_cast_origin_zone(a, depth + 1) || amt_reads_cast_origin_zone(b, depth + 1)
        }
        _ => false,
    }
}

fn target_spec_reads_cast_origin_zone(spec: &baylee_cards_dsl::TargetSpec) -> bool {
    use baylee_cards_dsl::TargetSpec;
    match spec {
        TargetSpec::Object(f)
        | TargetSpec::ObjectOfEachOpponent(f)
        | TargetSpec::OpponentOrObject(f)
        | TargetSpec::ObjectOfFirstTargetsPlayer(f)
        | TargetSpec::ObjectControlledBy(f, _)
        | TargetSpec::ObjectOfEventPlayer(f)
        | TargetSpec::Spell(f)
        | TargetSpec::StackOrBattlefield(f)
        | TargetSpec::ObjectOrCardInAnyGraveyard(f)
        | TargetSpec::CardInGraveyard(f, _)
        | TargetSpec::CardInFirstTargetsGraveyard(f)
        | TargetSpec::CardInPlayersGraveyard(f, _)
        | TargetSpec::CardInGraveyardBelowEvent(f, _)
        | TargetSpec::CardInGraveyardBelowValue(f, _, _)
        | TargetSpec::AbilityOnStack(f)
        | TargetSpec::SpellOrAbility(f)
        | TargetSpec::OpponentWithMoreThanActivePlayer(f)
        | TargetSpec::OpponentWithMoreThanPlayer(f, _)
        | TargetSpec::PlayerOrObject(f) => filter_reads_cast_origin_zone(f, 0),
        _ => false,
    }
}

fn trigger_reads_cast_origin_zone(trigger: &baylee_cards_dsl::Trigger) -> bool {
    use baylee_cards_dsl::Trigger;
    match trigger {
        Trigger::EntersBattlefield(f)
        | Trigger::EntersBattlefieldFromGraveyardOrCast(f)
        | Trigger::LeavesBattlefield(f)
        | Trigger::Dies(f)
        | Trigger::DiesNotSacrificed(f)
        | Trigger::SpellCast(f)
        | Trigger::SpellCastUsingSourceMana(f)
        | Trigger::SpellCastOrCopied(f)
        | Trigger::NthSpellCast { filter: f, .. }
        | Trigger::ChangesZoneAll { filter: f, .. } => filter_reads_cast_origin_zone(f, 0),
        Trigger::SpellCastTargeting { spell, target, .. } => {
            filter_reads_cast_origin_zone(spell, 0)
                || target.is_some_and(|tf| filter_reads_cast_origin_zone(tf, 0))
        }
        _ => false,
    }
}

fn modifier_reads_cast_origin_zone(modifier: &baylee_cards_dsl::Modifier) -> bool {
    use baylee_cards_dsl::Modifier;
    match modifier {
        Modifier::GrantTriggered {
            trigger, effects, ..
        }
        | Modifier::GrantTriggeredUpToOne {
            trigger, effects, ..
        } => trigger_reads_cast_origin_zone(trigger) || effects_read_cast_origin_zone(effects),
        Modifier::GrantTriggeredDefinition(ability) => ability_reads_cast_origin_zone(ability),
        Modifier::GrantActivated { effects, .. } => effects_read_cast_origin_zone(effects),
        Modifier::GrantFlash { filter, .. }
        | Modifier::FirstSpellEachTurnCostAdjust { filter, .. }
        | Modifier::ActivatedAbilitiesCostLess { filter, .. }
        | Modifier::CantBeTargetedBy(filter) => filter_reads_cast_origin_zone(filter, 0),
        _ => false,
    }
}

fn replacement_reads_cast_origin_zone(rule: &baylee_cards_dsl::ReplacementRule) -> bool {
    use baylee_cards_dsl::{ReplacementRule, TriggerCause};
    match rule {
        ReplacementRule::EnterTapped(f)
        | ReplacementRule::EnterUntapped(f)
        | ReplacementRule::EnterWithCountersFromSource { filter: f } => {
            filter_reads_cast_origin_zone(f, 0)
        }
        ReplacementRule::EnterTappedIf { filter, condition }
        | ReplacementRule::EnterUntappedIf { filter, condition } => {
            filter_reads_cast_origin_zone(filter, 0) || cond_reads_cast_origin_zone(condition, 0)
        }
        ReplacementRule::TriggerMultiplier { source_filter, .. }
        | ReplacementRule::TriggerSuppress { source_filter, .. } => {
            filter_reads_cast_origin_zone(source_filter, 0)
        }
        ReplacementRule::TriggerMultiplierMatching {
            source_filter,
            cause,
            condition,
        } => {
            filter_reads_cast_origin_zone(source_filter, 0)
                || condition.is_some_and(|c| cond_reads_cast_origin_zone(&c, 0))
                || match cause {
                    TriggerCause::EntersBattlefield(f)
                    | TriggerCause::Dies(f)
                    | TriggerCause::LeavesBattlefield(f)
                    | TriggerCause::Sacrificed { filter: f, .. }
                    | TriggerCause::Attack(f)
                    | TriggerCause::SpellCastOrCopy { filter: f, .. }
                    | TriggerCause::DealsCombatDamageToPlayer(f)
                    | TriggerCause::DealtDamage(f)
                    | TriggerCause::TurnedFaceUp(f)
                    | TriggerCause::BecomesTarget(f) => filter_reads_cast_origin_zone(f, 0),
                    TriggerCause::Any
                    | TriggerCause::LifeGained(_)
                    | TriggerCause::CardDrawn(_) => false,
                }
        }
        _ => false,
    }
}

#[allow(clippy::too_many_lines)]
fn ability_reads_cast_origin_zone(ability: &baylee_cards_dsl::AbilityDef) -> bool {
    use baylee_cards_dsl::AbilityDef;
    let req_hit = |req: Option<baylee_cards_dsl::TargetReq>| {
        req.is_some_and(|r| target_spec_reads_cast_origin_zone(&r.spec))
    };
    match ability {
        AbilityDef::Triggered {
            trigger,
            effects,
            targets,
            second_targets,
            third_targets,
            condition,
            ..
        } => {
            trigger_reads_cast_origin_zone(trigger)
                || req_hit(*targets)
                || req_hit(*second_targets)
                || req_hit(*third_targets)
                || condition.is_some_and(|c| cond_reads_cast_origin_zone(&c, 0))
                || effects_read_cast_origin_zone(effects)
        }
        AbilityDef::ModalTriggered {
            trigger,
            modes,
            condition,
            ..
        } => {
            trigger_reads_cast_origin_zone(trigger)
                || condition.is_some_and(|c| cond_reads_cast_origin_zone(&c, 0))
                || modes.iter().any(|m| {
                    req_hit(m.targets)
                        || req_hit(m.second_targets)
                        || effects_read_cast_origin_zone(m.effects)
                })
        }
        AbilityDef::Spell {
            effects,
            targets,
            second_targets,
            third_targets,
            condition,
            ..
        } => {
            req_hit(*targets)
                || req_hit(*second_targets)
                || req_hit(*third_targets)
                || condition.is_some_and(|c| cond_reads_cast_origin_zone(&c, 0))
                || effects_read_cast_origin_zone(effects)
        }
        AbilityDef::Activated {
            effects,
            targets,
            second_targets,
            third_targets,
            ..
        }
        | AbilityDef::Loyalty {
            effects,
            targets,
            second_targets,
            third_targets,
            ..
        } => {
            req_hit(*targets)
                || req_hit(*second_targets)
                || req_hit(*third_targets)
                || effects_read_cast_origin_zone(effects)
        }
        AbilityDef::ActivatedConditional {
            effects,
            targets,
            second_targets,
            third_targets,
            condition,
            ..
        } => {
            req_hit(*targets)
                || req_hit(*second_targets)
                || req_hit(*third_targets)
                || cond_reads_cast_origin_zone(condition, 0)
                || effects_read_cast_origin_zone(effects)
        }
        AbilityDef::ModalActivated {
            modes, condition, ..
        } => {
            condition.is_some_and(|c| cond_reads_cast_origin_zone(&c, 0))
                || modes.iter().any(|m| {
                    req_hit(m.targets)
                        || req_hit(m.second_targets)
                        || effects_read_cast_origin_zone(m.effects)
                })
        }
        AbilityDef::ModalSpell { modes, .. } | AbilityDef::Modal { modes, .. } => {
            modes.iter().any(|m| {
                req_hit(m.targets)
                    || req_hit(m.second_targets)
                    || effects_read_cast_origin_zone(m.effects)
            })
        }
        AbilityDef::SagaChapter {
            effects, targets, ..
        } => req_hit(*targets) || effects_read_cast_origin_zone(effects),
        AbilityDef::Static(sa) => {
            filter_reads_cast_origin_zone(&sa.filter, 0)
                || sa
                    .condition
                    .is_some_and(|c| cond_reads_cast_origin_zone(&c, 0))
                || modifier_reads_cast_origin_zone(&sa.modifier)
        }
        AbilityDef::Replacement(rule) => replacement_reads_cast_origin_zone(rule),
        _ => false,
    }
}

fn mana_restriction_reads_cast_origin_zone(
    restriction: Option<baylee_cards_dsl::ManaRestriction>,
) -> bool {
    restriction.is_some_and(|r| {
        filter_reads_cast_origin_zone(r.filter, 0)
            || r.ability_filter
                .is_some_and(|af| filter_reads_cast_origin_zone(af, 0))
    })
}

fn effects_read_cast_origin_zone(effects: &'static [baylee_cards_dsl::Effect]) -> bool {
    let mut seen = 0;
    let mut found = false;
    baylee_cards_dsl::Effect::walk(effects, &mut seen, &mut |effect| {
        use baylee_cards_dsl::Effect;
        found |= match effect {
            Effect::AddMana { restriction, .. } => {
                mana_restriction_reads_cast_origin_zone(*restriction)
            }
            Effect::IfCondition { condition, .. } | Effect::RepeatWhile { condition, .. } => {
                cond_reads_cast_origin_zone(condition, 0)
            }
            Effect::CreateContinuousEffect {
                filter, modifier, ..
            } => {
                filter_reads_cast_origin_zone(filter, 0)
                    || modifier_reads_cast_origin_zone(modifier)
            }
            Effect::AddTempModifier { modifier, .. }
            | Effect::AddGlobalTempModifier { modifier, .. } => {
                modifier_reads_cast_origin_zone(modifier)
            }
            Effect::ModifyTargets { modifier, .. } => modifier_reads_cast_origin_zone(modifier),
            Effect::CreateEmblem { abilities } => {
                abilities.iter().any(ability_reads_cast_origin_zone)
            }
            Effect::RegisterDelayedTrigger {
                when, condition, ..
            } => {
                condition.is_some_and(|c| cond_reads_cast_origin_zone(c, 0))
                    || matches!(
                        when,
                        baylee_cards_dsl::DelayedTriggerWhen::OnTrigger { trigger, .. }
                            if trigger_reads_cast_origin_zone(trigger)
                    )
            }
            _ => false,
        };
    });
    found
}

fn card_reads_filter_or_modifier(
    def: &CardDef,
    filter_pred: &impl Fn(&baylee_cards_dsl::Filter) -> bool,
    mod_pred: &impl Fn(&baylee_cards_dsl::Modifier) -> bool,
) -> bool {
    (0..def.faces.len().max(1)).any(|face| {
        def.faces.get(face).is_some_and(|f| {
            cost_reduction_reads_fm(f.cost_reduction.as_ref(), filter_pred)
                || f.alternative_costs.iter().any(|alt| {
                    matches!(
                        alt.condition,
                        baylee_cards_dsl::AltCondition::If(cond)
                            if cond_reads_fm(cond, filter_pred, 0)
                    )
                })
        }) || def
            .abilities_for_face(face)
            .iter()
            .any(|a| ability_reads_fm(a, filter_pred, mod_pred))
    })
}

fn filter_reads_fm(
    filter: &baylee_cards_dsl::Filter,
    filter_pred: &impl Fn(&baylee_cards_dsl::Filter) -> bool,
    depth: u8,
) -> bool {
    use baylee_cards_dsl::Filter;
    if filter_pred(filter) {
        return true;
    }
    if depth >= 8 {
        return false;
    }
    match filter {
        Filter::And(parts) | Filter::Or(parts) => parts
            .iter()
            .any(|f| filter_reads_fm(f, filter_pred, depth + 1)),
        Filter::Not(f)
        | Filter::AttachedTo(f)
        | Filter::BlockedThisTurnBy(f)
        | Filter::LowestManaValueOnBattlefield(f)
        | Filter::SharesAColorWith(f)
        | Filter::SharesColorWithMostCommon(f)
        | Filter::SharesCreatureTypeWith(f)
        | Filter::ControllerControls(f)
        | Filter::TargetsYouControl(f)
        | Filter::FirstMatchingSpellThisTurn(f)
        | Filter::ActivatedAbilityFrom(f) => filter_reads_fm(f, filter_pred, depth + 1),
        Filter::CmcAtMostAmount(a)
        | Filter::CmcEqualsAmount(a)
        | Filter::PowerAtMostAmount(a)
        | Filter::PowerAtLeastAmount(a)
        | Filter::PowerEqualsAmount(a)
        | Filter::ToughnessEqualsAmount(a) => amt_reads_fm(a, filter_pred, depth + 1),
        _ => false,
    }
}

fn cond_reads_fm(
    cond: &baylee_cards_dsl::Condition,
    filter_pred: &impl Fn(&baylee_cards_dsl::Filter) -> bool,
    depth: u8,
) -> bool {
    use baylee_cards_dsl::Condition;
    if depth >= 8 {
        return false;
    }
    match cond {
        Condition::AmountAtLeast(a, _) | Condition::AmountIsEven(a) => {
            amt_reads_fm(a, filter_pred, depth + 1)
        }
        Condition::AmountCompare { left, right, .. } => {
            amt_reads_fm(left, filter_pred, depth + 1)
                || amt_reads_fm(right, filter_pred, depth + 1)
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
        | Condition::ObjectMatches { filter: f, .. } => filter_reads_fm(f, filter_pred, depth + 1),
        Condition::EventBlockedByOthers { some, none } => {
            filter_reads_fm(some, filter_pred, depth + 1)
                || filter_reads_fm(none, filter_pred, depth + 1)
        }
        Condition::Not(inner) | Condition::TriggerOnly(inner) => {
            cond_reads_fm(inner, filter_pred, depth + 1)
        }
        Condition::All(parts) | Condition::Any(parts) => parts
            .iter()
            .any(|p| cond_reads_fm(p, filter_pred, depth + 1)),
        _ => false,
    }
}

fn amt_reads_fm(
    amount: &baylee_cards_dsl::Amount,
    filter_pred: &impl Fn(&baylee_cards_dsl::Filter) -> bool,
    depth: u8,
) -> bool {
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
            cond_reads_fm(condition, filter_pred, depth + 1)
                || amt_reads_fm(then, filter_pred, depth + 1)
                || amt_reads_fm(otherwise, filter_pred, depth + 1)
        }
        Amount::DistinctColorsAmong(f)
        | Amount::BasicLandTypesAmong(f)
        | Amount::CountOf { filter: f, .. }
        | Amount::ControlledCount { filter: f, .. }
        | Amount::DamageThisTurn { by: f, .. }
        | Amount::GreatestPowerAmong(f)
        | Amount::GreatestManaValueAmong(f)
        | Amount::TotalPowerAmong(f)
        | Amount::EnteredThisTurn { filter: f, .. }
        | Amount::TokensCreatedThisTurn { filter: f, .. }
        | Amount::LeftZoneThisTurn { filter: f, .. }
        | Amount::PermanentsSacrificedThisTurn { filter: f, .. }
        | Amount::SpellsCastMatchingThisTurn { filter: f, .. }
        | Amount::CreaturesAttackedThisTurn { filter: f, .. }
        | Amount::DistinctManaValuesInZone { filter: f, .. }
        | Amount::DistinctNames { filter: f, .. }
        | Amount::DistinctPowersAmong(f)
        | Amount::HighestControlledCount { filter: f, .. }
        | Amount::TotalManaValueInZone { filter: f, .. }
        | Amount::DistinctExactManaCostsInZone { filter: f, .. } => {
            filter_reads_fm(f, filter_pred, depth + 1)
        }
        Amount::Negated(a)
        | Amount::Plus { base: a, .. }
        | Amount::SaturatingSub { base: a, .. }
        | Amount::ConstantMinus { subtract: a, .. }
        | Amount::Half { base: a, .. }
        | Amount::Divided { base: a, .. }
        | Amount::Times { base: a, .. } => amt_reads_fm(a, filter_pred, depth + 1),
        Amount::Sum { left: a, right: b } | Amount::Difference { left: a, right: b } => {
            amt_reads_fm(a, filter_pred, depth + 1) || amt_reads_fm(b, filter_pred, depth + 1)
        }
        _ => false,
    }
}

fn target_spec_reads_fm(
    spec: &baylee_cards_dsl::TargetSpec,
    filter_pred: &impl Fn(&baylee_cards_dsl::Filter) -> bool,
) -> bool {
    use baylee_cards_dsl::TargetSpec;
    match spec {
        TargetSpec::Object(f)
        | TargetSpec::ObjectOfEachOpponent(f)
        | TargetSpec::OpponentOrObject(f)
        | TargetSpec::ObjectOfFirstTargetsPlayer(f)
        | TargetSpec::ObjectControlledBy(f, _)
        | TargetSpec::ObjectOfEventPlayer(f)
        | TargetSpec::Spell(f)
        | TargetSpec::StackOrBattlefield(f)
        | TargetSpec::ObjectOrCardInAnyGraveyard(f)
        | TargetSpec::CardInGraveyard(f, _)
        | TargetSpec::CardInFirstTargetsGraveyard(f)
        | TargetSpec::CardInPlayersGraveyard(f, _)
        | TargetSpec::CardInGraveyardBelowEvent(f, _)
        | TargetSpec::CardInGraveyardBelowValue(f, _, _)
        | TargetSpec::AbilityOnStack(f)
        | TargetSpec::SpellOrAbility(f)
        | TargetSpec::OpponentWithMoreThanActivePlayer(f)
        | TargetSpec::OpponentWithMoreThanPlayer(f, _)
        | TargetSpec::PlayerOrObject(f) => filter_reads_fm(f, filter_pred, 0),
        _ => false,
    }
}

fn trigger_reads_fm(
    trigger: &baylee_cards_dsl::Trigger,
    filter_pred: &impl Fn(&baylee_cards_dsl::Filter) -> bool,
) -> bool {
    use baylee_cards_dsl::Trigger;
    match trigger {
        Trigger::EntersBattlefield(f)
        | Trigger::EntersBattlefieldFromGraveyardOrCast(f)
        | Trigger::LeavesBattlefield(f)
        | Trigger::Dies(f)
        | Trigger::DiesNotSacrificed(f)
        | Trigger::SpellCast(f)
        | Trigger::SpellCastUsingSourceMana(f)
        | Trigger::SpellCastOrCopied(f)
        | Trigger::NthSpellCast { filter: f, .. }
        | Trigger::ChangesZoneAll { filter: f, .. } => filter_reads_fm(f, filter_pred, 0),
        Trigger::SpellCastTargeting { spell, target, .. } => {
            filter_reads_fm(spell, filter_pred, 0)
                || target.is_some_and(|tf| filter_reads_fm(tf, filter_pred, 0))
        }
        _ => false,
    }
}

#[allow(clippy::too_many_lines)]
fn modifier_reads_fm(
    modifier: &baylee_cards_dsl::Modifier,
    filter_pred: &impl Fn(&baylee_cards_dsl::Filter) -> bool,
    mod_pred: &impl Fn(&baylee_cards_dsl::Modifier) -> bool,
) -> bool {
    use baylee_cards_dsl::Modifier;
    if mod_pred(modifier) {
        return true;
    }
    match modifier {
        Modifier::GrantTriggered {
            trigger,
            effects,
            target,
        } => {
            trigger_reads_fm(trigger, filter_pred)
                || target.is_some_and(|t| target_spec_reads_fm(&t, filter_pred))
                || effects_read_fm(effects, filter_pred, mod_pred)
        }
        Modifier::GrantTriggeredUpToOne {
            trigger,
            effects,
            target,
        } => {
            trigger_reads_fm(trigger, filter_pred)
                || target_spec_reads_fm(target, filter_pred)
                || effects_read_fm(effects, filter_pred, mod_pred)
        }
        Modifier::GrantTriggeredDefinition(ability)
        | Modifier::GrantActivatedDefinition(ability) => {
            ability_reads_fm(ability, filter_pred, mod_pred)
        }
        Modifier::GrantActivated { effects, .. } => effects_read_fm(effects, filter_pred, mod_pred),
        Modifier::GrantStatic { filter, modifier } => {
            filter_reads_fm(filter, filter_pred, 0)
                || modifier_reads_fm(modifier, filter_pred, mod_pred)
        }
        Modifier::ModifyPTByAmount { power, toughness } => {
            amt_reads_fm(power, filter_pred, 0) || amt_reads_fm(toughness, filter_pred, 0)
        }
        Modifier::SpellsCostLessByAmount(amount) => amt_reads_fm(amount, filter_pred, 0),
        Modifier::MayCastWithAltCost { max_mana_value, .. } => max_mana_value
            .as_ref()
            .is_some_and(|a| amt_reads_fm(a, filter_pred, 0)),
        Modifier::ZonePlayGrant(grant) => grant
            .max_mana_value
            .as_ref()
            .is_some_and(|a| amt_reads_fm(a, filter_pred, 0)),
        Modifier::PreventDamagePerSource {
            sources, amount, ..
        }
        | Modifier::PreventDamageToAffectedPerSource {
            sources, amount, ..
        } => filter_reads_fm(sources, filter_pred, 0) || amt_reads_fm(amount, filter_pred, 0),
        Modifier::ActivatedAbilitiesCostLess { filter, amount, .. } => {
            filter_reads_fm(filter, filter_pred, 0) || amt_reads_fm(amount, filter_pred, 0)
        }
        Modifier::AdjustSpellCostByAmount { filter, amount, .. }
        | Modifier::SpellsCostLessManaByAmount { filter, amount, .. } => {
            filter_reads_fm(filter, filter_pred, 0) || amt_reads_fm(amount, filter_pred, 0)
        }
        Modifier::ReduceCostIfTargets {
            spell_filter,
            target_filter,
            ..
        }
        | Modifier::RaiseCostIfTargets {
            spell_filter,
            target_filter,
            ..
        } => {
            filter_reads_fm(spell_filter, filter_pred, 0)
                || filter_reads_fm(target_filter, filter_pred, 0)
        }
        Modifier::ReplacePlayerDamage {
            sources: filter, ..
        }
        | Modifier::SpellsCostMorePerControlled(filter)
        | Modifier::BandsWithOther(filter)
        | Modifier::OpponentsCantCast(filter)
        | Modifier::PlayersCantCast { filter, .. }
        | Modifier::PlayersCantPlay(filter)
        | Modifier::PlayerProtectionFrom { from: filter, .. }
        | Modifier::CantBeAttackedExceptBy { by: filter, .. }
        | Modifier::PreventDamageFrom(filter)
        | Modifier::PreventCombatDamageFrom(filter)
        | Modifier::CantBeBlockedBy(filter)
        | Modifier::CantBlock(filter)
        | Modifier::CantAttackUnlessDefenderControls(filter)
        | Modifier::LandwalkMatching(filter)
        | Modifier::LandwalkOfSubtype { lands: filter, .. }
        | Modifier::MustBeBlockedByAllAbleMatching(filter)
        | Modifier::RedirectDamageToYou(filter)
        | Modifier::UntapAtMost { of: filter, .. }
        | Modifier::ProtectionFrom(filter)
        | Modifier::CantBeTargetedBy(filter)
        | Modifier::CantBeTargetedByAbilitiesFrom(filter)
        | Modifier::GrantActivatedFromLinkedExile(filter)
        | Modifier::ModifyPTPerCount { filter, .. }
        | Modifier::ModifyPTPerGraveyardCard { filter, .. }
        | Modifier::PreventDamageToYouFrom(filter)
        | Modifier::CantAttackUnlessYouSacrifice { filter, .. }
        | Modifier::CantEnterBattlefield(filter)
        | Modifier::CantAttackIfDefenderControls(filter)
        | Modifier::SpellsCostSacrificePerManaSymbol {
            sacrifice: filter, ..
        }
        | Modifier::AbilitiesCostSacrificePerManaSymbol {
            sacrifice: filter, ..
        }
        | Modifier::CantBlockMatchingUnlessPayMana {
            attacker: filter, ..
        }
        | Modifier::MatchingSpellCastLimitPerTurn { filter, .. }
        | Modifier::PreventNoncombatDamageFrom(filter)
        | Modifier::PreventNoncombatDamageToPlayers { from: filter, .. }
        | Modifier::MultiplyDamageFrom {
            sources: filter, ..
        }
        | Modifier::AddDamageFrom {
            sources: filter, ..
        }
        | Modifier::GainActivatedAbilitiesOf { filter, .. }
        | Modifier::AdjustSpellCostByCast { filter, .. }
        | Modifier::GrantFlash { filter, .. }
        | Modifier::PlayerCantCast { filter, .. }
        | Modifier::SpellsCantBeCountered { filter, .. }
        | Modifier::ReplaceDamage {
            sources: filter, ..
        }
        | Modifier::ReplaceOrPreventDamageWithThen {
            sources: filter, ..
        }
        | Modifier::FirstSpellEachTurnCostAdjust { filter, .. }
        | Modifier::NonManaAbilitiesCostMore { filter, .. }
        | Modifier::LoyaltyAbilitiesCostMore { filter, .. }
        | Modifier::AdjustLoyaltyCost { filter, .. }
        | Modifier::EquipCostsLessIfTargets {
            target_filter: filter,
            ..
        }
        | Modifier::GrantAffinity(filter)
        | Modifier::CantBeAttackedBy(filter)
        | Modifier::CastCostIncreaseUnlessTargets { filter, .. } => {
            filter_reads_fm(filter, filter_pred, 0)
        }
        _ => false,
    }
}

fn replacement_reads_fm(
    rule: &baylee_cards_dsl::ReplacementRule,
    filter_pred: &impl Fn(&baylee_cards_dsl::Filter) -> bool,
) -> bool {
    use baylee_cards_dsl::{ReplacementRule, TriggerCause};
    match rule {
        ReplacementRule::EnterTapped(f)
        | ReplacementRule::EnterUntapped(f)
        | ReplacementRule::EnterWithCountersFromSource { filter: f } => {
            filter_reads_fm(f, filter_pred, 0)
        }
        ReplacementRule::EnterTappedIf { filter, condition }
        | ReplacementRule::EnterUntappedIf { filter, condition } => {
            filter_reads_fm(filter, filter_pred, 0) || cond_reads_fm(condition, filter_pred, 0)
        }
        ReplacementRule::TriggerMultiplier { source_filter, .. }
        | ReplacementRule::TriggerSuppress { source_filter, .. } => {
            filter_reads_fm(source_filter, filter_pred, 0)
        }
        ReplacementRule::TriggerMultiplierMatching {
            source_filter,
            cause,
            condition,
        } => {
            filter_reads_fm(source_filter, filter_pred, 0)
                || condition.is_some_and(|c| cond_reads_fm(&c, filter_pred, 0))
                || match cause {
                    TriggerCause::EntersBattlefield(f)
                    | TriggerCause::Dies(f)
                    | TriggerCause::LeavesBattlefield(f)
                    | TriggerCause::Sacrificed { filter: f, .. }
                    | TriggerCause::Attack(f)
                    | TriggerCause::SpellCastOrCopy { filter: f, .. }
                    | TriggerCause::DealsCombatDamageToPlayer(f)
                    | TriggerCause::DealtDamage(f)
                    | TriggerCause::TurnedFaceUp(f)
                    | TriggerCause::BecomesTarget(f) => filter_reads_fm(f, filter_pred, 0),
                    TriggerCause::Any
                    | TriggerCause::LifeGained(_)
                    | TriggerCause::CardDrawn(_) => false,
                }
        }
        _ => false,
    }
}

fn cost_reduction_reads_fm(
    cr: Option<&baylee_cards_dsl::CostReduction>,
    filter_pred: &impl Fn(&baylee_cards_dsl::Filter) -> bool,
) -> bool {
    match cr {
        Some(baylee_cards_dsl::CostReduction::If { condition, .. }) => {
            cond_reads_fm(condition, filter_pred, 0)
        }
        Some(baylee_cards_dsl::CostReduction::PerCount { amount, .. }) => {
            amt_reads_fm(amount, filter_pred, 0)
        }
        _ => false,
    }
}

#[allow(clippy::too_many_lines)]
fn ability_reads_fm(
    ability: &baylee_cards_dsl::AbilityDef,
    filter_pred: &impl Fn(&baylee_cards_dsl::Filter) -> bool,
    mod_pred: &impl Fn(&baylee_cards_dsl::Modifier) -> bool,
) -> bool {
    use baylee_cards_dsl::AbilityDef;
    let req_hit = |req: Option<baylee_cards_dsl::TargetReq>| {
        req.is_some_and(|r| target_spec_reads_fm(&r.spec, filter_pred))
    };
    match ability {
        AbilityDef::Static(sa) => {
            filter_reads_fm(&sa.filter, filter_pred, 0)
                || sa
                    .condition
                    .as_ref()
                    .is_some_and(|c| cond_reads_fm(c, filter_pred, 0))
                || modifier_reads_fm(&sa.modifier, filter_pred, mod_pred)
        }
        AbilityDef::Replacement(rule) => replacement_reads_fm(rule, filter_pred),
        AbilityDef::Triggered {
            trigger,
            effects,
            targets,
            second_targets,
            third_targets,
            condition,
            ..
        } => {
            trigger_reads_fm(trigger, filter_pred)
                || req_hit(*targets)
                || req_hit(*second_targets)
                || req_hit(*third_targets)
                || condition
                    .as_ref()
                    .is_some_and(|c| cond_reads_fm(c, filter_pred, 0))
                || effects_read_fm(effects, filter_pred, mod_pred)
        }
        AbilityDef::ModalTriggered {
            trigger,
            modes,
            condition,
            ..
        } => {
            trigger_reads_fm(trigger, filter_pred)
                || condition
                    .as_ref()
                    .is_some_and(|c| cond_reads_fm(c, filter_pred, 0))
                || modes.iter().any(|m| {
                    req_hit(m.targets)
                        || req_hit(m.second_targets)
                        || effects_read_fm(m.effects, filter_pred, mod_pred)
                })
        }
        AbilityDef::Spell {
            effects,
            targets,
            second_targets,
            third_targets,
            condition,
            ..
        } => {
            req_hit(*targets)
                || req_hit(*second_targets)
                || req_hit(*third_targets)
                || condition
                    .as_ref()
                    .is_some_and(|c| cond_reads_fm(c, filter_pred, 0))
                || effects_read_fm(effects, filter_pred, mod_pred)
        }
        AbilityDef::Activated {
            effects,
            targets,
            second_targets,
            third_targets,
            cost_reduction,
            ..
        } => {
            req_hit(*targets)
                || req_hit(*second_targets)
                || req_hit(*third_targets)
                || cost_reduction_reads_fm(cost_reduction.as_ref(), filter_pred)
                || effects_read_fm(effects, filter_pred, mod_pred)
        }
        AbilityDef::Loyalty {
            effects,
            targets,
            second_targets,
            third_targets,
            ..
        } => {
            req_hit(*targets)
                || req_hit(*second_targets)
                || req_hit(*third_targets)
                || effects_read_fm(effects, filter_pred, mod_pred)
        }
        AbilityDef::ActivatedConditional {
            effects,
            targets,
            second_targets,
            third_targets,
            condition,
            ..
        } => {
            req_hit(*targets)
                || req_hit(*second_targets)
                || req_hit(*third_targets)
                || cond_reads_fm(condition, filter_pred, 0)
                || effects_read_fm(effects, filter_pred, mod_pred)
        }
        AbilityDef::ModalActivated {
            modes, condition, ..
        } => {
            condition
                .as_ref()
                .is_some_and(|c| cond_reads_fm(c, filter_pred, 0))
                || modes.iter().any(|m| {
                    req_hit(m.targets)
                        || req_hit(m.second_targets)
                        || effects_read_fm(m.effects, filter_pred, mod_pred)
                })
        }
        AbilityDef::ModalSpell { modes, .. } | AbilityDef::Modal { modes, .. } => {
            modes.iter().any(|m| {
                req_hit(m.targets)
                    || req_hit(m.second_targets)
                    || effects_read_fm(m.effects, filter_pred, mod_pred)
            })
        }
        AbilityDef::SagaChapter {
            effects, targets, ..
        } => req_hit(*targets) || effects_read_fm(effects, filter_pred, mod_pred),
        _ => false,
    }
}

#[allow(clippy::too_many_lines)]
fn effects_read_fm(
    effects: &'static [baylee_cards_dsl::Effect],
    filter_pred: &impl Fn(&baylee_cards_dsl::Filter) -> bool,
    mod_pred: &impl Fn(&baylee_cards_dsl::Modifier) -> bool,
) -> bool {
    let mut seen = 0;
    let mut found = false;
    let defined_hit = |cards: &baylee_cards_dsl::DefinedObjects| match cards {
        baylee_cards_dsl::DefinedObjects::AllInZone { filter, .. } => {
            filter_reads_fm(filter, filter_pred, 0)
        }
        _ => false,
    };
    let host_hit = |host: &baylee_cards_dsl::AttachHost| match host {
        baylee_cards_dsl::AttachHost::Spec(spec) => target_spec_reads_fm(spec, filter_pred),
        baylee_cards_dsl::AttachHost::ChosenPermanent(filter) => {
            filter_reads_fm(filter, filter_pred, 0)
        }
        baylee_cards_dsl::AttachHost::ChosenPlayer(_) => false,
    };
    let req_hit = |req: Option<baylee_cards_dsl::TargetReq>| {
        req.is_some_and(|r| target_spec_reads_fm(&r.spec, filter_pred))
    };
    baylee_cards_dsl::Effect::walk(effects, &mut seen, &mut |effect| {
        use baylee_cards_dsl::Effect;
        found |= match effect {
            Effect::CreateContinuousEffect {
                filter, modifier, ..
            }
            | Effect::AddGlobalTempModifier {
                filter, modifier, ..
            }
            | Effect::ModifyFilter {
                filter, modifier, ..
            } => {
                filter_reads_fm(filter, filter_pred, 0)
                    || modifier_reads_fm(modifier, filter_pred, mod_pred)
            }
            Effect::AddTempModifier {
                target, modifier, ..
            } => {
                target_spec_reads_fm(target, filter_pred)
                    || modifier_reads_fm(modifier, filter_pred, mod_pred)
            }
            Effect::ModifyTargets { modifier, .. }
            | Effect::ModifyAttached { modifier, .. }
            | Effect::ModifySource { modifier, .. }
            | Effect::ModifyEvent { modifier, .. } => {
                modifier_reads_fm(modifier, filter_pred, mod_pred)
            }
            Effect::MoveAndModify {
                counters,
                modifiers,
                ..
            } => {
                counters
                    .as_ref()
                    .is_some_and(|(_, amt)| amt_reads_fm(amt, filter_pred, 0))
                    || modifiers
                        .iter()
                        .any(|m| modifier_reads_fm(m, filter_pred, mod_pred))
            }
            Effect::GraveyardToBattlefieldModified {
                target, modifiers, ..
            } => {
                target_spec_reads_fm(target, filter_pred)
                    || modifiers
                        .iter()
                        .any(|m| modifier_reads_fm(m, filter_pred, mod_pred))
            }
            Effect::CreateEmblem { abilities } => abilities
                .iter()
                .any(|a| ability_reads_fm(a, filter_pred, mod_pred)),
            Effect::Backup {
                grant_abilities, ..
            } => grant_abilities
                .iter()
                .any(|a| ability_reads_fm(a, filter_pred, mod_pred)),
            Effect::GrantTriggeredIfAbsent {
                target,
                trigger,
                effects: inner,
                ..
            } => {
                target_spec_reads_fm(target, filter_pred)
                    || trigger_reads_fm(trigger, filter_pred)
                    || effects_read_fm(inner, filter_pred, mod_pred)
            }
            Effect::RegisterDelayedTrigger {
                when,
                condition,
                targets,
                ..
            } => {
                req_hit(*targets)
                    || condition
                        .as_ref()
                        .is_some_and(|c| cond_reads_fm(c, filter_pred, 0))
                    || matches!(
                        when,
                        baylee_cards_dsl::DelayedTriggerWhen::OnTrigger { trigger, .. }
                            if trigger_reads_fm(trigger, filter_pred)
                    )
            }
            Effect::ReflexiveTrigger {
                condition, targets, ..
            } => {
                req_hit(*targets)
                    || condition
                        .as_ref()
                        .is_some_and(|c| cond_reads_fm(c, filter_pred, 0))
            }
            Effect::WheneverCombatEventThisTurn {
                filter, targets, ..
            } => filter_reads_fm(filter, filter_pred, 0) || req_hit(*targets),
            Effect::IfCondition { condition, .. } | Effect::RepeatWhile { condition, .. } => {
                cond_reads_fm(condition, filter_pred, 0)
            }
            Effect::RepeatProcess {
                max_times,
                pre_condition,
                post_condition,
                ..
            } => {
                max_times
                    .as_ref()
                    .is_some_and(|a| amt_reads_fm(a, filter_pred, 0))
                    || pre_condition
                        .as_ref()
                        .is_some_and(|c| cond_reads_fm(c, filter_pred, 0))
                    || post_condition
                        .as_ref()
                        .is_some_and(|c| cond_reads_fm(c, filter_pred, 0))
            }
            Effect::PutOntoBattlefieldAttached { subject, host, .. } => {
                defined_hit(subject) || host_hit(host)
            }
            Effect::CreateTokenAttached { amount, host, .. } => {
                amt_reads_fm(amount, filter_pred, 0) || host_hit(host)
            }
            Effect::PutOntoBattlefieldAttacking { subject: cards, .. }
            | Effect::DiscardDefined { cards } => defined_hit(cards),
            Effect::Play { source, filter, .. } => {
                filter_reads_fm(filter, filter_pred, 0)
                    || matches!(
                        source,
                        baylee_cards_dsl::PlaySource::Subject(spec)
                            if target_spec_reads_fm(spec, filter_pred)
                    )
            }
            Effect::AddMana {
                amount,
                restriction,
                ..
            } => {
                amt_reads_fm(amount, filter_pred, 0)
                    || restriction.is_some_and(|r| {
                        filter_reads_fm(r.filter, filter_pred, 0)
                            || r.ability_filter
                                .is_some_and(|af| filter_reads_fm(af, filter_pred, 0))
                    })
            }
            Effect::DealDamageWithoutPreventionOrRedirection {
                amount,
                target,
                recipients,
            } => {
                amt_reads_fm(amount, filter_pred, 0)
                    || target_spec_reads_fm(target, filter_pred)
                    || filter_reads_fm(recipients, filter_pred, 0)
            }
            Effect::DealDamage { amount, target }
            | Effect::DealDamageEvenly { amount, target }
            | Effect::DealDamageWithoutPrevention { amount, target }
            | Effect::RedirectNextDamage { amount, target, .. }
            | Effect::PreventNextDamage { amount, target }
            | Effect::EventObjectDealsDamage { amount, target }
            | Effect::GrantorDealsDamage { amount, target }
            | Effect::PutCounterChoice { amount, target, .. }
            | Effect::CreateTokenCopy { amount, target, .. }
            | Effect::CopySpellOrAbility { amount, target, .. } => {
                amt_reads_fm(amount, filter_pred, 0) || target_spec_reads_fm(target, filter_pred)
            }
            Effect::DamagePermanentAndController {
                target,
                permanent_amount,
                controller_amount,
            } => {
                target_spec_reads_fm(target, filter_pred)
                    || amt_reads_fm(permanent_amount, filter_pred, 0)
                    || amt_reads_fm(controller_amount, filter_pred, 0)
            }
            Effect::ChooseLandwalk { lands, target, .. } => {
                filter_reads_fm(lands, filter_pred, 0) || target_spec_reads_fm(target, filter_pred)
            }
            Effect::MoveAttachment {
                attachment,
                recipient_filter,
                ..
            } => {
                target_spec_reads_fm(attachment, filter_pred)
                    || filter_reads_fm(recipient_filter, filter_pred, 0)
            }
            Effect::AttachChosen {
                filter, to, max, ..
            } => {
                filter_reads_fm(filter, filter_pred, 0)
                    || target_spec_reads_fm(to, filter_pred)
                    || amt_reads_fm(max, filter_pred, 0)
            }
            Effect::BecomeCopy {
                subject_filter,
                original,
                ..
            } => {
                subject_filter.is_some_and(|f| filter_reads_fm(f, filter_pred, 0))
                    || target_spec_reads_fm(original, filter_pred)
            }
            Effect::CreateDelayedObserver { watch, about, .. } => {
                target_spec_reads_fm(watch, filter_pred) || target_spec_reads_fm(about, filter_pred)
            }
            Effect::MustBlockIfAble {
                blocker, attacker, ..
            } => {
                target_spec_reads_fm(blocker, filter_pred)
                    || target_spec_reads_fm(attacker, filter_pred)
            }
            Effect::PreventNextDamageWithThen {
                sources,
                recipients,
                ..
            } => {
                target_spec_reads_fm(sources, filter_pred)
                    || target_spec_reads_fm(recipients, filter_pred)
            }
            Effect::PreventFromChosenSource {
                target, sources, ..
            }
            | Effect::PreventNextFromMatchingSources {
                target, sources, ..
            }
            | Effect::PreventAllDamageFromMatchingThisTurn {
                target, sources, ..
            } => {
                target.is_some_and(|t| target_spec_reads_fm(&t, filter_pred))
                    || filter_reads_fm(sources, filter_pred, 0)
            }
            Effect::PreventFromChosenSourceWithLifeCondition {
                target,
                sources,
                gain_life_sources,
            } => {
                target.is_some_and(|t| target_spec_reads_fm(&t, filter_pred))
                    || filter_reads_fm(sources, filter_pred, 0)
                    || filter_reads_fm(gain_life_sources, filter_pred, 0)
            }
            Effect::ExileAndCastCopies { graveyard } => {
                graveyard.is_some_and(|f| filter_reads_fm(f, filter_pred, 0))
            }
            Effect::SacrificeOneThen {
                filter, if_it_was, ..
            }
            | Effect::SacrificeAndIf {
                filter,
                check: if_it_was,
                ..
            } => {
                filter_reads_fm(filter, filter_pred, 0)
                    || filter_reads_fm(if_it_was, filter_pred, 0)
            }
            Effect::KeepOneOfEachAndSacrificeRest { filters } => {
                filters.iter().any(|f| filter_reads_fm(f, filter_pred, 0))
            }
            Effect::KeepOneOfEachAndSacrificeRestMatching { filters, rest } => {
                filters.iter().any(|f| filter_reads_fm(f, filter_pred, 0))
                    || filter_reads_fm(rest, filter_pred, 0)
            }
            Effect::RevealTopPutOntoBattlefield { count, picks } => {
                amt_reads_fm(count, filter_pred, 0)
                    || picks.iter().any(|f| filter_reads_fm(f, filter_pred, 0))
            }
            Effect::PumpFilter {
                filter,
                power,
                toughness,
                ..
            }
            | Effect::SetPTFilter {
                filter,
                power,
                toughness,
                ..
            } => {
                filter_reads_fm(filter, filter_pred, 0)
                    || amt_reads_fm(power, filter_pred, 0)
                    || amt_reads_fm(toughness, filter_pred, 0)
            }
            Effect::PumpTarget {
                power, toughness, ..
            }
            | Effect::PumpTargetSlot {
                power, toughness, ..
            }
            | Effect::PumpEventObject {
                power, toughness, ..
            } => amt_reads_fm(power, filter_pred, 0) || amt_reads_fm(toughness, filter_pred, 0),
            Effect::CreateTokenSized {
                amount,
                power,
                toughness,
                ..
            } => {
                amt_reads_fm(amount, filter_pred, 0)
                    || amt_reads_fm(power, filter_pred, 0)
                    || amt_reads_fm(toughness, filter_pred, 0)
            }
            Effect::DealDamageWithLimitedLifeGain { amount, life_limit } => {
                amt_reads_fm(amount, filter_pred, 0) || amt_reads_fm(life_limit, filter_pred, 0)
            }
            Effect::ChooseNumberEx { min, max, .. } => {
                amt_reads_fm(min, filter_pred, 0) || amt_reads_fm(max, filter_pred, 0)
            }
            Effect::RevealUntilThen {
                filter,
                count,
                max_revealed,
                ..
            } => {
                filter_reads_fm(filter, filter_pred, 0)
                    || amt_reads_fm(count, filter_pred, 0)
                    || max_revealed
                        .as_ref()
                        .is_some_and(|a| amt_reads_fm(a, filter_pred, 0))
            }
            Effect::DealDamageToMatchingTargets { amount, filter }
            | Effect::SearchLibraryFor {
                count: amount,
                filter,
                ..
            }
            | Effect::MillMayTakeOne { amount, filter }
            | Effect::DealDamageEach { amount, filter }
            | Effect::SacrificeAmountOrLose { filter, amount }
            | Effect::SacrificeAmountForPlayers { filter, amount, .. }
            | Effect::LookHandDiscard {
                count: amount,
                filter,
                ..
            }
            | Effect::SearchWithManaDifference {
                filter,
                value: amount,
            }
            | Effect::AddCounterFilter { filter, amount, .. }
            | Effect::DestroyAllWithManaValue {
                filter,
                value: amount,
            }
            | Effect::SearchLibraryUpTo {
                filter,
                count: amount,
                ..
            }
            | Effect::DiscardRandomMatching {
                filter,
                count: amount,
                ..
            }
            | Effect::EventObjectDealsDamageEach { amount, filter, .. }
            | Effect::DealDamageEvenlyToFilter { amount, filter }
            | Effect::ChooseAndCreateTokenCopy { filter, amount, .. }
            | Effect::ChooseCard {
                filter,
                max: amount,
                ..
            }
            | Effect::ChooseCardEx {
                filter,
                max: amount,
                ..
            }
            | Effect::ChooseCardConstrained {
                filter,
                max: amount,
                ..
            }
            | Effect::SearchLibraryDistinctNames {
                filter,
                count: amount,
                ..
            }
            | Effect::PeekAndReveal {
                count: amount,
                filter,
                ..
            }
            | Effect::RevealFromHand {
                filter,
                max: amount,
                ..
            }
            | Effect::RevealTop {
                count: amount,
                filter,
                ..
            }
            | Effect::ReduceNextSpellCost {
                filter,
                generic: amount,
                ..
            }
            | Effect::PutCountersOnChosenControlled {
                filter,
                count: amount,
                ..
            }
            | Effect::ChooseDistinctPowersOnBattlefield {
                filter,
                max: amount,
                ..
            } => filter_reads_fm(filter, filter_pred, 0) || amt_reads_fm(amount, filter_pred, 0),
            Effect::PlayerMayGainControl { filter, .. }
            | Effect::SacrificeNOrElse { filter, .. }
            | Effect::RedirectCombatDamageToYouThisTurn { by: filter, .. }
            | Effect::TapAndPayEnergyForCounters { filter, .. }
            | Effect::LookAtTopMayTake { filter, .. }
            | Effect::RandomGraveyardToHand { filter }
            | Effect::IfTargetMatchesElse { filter, .. }
            | Effect::ManaForColorsAmong(filter)
            | Effect::ChooseFromYourGraveyard { filter, .. }
            | Effect::SacrificeChosenByOpponent { filter, .. }
            | Effect::LookAtTopFind { filter, .. }
            | Effect::RevealUntilMatching { filter, .. }
            | Effect::RevealUntil { filter, .. }
            | Effect::RevealTopAndSort { filter, .. }
            | Effect::RevealTopAllMatching { filter, .. }
            | Effect::LookAtTopMayPut { filter, .. }
            | Effect::SearchLibraryOrGraveyard { filter, .. }
            | Effect::PutFromHandOntoBattlefield { filter, .. }
            | Effect::DrawRevealDiscardUnless { keep: filter }
            | Effect::ChooseDamageSourcePreventAllThisTurnMatching(filter)
            | Effect::PreventNextFromChosenSource {
                sources: filter, ..
            }
            | Effect::WishToHand { filter }
            | Effect::DestroyEventThenMayReattach { to: filter }
            | Effect::DestroyWeakest { filter, .. }
            | Effect::TargetSourceLosesAbilities {
                source_filter: filter,
            }
            | Effect::ChangeTarget { to: filter }
            | Effect::ExchangeGreatestManaValue { filter, .. }
            | Effect::DestroyChosenForPlayers { filter, .. }
            | Effect::EqualizePermanents { filter }
            | Effect::RevealHandDiscard { filter }
            | Effect::RevealTopIf { filter, .. }
            | Effect::YourGraveyardToBattlefield { filter, .. }
            | Effect::SacrificeFilter { filter, .. }
            | Effect::SacrificeAll { filter }
            | Effect::ReturnChosenToHand { filter, .. }
            | Effect::UntapChosen { filter, .. }
            | Effect::WhenNextSpellCastThisTurn { filter, .. }
            | Effect::MayBehold { filter, .. }
            | Effect::IfTargetMatches { filter, .. }
            | Effect::IfEventObjectMatches { filter, .. }
            | Effect::IfControlGreatestCmc { filter, .. }
            | Effect::SearchLibrary { filter, .. }
            | Effect::RevealLibrarySearch { filter, .. }
            | Effect::EachPlayerSearchBeforeShuffle { filter, .. }
            | Effect::SearchLibraryBeforeShuffle { filter, .. }
            | Effect::DoubleCountersFilter { filter, .. }
            | Effect::ReturnAllToHand { filter, .. }
            | Effect::ExileAll { filter, .. }
            | Effect::ExileMostVotedNotControlledByYou { filter }
            | Effect::ChooseYoursThen { filter, .. }
            | Effect::DestroyAllThenForEachDestroyed { filter, .. }
            | Effect::DestroyAll { filter, .. }
            | Effect::RegenerateAll { filter }
            | Effect::GraveyardAllToHand { filter }
            | Effect::CreateTokenPtPerCount { filter, .. }
            | Effect::BottomCardFromHand { filter, .. }
            | Effect::PlayerMaySacrificeNOr { filter, .. }
            | Effect::PlayerMaySacrificeNThen { filter, .. }
            | Effect::UntapChosenPayingEach { filter, .. }
            | Effect::PhaseOutFilter(filter)
            | Effect::ExileChosenFromHandWithSource(filter)
            | Effect::ReturnChosenLinkedToBattlefield { filter, .. }
            | Effect::SearchLibraryOf { filter, .. }
            | Effect::SearchOpponentSplits { filter, .. }
            | Effect::PumpFilterPerKeyword { filter, .. }
            | Effect::TapAll { filter }
            | Effect::TapAllOf { filter, .. }
            | Effect::UntapAll { filter }
            | Effect::ReturnFromGraveyardOf { filter, .. }
            | Effect::DestroyRandom { filter }
            | Effect::WheneverTargetDealtDamageThisTurn { by: filter, .. }
            | Effect::DealHalfOfSpellDamage { spell: filter }
            | Effect::GainControlAll { filter, .. }
            | Effect::ReturnPerBlockedDeath { filter }
            | Effect::ReturnAllOwnedToHand { filter, .. }
            | Effect::DestroyAllExceptChosen { filter, .. }
            | Effect::RemoveAllCountersFilter { filter, .. }
            | Effect::MayPutFromHandOntoBattlefieldAttachIfEquipment { filter }
            | Effect::WheneverOneOrMoreDealCombatDamageToPlayerThisTurn { filter, .. }
            | Effect::MayExileFromYourGraveyardThen { filter, .. }
            | Effect::EachPlayerGraveyardToBattlefield { filter }
            | Effect::EachPlayerMayPutFromHandOntoBattlefieldRepeatUntilNone { filter }
            | Effect::DiscardMatching { filter, .. }
            | Effect::DestroyEachUnlessAnyPlayerPaysLife { filter, .. }
            | Effect::AnyPlayerMaySacrificeOrTakeDamageThen { filter, .. }
            | Effect::WheneverTappedForManaThisTurn { filter, .. }
            | Effect::ForEachMatching { filter, .. }
            | Effect::RevealTopPlayerMayPutOntoBattlefield { filter, .. }
            | Effect::TopOfYourGraveyard { filter }
            | Effect::ExileChosenFromHandWithSourceAndCast(filter)
            | Effect::CreateTokenCopiesOfGraveyard { filter, .. }
            | Effect::RevealTopFind { filter, .. }
            | Effect::BecomeChosenCreatureType { filter, .. }
            | Effect::DestroyAllOf { filter, .. }
            | Effect::ShuffleGraveyardMatchingIntoLibrary { filter, .. }
            | Effect::GrantProtectionFromChosenColorUntilEot { filter }
            | Effect::GoadAll { filter, .. }
            | Effect::DetainFilter { filter }
            | Effect::Champion { filter }
            | Effect::VoteObjects { filter, .. }
            | Effect::SetPTByCountFilter { filter, .. }
            | Effect::RepeatEachObject { filter, .. }
            | Effect::SeparateIntoTwoPiles { filter, .. }
            | Effect::GainControlAllEx { filter, .. }
            | Effect::SacrificeAnyNumber { filter, .. }
            | Effect::SearchLibraryAndGraveyard { filter, .. }
            | Effect::ForEachRemembered { filter, .. }
            | Effect::ToggleTapAll { filter }
            | Effect::CounterAll { filter, .. }
            | Effect::DestroyTargetAndSameName { filter }
            | Effect::DestroyAllAttachedToTarget { filter, .. }
            | Effect::GainControlLtbAttached { filter, .. }
            | Effect::CaptureMatching { filter, .. }
            | Effect::GrantRememberedExilePermission { filter, .. }
            | Effect::SearchLibraryAttachedToEvent { filter, .. } => {
                filter_reads_fm(filter, filter_pred, 0)
            }
            Effect::PreventNextDamageToAttached { amount }
            | Effect::DrawThenDiscardDrawn { draw: amount, .. }
            | Effect::DealDamageToBothTargetInstances { amount }
            | Effect::Energy { amount, .. }
            | Effect::CreateTokensFor { amount, .. }
            | Effect::ChangeAttachedCounters { amount, .. }
            | Effect::SetSourceToughness { base: amount, .. }
            | Effect::AddCounterSource { amount, .. }
            | Effect::PayManaToPreventDamage { amount, .. }
            | Effect::DealDamageWithCappedLifeGain { amount }
            | Effect::GainLife { amount }
            | Effect::GainLifeFor { amount, .. }
            | Effect::SetLifeTotal { amount, .. }
            | Effect::RaiseLifeTotalTo { amount, .. }
            | Effect::LookAtTopPick { count: amount, .. }
            | Effect::LookAtTopPickThenReorder { count: amount, .. }
            | Effect::ExileTop { count: amount, .. }
            | Effect::Connive { amount }
            | Effect::LoseLife { amount, .. }
            | Effect::DrawCards { amount }
            | Effect::DrawCardsFor { amount, .. }
            | Effect::DealDamageToTargetController { amount }
            | Effect::DealDamageToAttached { amount }
            | Effect::Poison { amount, .. }
            | Effect::RevealRandomDiscardNamed { count: amount, .. }
            | Effect::DiscardAmountForPlayers { amount, .. }
            | Effect::LookAtRandomFromHand { count: amount, .. }
            | Effect::DiscardRandom { count: amount, .. }
            | Effect::PutTargetIntoLibraryBeneathTop { count: amount }
            | Effect::FlipCoinLifeExchangeRepeat {
                initial: amount, ..
            }
            | Effect::PlayersWhoCastRevealedNameThisTurnLoseLife { amount, .. }
            | Effect::Scry { amount }
            | Effect::Surveil { amount }
            | Effect::ScryFor { amount, .. }
            | Effect::Mill { amount, .. }
            | Effect::AddCounter { amount, .. }
            | Effect::AddCountersUpTo { amount, .. }
            | Effect::CreateTokenThenCounters { amount, .. }
            | Effect::CreateTokenN { amount, .. }
            | Effect::CreateTokenModified { amount, .. }
            | Effect::CreateTokenAttacking { amount, .. }
            | Effect::NextDrawLooksAtTop { count: amount }
            | Effect::Amass { amount, .. }
            | Effect::CopyThisSpell { amount }
            | Effect::PlayerMayPayOr { mana: amount, .. }
            | Effect::PlayerMayPayLifeOr { life: amount, .. }
            | Effect::PlayerMayPayManaAndLifeEachThenElse { times: amount, .. }
            | Effect::PlayerMayPayManaEachThenElse { times: amount, .. }
            | Effect::PlayerMayPayThenElse { mana: amount, .. }
            | Effect::PlayerMayPayThen { mana: amount, .. }
            | Effect::DiscoverAmount { mana_value: amount }
            | Effect::DiscardCount { count: amount, .. }
            | Effect::Repeat { times: amount, .. }
            | Effect::Rad { amount, .. }
            | Effect::PlayerMayGetPoisonOr { poison: amount, .. }
            | Effect::RevealRandomFromHand { count: amount, .. }
            | Effect::FlipCoinsDamageTailsRepeatUntilBothHeads { damage: amount }
            | Effect::DealDamageToEachOpponentAndPlaneswalkerDamagedThisGame { amount }
            | Effect::SacrificeEventDealDamageToEachBlocker { amount }
            | Effect::AddManaAmountFor { amount, .. }
            | Effect::AddCounterCreatedTokens { amount, .. }
            | Effect::NextSpellThisTurnCostsLess(amount)
            | Effect::PlayerMayPayLifeThen { life: amount, .. }
            | Effect::FlipCoins { count: amount, .. }
            | Effect::RollDice { count: amount, .. }
            | Effect::ExileTopWithPermission { count: amount, .. }
            | Effect::ManifestTop { count: amount, .. }
            | Effect::Bolster(amount)
            | Effect::PlotTop { count: amount, .. }
            | Effect::Incubate { count: amount, .. }
            | Effect::Monstrosity(amount)
            | Effect::Firebend(amount)
            | Effect::Mobilize(amount)
            | Effect::Endure(amount)
            | Effect::PutCountersDivided { amount, .. }
            | Effect::TargetDealsDamageToController { amount }
            | Effect::Empower { amount, .. }
            | Effect::AnyPlayerMayPayManaAmount { mana: amount, .. }
            | Effect::PlayerMayWaterbendOr { mana: amount, .. } => {
                amt_reads_fm(amount, filter_pred, 0)
            }
            Effect::RedirectTargetSourceDamageThisTurn { source: target, .. }
            | Effect::RedirectNextChosenSourceDamageToTarget { target }
            | Effect::PreventDamageFromTargetingCausesThisTurn { target }
            | Effect::Exile { target }
            | Effect::Blink { target, .. }
            | Effect::EventObjectDealsDamageEqualToPower { target }
            | Effect::RedirectNextFromChosenSource { target }
            | Effect::Destroy { target, .. }
            | Effect::PutOnBottomOfLibraryFromGraveyard { target }
            | Effect::CantAttackDuringNextControllerTurn { target }
            | Effect::SuppressUntap { target, .. }
            | Effect::AtEndOfCombat { about: target, .. }
            | Effect::AtEndOfCombatThisTurn { about: target, .. }
            | Effect::OwnerPutsOnTopOrBottom { target }
            | Effect::ExileIfDiesThisTurn { target }
            | Effect::CantBeRegeneratedThisTurn { target }
            | Effect::ReturnToBattlefieldTapped { target }
            | Effect::Explore { target }
            | Effect::ReturnToHand { target }
            | Effect::DestroyOthersNamedLike { target }
            | Effect::Regenerate { target }
            | Effect::GraveyardToTop { target }
            | Effect::GraveyardToHand { target }
            | Effect::GraveyardToBattlefield { target, .. }
            | Effect::Assimilate { target }
            | Effect::AttachSelf { target }
            | Effect::ExileLinked { target, .. }
            | Effect::SacrificeObject { target }
            | Effect::GraveyardToBattlefieldLinked { target }
            | Effect::RevealUntilLandDealDamage { target }
            | Effect::Goad { target }
            | Effect::ExileWithPermission { target, .. }
            | Effect::TurnFaceUp { target }
            | Effect::Plot { target }
            | Effect::Suspect { target }
            | Effect::Unsuspect { target }
            | Effect::Detain { target }
            | Effect::GainControlEx { target, .. }
            | Effect::PutEachCounterKindFromSource { target }
            | Effect::PutAllCountersFrom { target, .. }
            | Effect::BecomeChosenBasicLandType { target, .. }
            | Effect::BecomesBlocked(target) => target_spec_reads_fm(target, filter_pred),
            Effect::Reflexive { target, .. }
            | Effect::CreateTokenCopyOf { target, .. }
            | Effect::PhaseOut { target } => {
                target.is_some_and(|t| target_spec_reads_fm(&t, filter_pred))
            }
            _ => false,
        };
    });
    found
}

pub(crate) fn card_reads_surveilled_this_turn(def: &CardDef) -> bool {
    (0..def.faces.len().max(1)).any(|face| {
        def.abilities_for_face(face)
            .iter()
            .any(ability_reads_surveilled_this_turn)
    })
}

fn filter_has_surveilled(filter: &baylee_cards_dsl::Filter, depth: u8) -> bool {
    use baylee_cards_dsl::Filter;
    if depth >= 8 {
        return false;
    }
    match filter {
        Filter::SurveilledByYouThisTurn => true,
        Filter::And(parts) | Filter::Or(parts) => {
            parts.iter().any(|f| filter_has_surveilled(f, depth + 1))
        }
        Filter::Not(f)
        | Filter::AttachedTo(f)
        | Filter::BlockedThisTurnBy(f)
        | Filter::LowestManaValueOnBattlefield(f)
        | Filter::SharesAColorWith(f)
        | Filter::SharesColorWithMostCommon(f)
        | Filter::SharesCreatureTypeWith(f)
        | Filter::ControllerControls(f)
        | Filter::TargetsYouControl(f) => filter_has_surveilled(f, depth + 1),
        _ => false,
    }
}

fn ability_reads_surveilled_this_turn(ability: &baylee_cards_dsl::AbilityDef) -> bool {
    use baylee_cards_dsl::{AbilityDef, Modifier};
    match ability {
        AbilityDef::Static(sa) => {
            filter_has_surveilled(&sa.filter, 0)
                || matches!(
                    sa.modifier,
                    Modifier::GrantFlash { filter, .. } if filter_has_surveilled(filter, 0)
                )
        }
        AbilityDef::Spell { effects, .. }
        | AbilityDef::Triggered { effects, .. }
        | AbilityDef::Activated { effects, .. }
        | AbilityDef::ActivatedConditional { effects, .. }
        | AbilityDef::SagaChapter { effects, .. }
        | AbilityDef::Loyalty { effects, .. } => effects_read_surveilled_this_turn(effects),
        AbilityDef::ModalSpell { modes, .. }
        | AbilityDef::Modal { modes, .. }
        | AbilityDef::ModalActivated { modes, .. }
        | AbilityDef::ModalTriggered { modes, .. } => modes
            .iter()
            .any(|m| effects_read_surveilled_this_turn(m.effects)),
        _ => false,
    }
}

fn effects_read_surveilled_this_turn(effects: &'static [baylee_cards_dsl::Effect]) -> bool {
    let mut seen = 0;
    let mut found = false;
    baylee_cards_dsl::Effect::walk(effects, &mut seen, &mut |effect| {
        use baylee_cards_dsl::{Effect, Modifier};
        found |= match effect {
            Effect::CreateContinuousEffect {
                filter, modifier, ..
            }
            | Effect::AddGlobalTempModifier {
                filter, modifier, ..
            } => {
                filter_has_surveilled(filter, 0)
                    || matches!(
                        modifier,
                        Modifier::GrantFlash { filter: f, .. } if filter_has_surveilled(f, 0)
                    )
            }
            Effect::AddTempModifier { modifier, .. } => matches!(
                modifier,
                Modifier::GrantFlash { filter: f, .. } if filter_has_surveilled(f, 0)
            ),
            Effect::CreateEmblem { abilities } => {
                abilities.iter().any(ability_reads_surveilled_this_turn)
            }
            _ => false,
        };
    });
    found
}

pub(crate) fn card_reads_foretold_count(def: &CardDef) -> bool {
    (0..def.faces.len().max(1)).any(|face| {
        def.abilities_for_face(face)
            .iter()
            .any(ability_reads_foretold_count)
    })
}

fn ability_reads_foretold_count(ability: &baylee_cards_dsl::AbilityDef) -> bool {
    use baylee_cards_dsl::{AbilityDef, Modifier};
    match ability {
        AbilityDef::Static(sa) => matches!(
            sa.modifier,
            Modifier::ForetellActionCostsLess {
                first_each_turn_only: true,
                ..
            }
        ),
        AbilityDef::Spell { effects, .. }
        | AbilityDef::Triggered { effects, .. }
        | AbilityDef::Activated { effects, .. }
        | AbilityDef::ActivatedConditional { effects, .. }
        | AbilityDef::SagaChapter { effects, .. }
        | AbilityDef::Loyalty { effects, .. } => effects_read_foretold_count(effects),
        AbilityDef::ModalSpell { modes, .. }
        | AbilityDef::Modal { modes, .. }
        | AbilityDef::ModalActivated { modes, .. }
        | AbilityDef::ModalTriggered { modes, .. } => {
            modes.iter().any(|m| effects_read_foretold_count(m.effects))
        }
        _ => false,
    }
}

fn effects_read_foretold_count(effects: &'static [baylee_cards_dsl::Effect]) -> bool {
    let mut seen = 0;
    let mut found = false;
    baylee_cards_dsl::Effect::walk(effects, &mut seen, &mut |effect| {
        use baylee_cards_dsl::{Effect, Modifier};
        found |= match effect {
            Effect::CreateContinuousEffect {
                modifier:
                    Modifier::ForetellActionCostsLess {
                        first_each_turn_only: true,
                        ..
                    },
                ..
            }
            | Effect::AddTempModifier {
                modifier:
                    Modifier::ForetellActionCostsLess {
                        first_each_turn_only: true,
                        ..
                    },
                ..
            }
            | Effect::AddGlobalTempModifier {
                modifier:
                    Modifier::ForetellActionCostsLess {
                        first_each_turn_only: true,
                        ..
                    },
                ..
            } => true,
            Effect::CreateEmblem { abilities } => {
                abilities.iter().any(ability_reads_foretold_count)
            }
            _ => false,
        };
    });
    found
}

pub(crate) fn card_reads_first_discard_batch(def: &CardDef) -> bool {
    (0..def.faces.len().max(1)).any(|face| {
        def.abilities_for_face(face)
            .iter()
            .any(ability_reads_first_discard_batch)
    })
}

fn ability_reads_first_discard_batch(ability: &baylee_cards_dsl::AbilityDef) -> bool {
    use baylee_cards_dsl::{AbilityDef, Modifier, Trigger};
    match ability {
        AbilityDef::Triggered {
            trigger:
                Trigger::OneOrMoreCardsDiscarded {
                    first_this_turn: true,
                    ..
                },
            ..
        }
        | AbilityDef::ModalTriggered {
            trigger:
                Trigger::OneOrMoreCardsDiscarded {
                    first_this_turn: true,
                    ..
                },
            ..
        } => true,
        AbilityDef::Static(sa) => match sa.modifier {
            Modifier::GrantTriggered {
                trigger:
                    Trigger::OneOrMoreCardsDiscarded {
                        first_this_turn: true,
                        ..
                    },
                ..
            } => true,
            Modifier::GrantTriggeredDefinition(inner) => ability_reads_first_discard_batch(inner),
            _ => false,
        },
        AbilityDef::Spell { effects, .. }
        | AbilityDef::Triggered { effects, .. }
        | AbilityDef::Activated { effects, .. }
        | AbilityDef::ActivatedConditional { effects, .. }
        | AbilityDef::SagaChapter { effects, .. }
        | AbilityDef::Loyalty { effects, .. } => effects_read_first_discard_batch(effects),
        AbilityDef::ModalSpell { modes, .. }
        | AbilityDef::Modal { modes, .. }
        | AbilityDef::ModalActivated { modes, .. }
        | AbilityDef::ModalTriggered { modes, .. } => modes
            .iter()
            .any(|m| effects_read_first_discard_batch(m.effects)),
        _ => false,
    }
}

fn effects_read_first_discard_batch(effects: &'static [baylee_cards_dsl::Effect]) -> bool {
    let mut seen = 0;
    let mut found = false;
    baylee_cards_dsl::Effect::walk(effects, &mut seen, &mut |effect| {
        use baylee_cards_dsl::{DelayedTriggerWhen, Effect, Modifier, Trigger};
        found |= match effect {
            Effect::RegisterDelayedTrigger {
                when:
                    DelayedTriggerWhen::OnTrigger {
                        trigger:
                            Trigger::OneOrMoreCardsDiscarded {
                                first_this_turn: true,
                                ..
                            },
                        ..
                    },
                ..
            }
            | Effect::CreateContinuousEffect {
                modifier:
                    Modifier::GrantTriggered {
                        trigger:
                            Trigger::OneOrMoreCardsDiscarded {
                                first_this_turn: true,
                                ..
                            },
                        ..
                    },
                ..
            }
            | Effect::AddTempModifier {
                modifier:
                    Modifier::GrantTriggered {
                        trigger:
                            Trigger::OneOrMoreCardsDiscarded {
                                first_this_turn: true,
                                ..
                            },
                        ..
                    },
                ..
            }
            | Effect::AddGlobalTempModifier {
                modifier:
                    Modifier::GrantTriggered {
                        trigger:
                            Trigger::OneOrMoreCardsDiscarded {
                                first_this_turn: true,
                                ..
                            },
                        ..
                    },
                ..
            } => true,
            Effect::CreateContinuousEffect {
                modifier: Modifier::GrantTriggeredDefinition(inner),
                ..
            }
            | Effect::AddTempModifier {
                modifier: Modifier::GrantTriggeredDefinition(inner),
                ..
            }
            | Effect::AddGlobalTempModifier {
                modifier: Modifier::GrantTriggeredDefinition(inner),
                ..
            } => ability_reads_first_discard_batch(inner),
            Effect::CreateEmblem { abilities } => {
                abilities.iter().any(ability_reads_first_discard_batch)
            }
            Effect::Backup {
                grant_abilities, ..
            } => grant_abilities
                .iter()
                .any(ability_reads_first_discard_batch),
            _ => false,
        };
    });
    found
}
