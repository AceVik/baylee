//! Continuous permissions concerning the top card of a player's library.
//!
//! Every reader checks the current top card, never a card remembered below
//! it. Temporary effects remain after their source leaves the battlefield;
//! static permissions use the ordinary source-presence rule.

use crate::state::GameState;
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::Modifier;
use baylee_core::ids::{ObjectId, PlayerId};

fn permission(state: &GameState, player: PlayerId, modifier: Modifier) -> bool {
    state.effects.may_grant_graveyard_or_top_play()
        && state.effects.iter().any(|effect| {
            effect.controller == player
                && effect.modifier == modifier
                && crate::effects::source_is_present(state, effect)
        })
}

/// Whether `player` has the private, continuous right to see their own top.
#[must_use]
pub fn library_top_visible_to(state: &GameState, viewer: PlayerId, owner: PlayerId) -> bool {
    state.library_top_revealed(owner)
        || viewer == owner
            && state.effects.may_reveal_or_change_hand_size()
            && state.effects.iter().any(|effect| {
                effect.controller == owner
                    && effect.modifier == Modifier::LookAtLibraryTop
                    && crate::effects::source_is_present(state, effect)
            })
}

/// Whether the mana-value-for-life permission opens this exact library card.
#[must_use]
pub(crate) fn library_top_life_open(state: &GameState, player: PlayerId, card: ObjectId) -> bool {
    state.object(card).is_some_and(|object| {
        object.zone == Zone::Library
            && object.zone_owner == Some(player)
            && state.zones.list(ZoneLocation::Library(player)).last() == Some(&card)
    }) && permission(state, player, Modifier::PlayFromLibraryTopForManaValueLife)
}

/// Whether any spell permission opens this exact library card.
#[must_use]
pub(crate) fn library_top_spell_open(state: &GameState, player: PlayerId, card: ObjectId) -> bool {
    library_top_life_open(state, player, card)
}

/// Mana value of the prospective spell face. X contributes zero to a mana
/// cost's mana value; the alternative cost itself has no X to announce.
#[must_use]
pub(crate) fn library_top_life_value(def: &baylee_cards_dsl::CardDef, face: usize) -> u32 {
    def.faces[face].mana_cost.cmc()
}

/// All ways to cast the exact current top card for its mana value in life.
///
/// Only the remaining mana is described by `CastModeDesc::cost`; `life` is
/// frozen into the public route. A deferred mana window never defers life or
/// another non-mana cost, and all fixed life is checked before payment starts.
pub(crate) fn library_top_life_options(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
    paying: crate::casting::Paying,
) -> Result<Vec<crate::choice::CastModeDesc>, crate::casting::CastError> {
    use crate::casting::{CastError, additional_payment_capacity_for};
    use crate::choice::{CastModeDesc, CastModeKind};

    if !library_top_life_open(state, player, card) {
        return Err(CastError::NotInHand);
    }
    if !crate::casting::may_begin_casting(state, player) {
        return Err(CastError::Forbidden);
    }
    let Some(def) = state
        .object(card)
        .and_then(|object| object.card)
        .and_then(|card| lookup.card(card.index))
    else {
        return Err(CastError::NotInHand);
    };
    let choices = super::face_options::replaced_cost_options(
        state,
        lookup,
        player,
        card,
        paying,
        super::face_options::CostBasis::instead(baylee_core::mana::ManaCost::ZERO),
    );
    let mut options = Vec::new();
    let mut mana_short = false;
    let mut life_short = false;
    for choice in choices {
        let life = library_top_life_value(def, choice.face);
        let costs = choice.cost_choice(def);
        let fixed = crate::engine::cast_wizard::fixed_life_cost(&def.faces[choice.face], costs);
        let total_life = i32::try_from(life)
            .unwrap_or(i32::MAX)
            .saturating_add(fixed);
        if !state.can_pay_life(player, total_life) {
            life_short = true;
            continue;
        }
        if additional_payment_capacity_for(state, player, card, &def.faces[choice.face], costs, &[])
            .is_none()
        {
            continue;
        }
        if !choice.mana_ready {
            mana_short = true;
            continue;
        }
        options.push(CastModeDesc {
            index: u32::try_from(options.len()).expect("bounded cast options"),
            kind: CastModeKind::LibraryTopLife {
                life,
                face: choice.face,
                modes: choice.modes,
                kicked: choice.kicked,
            },
            cost: choice.cost,
        });
    }
    if options.is_empty() {
        return Err(if life_short {
            CastError::NotEnoughLife
        } else if mana_short {
            CastError::NotEnoughMana
        } else {
            CastError::NoWayToCast
        });
    }
    Ok(options)
}
