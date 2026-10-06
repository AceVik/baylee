//! The mana a land's basic land types give it (CR 305.6), and activating mana abilities.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// Every mana color a land's **basic types** entitle it to (CR 305.6), in
/// the rules' own order.
///
/// One entry per basic type present, because CR 305.6 gives the land one
/// mana ability per type and the controller picks which to activate. Empty
/// for a land with no basic type and for anything that is not a land.
///
/// **A land can gain a basic type it was not printed with**, and that is why
/// this returns a list rather than an `Option`. It used to answer `None` for
/// a land with several — deliberately, so that Godless Shrine would not tap
/// for white and never for black — and the dual was left to the
/// `AddManaChoice` ability printed on its card, which does ask. That works
/// for a card whose *printed* type line carries both types. It cannot work
/// for a type a continuous effect adds: a basic Forest under Urborg, Tomb of
/// Yawgmoth is a Forest Swamp, and the only ability it prints is the green
/// one, so the black CR 305.6 gives it existed nowhere. The land kept making
/// green and silently made no black at all — which is the shape of this
/// defect and worth stating precisely, because "Urborg turns off your
/// lands" would have been the wrong reading and the wrong fix. Four cards in
/// this pool add a basic land type: Urborg, Yavimaya, Blanket of Night and
/// Ashaya.
#[must_use]
pub fn intrinsic_mana_colors(state: &GameState, source: ObjectId) -> Vec<ManaColor> {
    let Some(obj) = state.object(source) else {
        return Vec::new();
    };
    if !obj.characteristics().types.contains(TypeSet::LAND)
        || obj.characteristics().abilities_lost.is_some()
    {
        return Vec::new();
    }
    let s = &obj.characteristics().subtypes;
    [
        (land::PLAINS, ManaColor::White),
        (land::ISLAND, ManaColor::Blue),
        (land::SWAMP, ManaColor::Black),
        (land::MOUNTAIN, ManaColor::Red),
        (land::FOREST, ManaColor::Green),
    ]
    .into_iter()
    .filter(|(subtype, _)| s.contains(*subtype))
    .map(|(_, color)| color)
    .collect()
}

/// Colors available through the unindexed intrinsic shortcut. A multi-type
/// land with an explicit intrinsic entry uses that entry's color choice; a
/// printed fixed-symbol ability never removes a type-derived alternative.
#[must_use]
pub fn intrinsic_mana_offer(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    source: ObjectId,
) -> Vec<ManaColor> {
    let colors = intrinsic_mana_colors(state, source);
    if colors.len() > 1
        && state.object(source).is_some_and(|obj| {
            obj.abilities(lookup)
                .iter()
                .any(baylee_cards_dsl::AbilityDef::is_intrinsic_mana_ability)
        })
    {
        return Vec::new();
    }
    colors
}

/// The colours `player` may tap `source` for through the CR 305.6 shortcut
/// right now, and so what `PlayerAction::ActivateManaAbility` does with it:
/// empty when the shortcut is closed (a land it cannot activate now, or one
/// whose own card prints every colour its types give it), one colour to add,
/// several to ask.
///
/// The one predicate for the offer (`legal.mana_abilities`) and for `apply`.
/// They used to ask two: the offer this, and `apply` only
/// [`can_activate_mana`]. A dual land prints its own "Add {G} or {U}", so its
/// shortcut is empty and it is offered through that printed ability; under
/// Chromatic Lantern it is also in `mana_abilities` for the granted "{T}: Add
/// one mana of any color". `apply` saw an untapped land with basic types,
/// took the shortcut, found no colour in it and refused the press the offer
/// had just listed (Breeding Pool, Stomping Ground, Canopy Vista: 63 refusals
/// in 10,000 fuzzed games), where the granted ability was the one to take.
#[must_use]
pub fn intrinsic_mana_choices(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    source: ObjectId,
) -> Vec<ManaColor> {
    if !can_activate_mana(state, player, source) {
        return Vec::new();
    }
    intrinsic_mana_offer(state, lookup, source)
}

/// The one color a land's basic types entitle it to, where there is exactly
/// one and so nothing to ask.
///
/// `None` where the land has several, which is a question and not an answer;
/// [`intrinsic_mana_colors`] is what a caller that can ask reads instead.
#[must_use]
pub fn intrinsic_mana(state: &GameState, source: ObjectId) -> Option<ManaColor> {
    match intrinsic_mana_colors(state, source).as_slice() {
        [only] => Some(*only),
        _ => None,
    }
}

