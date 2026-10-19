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
    if !state.effects.may_grant_graveyard_or_top_play() {
        return 1;
    }
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
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum GraveyardPermission {
    /// Uncounted: Wrenn and Realmbreaker's emblem, Crucible of Worlds.
    Unlimited,
    /// A once-per-turn static allowance or a single resolved permission.
    Once {
        /// Exact effect identity; reacquiring an ability creates a new allowance.
        effect: baylee_core::ids::EffectId,
        /// Source shown when choosing between permissions.
        source: Option<ObjectId>,
        /// The Tomb's permission adds Vampire and an entry finality counter.
        vampire: bool,
    },
    /// Flashback granted at the card's mana cost, chosen independently of Kess.
    Flashback,
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
    graveyard_cast_permissions(state, player, obj)
        .into_iter()
        .next()
}

/// All applicable permissions. Choosing one must not consume another's allowance.
#[must_use]
pub fn graveyard_cast_permissions(
    state: &GameState,
    player: PlayerId,
    obj: &crate::object::GameObject,
) -> Vec<GraveyardPermission> {
    use baylee_cards_dsl::Modifier;
    if !state.effects.may_grant_graveyard_or_top_play() {
        return Vec::new();
    }
    let types = obj.characteristics().types;
    if obj.zone != Zone::Graveyard
        || obj.zone_owner != Some(player)
        || types.contains(TypeSet::LAND)
    {
        return Vec::new();
    }
    // "You may cast spells from your graveyard this turn": every spell, so
    // it is asked before the word "permanent" is.
    let mut permissions = Vec::new();
    if state.effects.iter().any(|fx| {
        fx.controller == player
            && crate::effects::matches_filter(state, fx, obj)
            && matches!(
                fx.modifier,
                baylee_cards_dsl::Modifier::CastSpellsFromGraveyard
            )
    }) {
        permissions.push(GraveyardPermission::Unlimited);
    }
    if types.is_permanent()
        && state.effects.iter().any(|fx| {
            fx.controller == player
                && crate::effects::matches_filter(state, fx, obj)
                && matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::CastPermanentSpellsFromGraveyard
                )
        })
        && !permissions.contains(&GraveyardPermission::Unlimited)
    {
        permissions.push(GraveyardPermission::Unlimited);
    }
    if types.is_permanent() {
        permissions.extend(each_type_allowances(state, player).filter_map(
            |(source, version, mut plays)| {
                plays.retain(|t| !t.contains(TypeSet::LAND));
                plays.push(types);
                one_type_each(&plays).then_some(GraveyardPermission::EachType { source, version })
            },
        ));
    }
    for fx in state.effects.iter().filter(|fx| fx.controller == player) {
        let vampire = fx.modifier == Modifier::GraveyardVampireOnce;
        let matches = if vampire {
            types.contains(TypeSet::CREATURE)
        } else {
            fx.modifier == Modifier::GraveyardInstantSorceryOnce
                && state.turn.active == player
                && types.intersects(TypeSet::INSTANT.union(TypeSet::SORCERY))
                && !state
                    .per_turn
                    .graveyard_allowances_used
                    .contains(&(player, fx.id))
        };
        if matches
            && crate::effects::source_is_present(state, fx)
            && crate::effects::matches_filter(state, fx, obj)
        {
            permissions.push(GraveyardPermission::Once {
                effect: fx.id,
                source: fx.source,
                vampire,
            });
        }
    }
    permissions
}

/// Read a graveyard allowance using the characteristics of the announced face.
pub(crate) fn graveyard_face_permissions(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    face: usize,
) -> Vec<GraveyardPermission> {
    if !state.effects.may_grant_graveyard_or_top_play() {
        return Vec::new();
    }
    let what = if face == 0 {
        SpendFor::Spell(card)
    } else {
        SpendFor::SpellOnFace(card, face)
    };
    payment::spending_object(state, what).map_or_else(Vec::new, |object| {
        graveyard_cast_permissions(state, player, &object)
    })
}

/// Whether any ordinary or printed graveyard route can announce this face.
pub(crate) fn graveyard_face_open(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    face: usize,
) -> bool {
    use crate::state::CardLookup;
    let Some(object) = state.object(card) else {
        return false;
    };
    if object.zone != Zone::Graveyard || play_permission(state, player, card).is_some() {
        return true;
    }
    if object.zone_owner != Some(player) {
        return false;
    }
    !graveyard_face_permissions(state, player, card, face).is_empty()
        || !escape_grants(state, player, card, face).is_empty()
        || (face == 0 && flashback_granted(state, card))
        || object
            .card
            .and_then(|card| state.bases.card(card.index))
            .and_then(|def| def.faces.get(face))
            .is_some_and(|face| {
                face.disturb
                    || face.flashback.is_some()
                    || face.escape.is_some()
                    || face.beam_me_up.is_some()
            })
}

