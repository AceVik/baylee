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
/// `Amount::LeftZoneThisTurn`).
pub(crate) fn card_reads_zone_changes(def: &CardDef) -> bool {
    card_reads_pred(def, &|_| false, &|amt| {
        matches!(
            amt,
            baylee_cards_dsl::Amount::EnteredThisTurn { .. }
                | baylee_cards_dsl::Amount::TokensCreatedThisTurn { .. }
                | baylee_cards_dsl::Amount::LeftZoneThisTurn { .. }
        )
    })
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
        | Amount::DistinctManaValuesInZone { filter: f, .. } => {
            filter_reads_cast_origin_zone(f, depth + 1)
        }
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
