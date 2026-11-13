//! Shared alternative-cost and modal options, including madness preflight.
//!
//! The trigger's offer and the casting wizard must enumerate the same modes:
//! a mode's own alternative cost cannot combine with madness (CR 118.9a).

#[allow(clippy::wildcard_imports)] // the casting wizard's vocabulary
use super::*;
use crate::state::GameState;

/// Price an alternative/front-face cost through the same increases and
/// reductions as a normal cast. Waiving mana cost does not waive a tax.
pub(super) fn front_spell_price<L: CardLookup>(
    state: &GameState,
    lookup: &L,
    player: PlayerId,
    card: ObjectId,
    cost: ManaCost,
) -> ManaCost {
    let Some(def) = state
        .object(card)
        .and_then(|object| object.card)
        .and_then(|card| lookup.card(card.index))
    else {
        return cost;
    };
    casting::face_adjustment(state, player, card, def, 0, None).price(
        cost,
        casting::printed_reduction(state, &def.faces[0], player, card),
    )
}

/// Whether the current pool, including spell-restricted mana and printed
/// reductions, covers a cost imposed by an alternative-cost cast.
pub(super) fn alternative_cost_affordable<L: CardLookup>(
    state: &GameState,
    lookup: &L,
    player: PlayerId,
    card: ObjectId,
    cost: ManaCost,
) -> bool {
    let reduction = state
        .object(card)
        .and_then(|object| object.card)
        .and_then(|card| lookup.card(card.index))
        .map_or(0, |def| {
            casting::keyword_reduction(state, &def.faces[0], player, card)
        });
    let restricted = casting::planning_pool(state, player, casting::SpendFor::Spell(card), 0);
    let pool = restricted
        .as_ref()
        .unwrap_or(&state.players[usize::from(player.get())].mana_pool);
    let what = casting::SpendFor::Spell(card);
    casting::source_affordable(
        state,
        player,
        what,
        pool,
        &cost.with_less_generic(reduction),
        &[],
        Some(casting::live_allowance(state, player, what).reduced(reduction)),
    )
}

/// Modal choices over one alternative base cost. A mode's own alternative
/// cost cannot combine with that base; its additional costs still can.
pub(super) fn modes_with_base<L: CardLookup>(
    state: &GameState,
    lookup: &L,
    player: PlayerId,
    card: ObjectId,
    def: &baylee_cards_dsl::CardDef,
    base: ManaCost,
    make_mana: bool,
) -> Vec<CastModeDesc> {
    let mut options = Vec::new();
    for ability in def.abilities_for_face(0) {
        let AbilityDef::ModalSpell { modes, choose } = ability else {
            continue;
        };
        if !choose.is_one() {
            // For a set of modes, each mode's own cost is additional to the
            // alternative base (CR 118.9d), not another alternative cost.
            for set in casting::mode_sets(modes, *choose) {
                let cost = front_spell_price(
                    state,
                    lookup,
                    player,
                    card,
                    casting::mode_set_cost(base, modes, set),
                );
                if (make_mana || alternative_cost_affordable(state, lookup, player, card, cost))
                    && casting::mode_set_has_legal_targets(state, lookup, player, card, set)
                {
                    options.push(CastModeDesc {
                        index: u32::try_from(options.len()).unwrap_or(u32::MAX),
                        kind: CastModeKind::Modes(set),
                        cost,
                    });
                }
            }
            continue;
        }
        for (i, mode) in modes.iter().enumerate() {
            let cost = front_spell_price(state, lookup, player, card, base);
            if mode.cost_override.is_none()
                && (make_mana || alternative_cost_affordable(state, lookup, player, card, cost))
                && casting::mode_has_a_legal_target(state, lookup, player, card, i)
            {
                options.push(CastModeDesc {
                    index: u32::try_from(options.len()).unwrap_or(u32::MAX),
                    kind: CastModeKind::Mode(i),
                    cost,
                });
            }
        }
    }
    options
}

/// Every legal option for this exact exiled incarnation's madness cast.
/// Both the trigger's offer and the accepted cast use this one preflight.
pub(crate) fn madness_options<L: CardLookup>(
    state: &GameState,
    lookup: &L,
    player: PlayerId,
    card: ObjectId,
    version: u32,
) -> Option<Vec<CastModeDesc>> {
    let object = state
        .object(card)
        .filter(|object| object.zone == crate::zone::Zone::Exile && object.version == version)?;
    if !casting::may_begin_casting(state, player)
        || casting::cast_is_forbidden(state, player, object)
        || object
            .characteristics()
            .types
            .contains(baylee_core::types::TypeSet::LAND)
    {
        return None;
    }
    let def = object.card.and_then(|card| lookup.card(card.index))?;
    let face = &def.faces[0];
    let madness = face.madness?;
    let choice = CostChoice {
        option: Some(CastModeKind::Normal),
        alternative_cost: Some(CastModeKind::Madness),
        buyback: false,
        kicked: false,
    };
    casting::additional_payment_capacity_for(state, player, card, face, choice, &[])?;
    let options = if casting::modes_are_the_only_way(def, 0) {
        // A madness cast may activate mana abilities after its choices
        // (CR 601.2g), so an empty pool does not erase an otherwise legal mode.
        modes_with_base(state, lookup, player, card, def, madness.mana, true)
    } else if casting::face_has_a_legal_target(state, lookup, player, card, 0) {
        vec![CastModeDesc {
            index: 0,
            kind: CastModeKind::Normal,
            cost: front_spell_price(state, lookup, player, card, madness.mana),
        }]
    } else {
        Vec::new()
    };
    (!options.is_empty()).then_some(options)
}
