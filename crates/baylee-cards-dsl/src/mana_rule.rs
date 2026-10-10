//! The activated-mana-ability criteria shared by execution and validation.

use crate::{Cost, CostPart, Effect, SearchDest};

/// Whether a nonloyalty activated ability meets CR 605.1a.
///
/// Read its own cost and immediate effects, including conditional branches.
/// Other abilities it grants or schedules are separate abilities. External
/// replacements are deliberately not evaluated; a card's self-replacement
/// must be represented in the immediate effect being classified.
#[must_use]
pub fn activated_mana_ability(cost: &Cost, effects: &[Effect], targets: bool) -> bool {
    !targets
        && !cost.parts.iter().any(cost_moves_library)
        && could_add_mana(effects)
        && !effects.iter().any(moves_library)
}

/// Whether an immediate branch could add mana, independently of its targets
/// or library movements. Triggered mana abilities use the different CR 605.1b.
#[must_use]
pub fn could_add_mana(effects: &[Effect]) -> bool {
    effects.iter().any(|effect| {
        matches!(
            effect,
            Effect::AddMana { .. } | Effect::AddManaFor { .. } | Effect::AddManaLikeEvent { .. }
        ) || {
            let (then, otherwise) = immediate_branches(effect);
            could_add_mana(then) || could_add_mana(otherwise)
        }
    })
}

fn immediate_branches(effect: &Effect) -> (&'static [Effect], &'static [Effect]) {
    match effect {
        Effect::Reflexive { .. }
        | Effect::ScheduleLinkedCounterCleanup { .. }
        | Effect::AtNextEndStep { .. }
        | Effect::AtEndOfCombat { .. } => (&[], &[]),
        _ => effect.branches(),
    }
}

// Exhaustive: a future library-payment part must make an explicit decision.
fn cost_moves_library(part: &CostPart) -> bool {
    match part {
        CostPart::TapSelf
        | CostPart::UntapSelf
        | CostPart::SacrificeSelf
        | CostPart::Sacrifice(_)
        | CostPart::PayLife(_)
        | CostPart::Discard(_)
        | CostPart::TapOther(_)
        | CostPart::Crew(_)
        | CostPart::DiscardSelf
        | CostPart::ExileSelf
        | CostPart::ReturnSelfToHand
        | CostPart::ReturnToHand(_)
        | CostPart::ExileFromGraveyard(_)
        | CostPart::ExileFromHand(_)
        | CostPart::PayLifeX
        | CostPart::RemoveCounterSelf { .. }
        | CostPart::RemoveCounterSelfX { .. }
        | CostPart::PutCounterSelf { .. } => false,
    }
}

fn outside_library(destination: SearchDest) -> bool {
    !matches!(destination, SearchDest::TopOfLibrary)
}

fn moves_library(effect: &Effect) -> bool {
    match effect {
        Effect::DrawCards { .. }
        | Effect::DrawCardsFor { .. }
        | Effect::DrawRevealDiscardUnless { .. }
        | Effect::DiscardUpToThenDraw { .. }
        | Effect::LookAtTopPick { .. }
        | Effect::LookAtTopKeepBottomPlay { .. }
        | Effect::RevealAndSeparate { .. }
        | Effect::MillMayTakeOne { .. }
        | Effect::Cascade
        | Effect::Discover { .. }
        | Effect::ExileTopMayCast { .. }
        | Effect::RevealTopOnePerType { .. }
        | Effect::PutFromHandOnTop { .. }
        | Effect::PayLifeOrPutBackDrawn { .. }
        | Effect::PutTargetOnBottomOfLibrary
        | Effect::PutOnBottomOfLibraryFromGraveyard { .. }
        | Effect::OwnerPutsOnTopOrBottom { .. }
        | Effect::ShuffleGraveyardIntoLibrary
        | Effect::ShuffleIntoLibrary { .. }
        | Effect::Surveil { .. }
        | Effect::ExileLibraryAndShuffleHand { .. }
        | Effect::Mill { .. }
        | Effect::GraveyardToTop { .. }
        | Effect::PutSourceOnTopOfLibrary
        | Effect::BottomCardFromHand { .. }
        | Effect::OptionalBasicLandSearchFor { .. }
        | Effect::SearchOpponentSplits { .. }
        | Effect::SearchLibraryOrGraveyard { .. } => true,
        Effect::RevealUntil { found, .. } => outside_library(*found),
        Effect::RevealTopAndSort {
            matched, otherwise, ..
        } => outside_library(*matched) || outside_library(*otherwise),
        Effect::LookAtTopMayPut {
            matched, otherwise, ..
        } => outside_library(matched.dest) || outside_library(*otherwise),
        Effect::SearchLibraryUpTo { find, .. } => outside_library(find.dest),
        Effect::SearchLibrary { finds, .. } | Effect::SearchLibraryOf { finds, .. } => {
            finds.iter().any(|find| outside_library(find.dest))
        }
        // Scry, reordering, revealing and shuffling alone do not move a card
        // to/from this zone. They can nevertheless be irreversible under
        // CR 732.1; that is a separate question from being a mana ability.
        _ => {
            let (then, otherwise) = immediate_branches(effect);
            then.iter().chain(otherwise).any(moves_library)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Amount, ManaColor, PlayerRel};

    const ADD: Effect = Effect::mana(ManaColor::Blue, 1);

    #[test]
    fn direct_library_movement_excludes_printed_and_nested_mana_effects() {
        const NESTED: &[Effect] = &[
            ADD,
            Effect::MayDo {
                effects: &[Effect::draw(1)],
            },
        ];
        for library in [
            Effect::draw(1),
            Effect::Surveil {
                amount: Amount::Fixed(1),
            },
            Effect::ShuffleGraveyardIntoLibrary,
        ] {
            assert!(!activated_mana_ability(&Cost::TAP, &[ADD, library], false));
        }
        assert!(!activated_mana_ability(&Cost::TAP, NESTED, false));
        assert!(!activated_mana_ability(&Cost::TAP, &[ADD], true));
    }

    #[test]
    fn nonmoving_riders_and_separately_defined_abilities_do_not_change_classification() {
        const RIDERS: &[Effect] = &[
            ADD,
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: crate::TargetSpec::Player(PlayerRel::You),
            },
            Effect::Scry {
                amount: Amount::Fixed(1),
            },
            Effect::ShuffleLibrary {
                who: PlayerRel::You,
            },
            Effect::AtNextEndStep {
                effects: &[Effect::draw(1)],
            },
        ];
        assert!(activated_mana_ability(&Cost::TAP, RIDERS, false));
        assert!(!activated_mana_ability(
            &Cost::TAP,
            &[Effect::AtNextEndStep { effects: &[ADD] }],
            false
        ));
    }
}