/// Beam me up's nonmana cost, shared by its offer, question and payment.
pub(crate) const BEAM_RETURN: baylee_cards_dsl::CostPart =
    baylee_cards_dsl::CostPart::ReturnToHand(&baylee_cards_dsl::Filter::CREATURE);

pub(crate) fn beam_return_options(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
) -> Vec<ObjectId> {
    crate::engine::cost_wizard::options(state, player, card, &BEAM_RETURN)
}

/// Which permission lets `player` play a land from their own graveyard:
/// Crucible of Worlds' first, then a Muldrotha allowance whose land is still
/// unplayed this turn.
#[must_use]
pub fn graveyard_land_permission(
    state: &GameState,
    player: PlayerId,
) -> Option<GraveyardPermission> {
    if !state.effects.may_grant_graveyard_or_top_play() {
        return None;
    }
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
    if let Some(GraveyardPermission::Once {
        effect, vampire, ..
    }) = permission
    {
        if vampire {
            state.effects.remove_where(|fx| fx.id == effect);
        } else {
            state
                .per_turn
                .graveyard_allowances_used
                .push((player, effect));
        }
    }
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
        Zone::Library => {
            state.effects.may_grant_graveyard_or_top_play()
                && state.effects.iter().any(|fx| {
                    fx.controller == player
                        && matches!(
                            fx.modifier,
                            baylee_cards_dsl::Modifier::PlayLandsFromLibraryTop
                        )
                })
        }
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
    state.play_permissions.iter().copied().find(|p| {
        let controller = match p.window {
            crate::state::PlayLifetime::Prepared(source) => state.prepared_controller(source),
            crate::state::PlayLifetime::Turns(_) => Some(p.player),
        };
        controller == Some(player) && p.card == card && p.version == version
    })
}

/// Whether a continuous effect forbids playing `obj` as a land ("Players
/// can't play cards originally printed in the Arabian Nights expansion",
/// CR 101.2 / 305.2b).
pub(crate) fn land_play_is_forbidden(state: &GameState, obj: &crate::object::GameObject) -> bool {
    if obj.zone == crate::zone::Zone::Hand
        && obj
            .riders
            .contains(&crate::object::Rider::RevealedInHandCantPlayUntilTurn)
    {
        return true;
    }
    state.effects.may_change_cast_timing_or_lock()
        && state.effects.iter().any(|fx| match fx.modifier {
            baylee_cards_dsl::Modifier::CantPlayLandsAndLandsCantEnter => true,
            baylee_cards_dsl::Modifier::PlayersCantPlay(filter) => crate::eval::matches(
                filter,
                state,
                obj,
                fx.controller,
                fx.source.unwrap_or(obj.id),
            ),
            _ => false,
        })
}

/// The zone permission plus the particular card restriction. A library
/// permission never grants access to a card below the top. A permission for
/// the card itself ([`play_permission`]) opens it wherever it lies, an
/// opponent's exile included — unless it lets the card be cast and nothing
/// else (Ragavan, Nimble Pilferer), which plays no land (CR 601.1a).
#[must_use]
pub fn land_card_open(state: &GameState, player: PlayerId, card: ObjectId) -> bool {
    let Some(obj) = state.object(card) else {
        return false;
    };
    if land_play_is_forbidden(state, obj) {
        return false;
    }
    let version = Some(obj.version);
    if state
        .play_permissions
        .iter()
        .any(|p| p.player == player && p.card == card && Some(p.version) == version && !p.cast_only)
    {
        return true;
    }
    mayhem_land_permission(state, player, card) || adventure_permission(state, player, card) || {
        if obj.zone == Zone::Exile
            && obj
                .riders
                .contains(&crate::object::Rider::PlayableFromExileFor(player))
        {
            return true;
        }
        obj.zone_owner == Some(player)
            && land_zone_open(state, player, obj.zone)
            && (obj.zone != Zone::Library
                || state.zones.list(ZoneLocation::Library(player)).last() == Some(&card))
    }
}

/// Native costless Mayhem applies only to the exact discarded land incarnation.
pub(crate) fn mayhem_land_permission(state: &GameState, player: PlayerId, card: ObjectId) -> bool {
    state.object(card).is_some_and(|object| {
        object.zone == Zone::Graveyard
            && object.zone_owner == Some(player)
            && object.characteristics().types.contains(TypeSet::LAND)
            && object
                .characteristics()
                .keywords
                .contains(baylee_cards_dsl::KeywordSet::MAYHEM)
            && state
                .per_turn
                .discarded_to_graveyard
                .iter()
                .any(|(reference, discarder)| {
                    *discarder == player
                        && reference.object == card
                        && reference.version == object.version
                })
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
    state
        .entries
        .begin_event(state.journal.last_seq(), Some(player));
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
    if obj.zone == Zone::Graveyard
        && play_permission(state, player, card).is_none()
        && !mayhem_land_permission(state, player, card)
    {
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
