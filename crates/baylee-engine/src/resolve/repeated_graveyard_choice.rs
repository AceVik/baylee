//! Repeated non-targeted graveyard choices, buying another choice with mana.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::ids::DamageSourceRef;
use baylee_core::mana::ManaCost;

/// All history belongs to this resolution, including exact card incarnations.
#[derive(Clone, Debug, Hash)]
pub struct Selection {
    /// Graveyard owner and recipient of the final chosen card.
    pub owner: PlayerId,
    /// Player making each non-targeted choice.
    pub chooser: PlayerId,
    /// Cost paid by the resolving controller for another choice.
    pub cost: ManaCost,
    /// Earlier choices cannot be selected again in this resolution.
    pub selected: Vec<DamageSourceRef>,
}

pub(super) fn start(
    state: &GameState,
    res: &mut Resolution,
    owner: PlayerRel,
    chooser: PlayerRel,
    cost: ManaCost,
) -> Option<Pending> {
    let owner = players_of(owner, state, res.controller, res)
        .first()
        .copied()?;
    let chooser = players_of(chooser, state, res.controller, res)
        .first()
        .copied()?;
    ask(
        state,
        res,
        Selection {
            owner,
            chooser,
            cost,
            selected: Vec::new(),
        },
    )
}

fn ask(state: &GameState, res: &mut Resolution, selection: Selection) -> Option<Pending> {
    let options: Vec<_> = state
        .zones
        .list(ZoneLocation::Graveyard(selection.owner))
        .iter()
        .copied()
        .filter(|id| {
            state.object(*id).is_some_and(|object| {
                object.is_card()
                    && !selection.selected.contains(&DamageSourceRef {
                        object: *id,
                        version: object.version,
                    })
            })
        })
        .collect();
    if options.is_empty() || state.has_left(selection.chooser) {
        return (!selection.selected.is_empty()).then(|| payment(state, res, selection));
    }
    let chooser = selection.chooser;
    res.awaiting = Some(AwaitingOp::RepeatedGraveyardChoice(Box::new(selection)));
    Some(Pending::ChooseCards {
        player: chooser,
        options,
        min: 1,
        max: 1,
        prompt: ChoicePrompt::PutIntoHand,
        total: None,
    })
}

fn payment(state: &GameState, res: &mut Resolution, selection: Selection) -> Pending {
    let cost = selection.cost;
    res.awaiting = Some(AwaitingOp::PlayerMayPay {
        player: res.controller,
        cost,
        outcome: ManaPaymentOutcome::RepeatGraveyardChoice(Box::new(selection)),
    });
    Pending::YesNo {
        player: res.controller,
        prompt: YesNoPrompt::PayMana { cost },
        source: resolving_ability(state, res),
    }
}

pub(super) fn chosen(
    state: &GameState,
    res: &mut Resolution,
    mut selection: Selection,
    chosen: &[ObjectId],
) -> Option<Pending> {
    if let Some(object) = chosen.first().and_then(|id| state.object(*id)) {
        selection.selected.push(DamageSourceRef {
            object: chosen[0],
            version: object.version,
        });
    }
    (!selection.selected.is_empty()).then(|| payment(state, res, selection))
}

pub(super) fn paid(
    state: &mut GameState,
    res: &mut Resolution,
    selection: Selection,
    paid: bool,
) -> Option<Pending> {
    if paid {
        return ask(state, res, selection);
    }
    let card = *selection.selected.last()?;
    if state.object(card.object).is_some_and(|object| {
        object.version == card.version
            && object.zone == crate::zone::Zone::Graveyard
            && object.zone_owner == Some(selection.owner)
    }) {
        return names::put_top(state, res, card.object, selection.owner, SearchDest::Hand);
    }
    None
}
