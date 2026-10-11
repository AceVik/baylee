//! Prospective spell permissions and prices for exiled cards.
#[allow(clippy::wildcard_imports)]
use super::*;

/// Evaluate an exile permission against the prospective spell (CR 601.3e).
/// With no such permission, the other zone permissions retain their own rules.
pub(crate) fn permitted_spell_types(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    types: baylee_core::types::TypeSet,
    face: usize,
) -> bool {
    let Some(object) = state.object(card) else {
        return false;
    };
    if object.zone == Zone::Hand
        || (face == 0 && adventure_permission(state, player, card))
        || object
            .riders
            .contains(&crate::object::Rider::PlayableFromExileFor(player))
    {
        return true;
    }
    let mut found = false;
    for permission in &state.play_permissions {
        if permission.player == player
            && permission.card == card
            && permission.version == object.version
        {
            found = true;
            if permission
                .spell_types
                .is_none_or(|allowed| allowed.intersects(types))
            {
                return true;
            }
        }
    }
    !found
}

/// The mana price after waiving a face's mana cost, before convoke/delve.
pub(crate) fn free_face_price(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &baylee_cards_dsl::CardDef,
    face: usize,
) -> baylee_core::mana::ManaCost {
    super::face_adjustment(state, player, card, def, face, None).price(
        baylee_core::mana::ManaCost::ZERO,
        super::printed_reduction(state, &def.faces[face], player, card),
    )
}

pub(crate) fn free_face_affordable(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &baylee_cards_dsl::CardDef,
    face: usize,
) -> bool {
    let restricted =
        super::planning_pool(state, player, super::SpendFor::SpellOnFace(card, face), 0);
    let pool = restricted
        .as_ref()
        .unwrap_or(&state.players[usize::from(player.get())].mana_pool);
    let cost = free_face_price(state, player, card, def, face).with_less_generic(
        super::keyword_reduction(state, &def.faces[face], player, card),
    );
    super::affordable(state, player, pool, &cost)
        && !crate::engine::cast_wizard::additional_sacrifices(&def.faces[face])
            .any(|part| crate::engine::cost_wizard::options(state, player, card, part).is_empty())
}
