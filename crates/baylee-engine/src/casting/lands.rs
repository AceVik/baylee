//! Playing lands, and the permissions to play cards from other zones.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// How many lands `player` may play this turn (CR 305.2).
///
/// One, "however, continuous effects may increase this number" — which the
/// rule says in those words, so the number is read off the effect table
/// rather than being a constant with an exception bolted to it. Two such
/// effects add up: nothing in CR 305.2 makes them redundant with each other
/// the way two copies of a keyword are.
///
/// Saturating, because the sum is a `u8` and what it feeds is "may I play
/// one more": a table holding 255 extra land drops is a player who may play
/// lands all day either way, and wrapping to nought is the one answer that
/// would be wrong.
#[must_use]
pub fn land_drops_allowed(state: &GameState, player: PlayerId) -> u8 {
    state
        .effects
        .iter()
        .filter(|fx| fx.controller == player)
        .fold(1u8, |total, fx| match fx.modifier {
            baylee_cards_dsl::Modifier::ExtraLandDrops(n) => total.saturating_add(n),
            _ => total,
        })
}

/// Whether `player` has a land drop left this turn (CR 305.2a).
///
/// One predicate with two ends asking it: the offer in
/// `abilities::compute_legal`, which decides whether a land is in
/// `legal.lands` at all, and [`play_land`], which refuses an answer nobody
/// offered. Written out they were `== 0` and `>= 1` — the same sentence
/// exactly once, so the moment the limit stopped being one, one of the two
/// would have gone on reading the old rule and the difference would show as
/// a land the engine offers and then refuses.
#[must_use]
pub fn has_a_land_drop_left(state: &GameState, player: PlayerId) -> bool {
    state.players[player.get() as usize].lands_played_this_turn < land_drops_allowed(state, player)
}

/// The permanent types a spell can be cast "of" under Muldrotha's allowance:
/// every permanent type but land, which is played and not cast (CR 305.9).
pub(super) const SPELL_PERMANENT_TYPES: [TypeSet; 5] = [
    TypeSet::ARTIFACT,
    TypeSet::CREATURE,
    TypeSet::ENCHANTMENT,
    TypeSet::PLANESWALKER,
    TypeSet::BATTLE,
];

/// Whether each card in `cards` can be given a permanent type of its own,
/// no two the same: "a permanent spell of each permanent type", with a card
/// of several types using one of them. Five types at most, so the search is
/// small enough to try every assignment.
pub(super) fn one_type_each(cards: &[TypeSet]) -> bool {
    fn assign(cards: &[TypeSet], used: u8) -> bool {
        let Some((first, rest)) = cards.split_first() else {
            return true;
        };
        SPELL_PERMANENT_TYPES.iter().enumerate().any(|(i, t)| {
            used & (1 << i) == 0 && first.contains(*t) && assign(rest, used | (1 << i))
        })
    }
    cards.len() <= SPELL_PERMANENT_TYPES.len() && assign(cards, 0)
}

/// Which permission lets a card be played from its owner's graveyard, when
/// the one that does is counted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GraveyardPermission {
    /// Uncounted: Wrenn and Realmbreaker's emblem, Crucible of Worlds.
    Unlimited,
    /// Muldrotha's allowance, which this play uses up a part of.
    EachType {
        /// The allowance's source.
        source: ObjectId,
        /// Its version.
        version: u32,
    },
}

/// The Muldrotha-style allowances `player` holds right now: during their own
/// turn only ("during each of your turns"), each with the plays already made
/// under it this turn.
pub(super) fn each_type_allowances(
    state: &GameState,
    player: PlayerId,
) -> impl Iterator<Item = (ObjectId, u32, Vec<TypeSet>)> + '_ {
    let my_turn = state.turn.active == player;
    state
        .effects
        .iter()
        .filter(move |fx| {
            my_turn
                && fx.controller == player
                && matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::PermanentOfEachTypeFromGraveyard
                )
        })
        .filter_map(move |fx| {
            let source = fx.source?;
            let version = state.object(source)?.version;
            let plays = state
                .per_turn
                .graveyard_plays
                .iter()
                .filter(|p| p.player == player && p.source == source && p.version == version)
                .map(|p| p.types)
                .collect();
            Some((source, version, plays))
        })
}