pub(crate) fn intrinsic_mana_price(state: &GameState, source: ObjectId) -> ManaCost {
    ManaCost::ZERO.with_more_generic(activation_increase(state, source))
}

pub(crate) fn pay_intrinsic_mana_price(
    state: &mut GameState,
    player: PlayerId,
    source: ObjectId,
) -> bool {
    let cost = intrinsic_mana_price(state, source);
    pay_mana_for(state, player, SpendFor::Ability(source), &cost).is_some()
}

/// Whether the intrinsic mana ability of `source` can be activated now.
#[must_use]
pub fn can_activate_mana(state: &GameState, player: PlayerId, source: ObjectId) -> bool {
    let Some(obj) = state.object(source) else {
        return false;
    };
    obj.zone == Zone::Battlefield
        && obj.controller == player
        && !obj.status.contains(crate::object::Status::TAPPED)
        // A land is never summoning sick, so this costs an ordinary land
        // nothing; it is here for the land that is also a creature (Dryad
        // Arbor, an animated manland), whose intrinsic {T} is an activated
        // ability of a creature like any other (CR 302.6).
        && !crate::combat::summoning_sick(state, obj)
        && !intrinsic_mana_colors(state, source).is_empty()
        && affordable(state, player, &state.players[player.get() as usize].mana_pool,
            &intrinsic_mana_price(state, source))
}

/// Taps a basic land for its intrinsic mana (CR 305.6).
///
/// # Errors
/// [`CastFailure::NoSuchObject`] for stale handles.
///
/// # Panics
/// Internal invariant violations (legality checked first).
pub fn activate_mana(
    state: &mut GameState,
    player: PlayerId,
    source: ObjectId,
) -> Result<(), CastFailure> {
    if !can_activate_mana(state, player, source) {
        return Err(CastFailure::Legality(CastError::BadTiming));
    }
    let colors = intrinsic_mana_colors(state, source);
    let [color] = colors.as_slice() else {
        // Several basic types is a question, and this function cannot ask
        // one: `actions.rs` taps the land and publishes `Pending::ChooseColor`
        // instead, then finishes here through `add_intrinsic_mana`. Reaching
        // this arm means a caller skipped that fork.
        return Err(CastFailure::Legality(CastError::BadTiming));
    };
    if !pay_intrinsic_mana_price(state, player, source) {
        return Err(CastFailure::Legality(CastError::NotEnoughMana));
    }
    add_intrinsic_mana(state, player, source, *color);
    Ok(())
}

/// Taps a land for one mana of `color` and pays it into the pool — the half
/// of [`activate_mana`] that happens once the colour is settled.
///
/// Its own function because the colour arrives two ways: straight out of the
/// land's one basic type, or out of a `Pending::ChooseColor` the player
/// answered because the land has several. One door, so the two cannot come
/// out as different events — the tap is journalled under [`Cause::Cost`] and
/// the mana under [`GameEvent::ManaProduced`] either way, and a trigger
/// watching for either sees the same thing.
pub fn add_intrinsic_mana(
    state: &mut GameState,
    player: PlayerId,
    source: ObjectId,
    color: ManaColor,
) {
    if state.players[player.get() as usize]
        .mana_pool
        .available(color)
        == u32::MAX
    {
        state.numeric_failure = Some("mana production exceeds u32 per color");
        return;
    }
    let obligation_before = state
        .constrained_payment(player)
        .map(|_| state.players[usize::from(player.get())].mana_pool.clone());
    state.set_tapped(source, true);
    state.journal.record(GameEvent::ObjectTapped {
        object: source,
        cause: Cause::Cost,
    });
    let snow = state.object(source).is_some_and(|o| {
        o.characteristics()
            .supertypes
            .contains(baylee_core::types::SupertypeSet::SNOW)
    });
    if snow {
        state.players[player.get() as usize]
            .mana_pool
            .add_snow(color, 1);
    } else {
        state.players[player.get() as usize].mana_pool.add(color, 1);
    }
    if let Some(before) = obligation_before {
        state.note_constrained_production(player, &before);
    }
    state.journal.record(GameEvent::ManaProduced {
        player,
        color,
        amount: 1,
        source: Some(source),
    });
}