/// Which permission, if any, lets `player` cast the card `obj` from their
/// own graveyard: Forgotten Cellar's first, the one that casts any spell;
/// then, for a permanent card, Wrenn's emblem, because it costs nothing to
/// use, and a Muldrotha allowance with a type still open for it. A land card
/// is played and never cast (CR 305.9), whichever permission is there.
#[must_use]
pub fn graveyard_cast_permission(
    state: &GameState,
    player: PlayerId,
    obj: &crate::object::GameObject,
) -> Option<GraveyardPermission> {
    let types = obj.characteristics().types;
    if obj.zone != Zone::Graveyard
        || obj.zone_owner != Some(player)
        || types.contains(TypeSet::LAND)
    {
        return None;
    }
    // "You may cast spells from your graveyard this turn": every spell, so
    // it is asked before the word "permanent" is.
    if state.effects.iter().any(|fx| {
        fx.controller == player
            && matches!(
                fx.modifier,
                baylee_cards_dsl::Modifier::CastSpellsFromGraveyard
            )
    }) {
        return Some(GraveyardPermission::Unlimited);
    }
    if !types.is_permanent() {
        return None;
    }
    if state.effects.iter().any(|fx| {
        fx.controller == player
            && matches!(
                fx.modifier,
                baylee_cards_dsl::Modifier::CastPermanentSpellsFromGraveyard
            )
    }) {
        return Some(GraveyardPermission::Unlimited);
    }
    each_type_allowances(state, player).find_map(|(source, version, mut plays)| {
        plays.retain(|t| !t.contains(TypeSet::LAND));
        plays.push(types);
        one_type_each(&plays).then_some(GraveyardPermission::EachType { source, version })
    })
}

/// Which permission lets `player` play a land from their own graveyard:
/// Crucible of Worlds' first, then a Muldrotha allowance whose land is still
/// unplayed this turn.
#[must_use]
pub fn graveyard_land_permission(
    state: &GameState,
    player: PlayerId,
) -> Option<GraveyardPermission> {
    if state.effects.iter().any(|fx| {
        fx.controller == player
            && matches!(
                fx.modifier,
                baylee_cards_dsl::Modifier::PlayLandsFromGraveyard
            )
    }) {
        return Some(GraveyardPermission::Unlimited);
    }
    each_type_allowances(state, player).find_map(|(source, version, plays)| {
        (!plays.iter().any(|t| t.contains(TypeSet::LAND)))
            .then_some(GraveyardPermission::EachType { source, version })
    })
}

/// Writes down a play made under a Muldrotha allowance.
pub fn note_graveyard_play(
    state: &mut GameState,
    player: PlayerId,
    permission: Option<GraveyardPermission>,
    types: TypeSet,
) {
    if let Some(GraveyardPermission::EachType { source, version }) = permission {
        state
            .per_turn
            .graveyard_plays
            .push(crate::state::GraveyardPlay {
                player,
                source,
                version,
                types,
            });
    }
}

/// Whether a land sitting in `zone` is one `player` may play (CR 305.1, and
/// the permissions that widen it).
///
/// The hand is the rules' own answer and needs no effect. The graveyard is
/// Crucible of Worlds and Ramunap Excavator, and it is a **permission**
/// rather than a second way of casting: `Modifier::GrantsFlashback` is the
/// neighbouring sentence about a graveyard and says nothing at all here,
/// because playing a land is not casting a spell (CR 305.1).
#[must_use]
pub fn land_zone_open(state: &GameState, player: PlayerId, zone: Zone) -> bool {
    match zone {
        Zone::Hand => true,
        Zone::Graveyard => graveyard_land_permission(state, player).is_some(),
        Zone::Library => state.effects.iter().any(|fx| {
            fx.controller == player
                && matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::PlayLandsFromLibraryTop
                )
        }),
        _ => false,
    }
}

/// The cards `player` may exile to pay `card`'s escape cost: every other
/// card in their graveyard (CR 702.138a, "exile [N] other cards"). The same
/// list for the offer's count and the cast wizard's question.
#[must_use]
pub fn escape_exile_options(state: &GameState, player: PlayerId, card: ObjectId) -> Vec<ObjectId> {
    state
        .zones
        .list(ZoneLocation::Graveyard(player))
        .iter()
        .copied()
        .filter(|id| *id != card)
        .collect()
}

/// The permission `player` holds to play `card` this turn, if any
/// ([`crate::state::PlayPermission`]): one given for this very object, so a
/// card that has moved since holds none (CR 400.7).
#[must_use]
pub fn play_permission(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
) -> Option<crate::state::PlayPermission> {
    let version = state.object(card)?.version;
    state
        .per_turn
        .playable
        .iter()
        .copied()
        .find(|p| p.player == player && p.card == card && p.version == version)
}

/// The zone permission plus the particular card restriction. A library
/// permission never grants access to a card below the top. A permission for
/// the card itself ([`play_permission`]) opens it wherever it lies, an
/// opponent's exile included — unless it lets the card be cast and nothing
/// else (Ragavan, Nimble Pilferer), which plays no land (CR 601.1a).
#[must_use]
pub fn land_card_open(state: &GameState, player: PlayerId, card: ObjectId) -> bool {
    let version = state.object(card).map(|o| o.version);
    if state
        .per_turn
        .playable
        .iter()
        .any(|p| p.player == player && p.card == card && Some(p.version) == version && !p.cast_only)
    {
        return true;
    }
    state.object(card).is_some_and(|obj| {
        obj.zone_owner == Some(player)
            && land_zone_open(state, player, obj.zone)
            && (obj.zone != Zone::Library
                || state.zones.list(ZoneLocation::Library(player)).last() == Some(&card))
    })
}

/// Plays a land (special action, no stack).
///
/// # Errors
/// [`CastFailure`] when the action is illegal.
///
/// # Panics
/// Internal invariant violations (existence validated first).
pub fn play_land(
    state: &mut GameState,
    player: PlayerId,
    card: ObjectId,
) -> Result<(), CastFailure> {
    play_land_with_timing(state, player, card, false)
}

/// A resolving instruction waives priority/main-phase timing, not the turn or allowance.
pub(crate) fn play_land_by_effect(
    state: &mut GameState,
    player: PlayerId,
    card: ObjectId,
) -> Result<(), CastFailure> {
    play_land_with_timing(state, player, card, true)
}

pub(super) fn play_land_with_timing(
    state: &mut GameState,
    player: PlayerId,
    card: ObjectId,
    by_effect: bool,
) -> Result<(), CastFailure> {
    let obj = state.object(card).ok_or(CastFailure::NoSuchObject)?;
    if !land_card_open(state, player, card) {
        return Err(CastFailure::Legality(CastError::NotInHand));
    }
    if !obj.characteristics().types.contains(TypeSet::LAND) {
        return Err(CastFailure::Legality(CastError::BadTiming));
    }
    let main_phase = matches!(state.turn.phase, Phase::FirstMain | Phase::SecondMain);
    if state.turn.active != player || (!by_effect && (!main_phase || !state.zones.stack_is_empty()))
    {
        return Err(CastFailure::Legality(CastError::BadTiming));
    }
    if !has_a_land_drop_left(state, player) {
        return Err(CastFailure::Legality(CastError::BadTiming));
    }
    // A land from the graveyard under Muldrotha's allowance uses its land
    // for the turn; one a permission for this very card opened does not.
    if obj.zone == Zone::Graveyard && play_permission(state, player, card).is_none() {
        let permission = graveyard_land_permission(state, player);
        note_graveyard_play(state, player, permission, TypeSet::LAND);
    }
    state.players[player.get() as usize].lands_played_this_turn += 1;
    {
        let obj = state.object_mut(card).expect("validated");
        obj.kind = ObjectKind::Permanent;
        obj.set_controller(player);
    }
    state
        .move_object(
            card,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .map_err(CastFailure::State)?;
    state.journal.record(GameEvent::LandPlayed {
        object: card,
        player,
    });
    Ok(())
}
