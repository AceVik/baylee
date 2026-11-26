//! Replacement rules that multiply what an effect does (CR 614): Doubling
//! Season, Elspeth Storm-Slayer and the cards written like them.
//!
//! They live in one module because the bug that collected them here was
//! one reader taking its own rule differently from the rest: a filter over
//! the *affected controller* was being asked of the resolving effect's own
//! controller, which is a question that answers yes for everybody, so my
//! Doubling Season doubled an opponent's tokens.
//!
//! There are exactly three readers, and they are **doors** for the same
//! reason [`crate::sba::destroy`] is: [`put_counters`] here, and
//! `resolve::tokens::create_tokens` and
//! `resolve::tokens::create_token_copies` beside the token
//! factory. A counter or a token produced beside one of them is invisible
//! to the rule, and nothing says so until someone plays the pair — which is
//! exactly how "put a +1/+1 counter on each other Ally you control", the
//! shape most of the pool writes, sat outside while the single-target shape
//! next to it was inside, and how a token *copy* was outside while the
//! plain token was in.

pub(crate) mod counter_order;
mod counters;
#[cfg(feature = "test-support")]
pub use counters::{can_move_counter, move_counter, move_counters};
#[cfg(not(feature = "test-support"))]
pub(crate) use counters::{can_move_counter, move_counters};
pub use counters::{
    counter_multiplier, put_counters, put_counters_bounded_by, put_counters_by, record_counters,
    record_counters_by, remove_counters, remove_counters_by,
};

mod counter_effects;
pub(crate) mod counter_limits;
pub(crate) mod departure;

use crate::eval;
#[cfg(any(test, feature = "test-support"))]
use crate::event::GameEvent;
use crate::state::GameState;
use baylee_core::ids::{ObjectId, PlayerId};

/// Outcome of checking graveyard-arrival replacement rules (`CR 614.6`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct GraveyardReplacement {
    /// Final destination zone (unchanged if no replacement applied).
    pub destination: crate::zone::ZoneLocation,
    /// Optional counter placed on the card in exile (e.g. Dauthi Voidwalker's void counter).
    pub exile_counter: Option<baylee_cards_dsl::CounterKind>,
    /// Firestorm Phoenix: returned to hand revealed and unplayable until owner's next turn.
    pub revealed_in_hand: bool,
    /// Blightsteel Colossus / Progenitus: shuffle into destination library instead of
    /// entering the graveyard, revealing first when `reveal_on_shuffle` is set.
    pub shuffle_into_library: bool,
    /// Reveal the card as part of `shuffle_into_library`.
    pub reveal_on_shuffle: bool,
    /// Optional replacement source `(id, version, acquisition)` to link the exiled
    /// card to (`Rider::Linked` / `Rider::ExiledWith` / `Rider::LinkedExile`).
    pub link_source: Option<(ObjectId, u32, u64)>,
    /// Optional zone insertion position (top or bottom of library).
    pub position: Option<crate::zone::ZonePosition>,
    /// Reveal the redirected card as part of the replacement.
    pub reveal: bool,
    /// Optional immediate or delayed follow-up rider executed when exiled
    /// `(controller, source, source_version, rider)`.
    pub exile_rider: Option<(
        PlayerId,
        ObjectId,
        Option<u32>,
        baylee_cards_dsl::GraveyardExileRider,
    )>,
}

impl GraveyardReplacement {
    const fn unchanged(destination: crate::zone::ZoneLocation) -> Self {
        Self {
            destination,
            exile_counter: None,
            revealed_in_hand: false,
            shuffle_into_library: false,
            reveal_on_shuffle: false,
            link_source: None,
            position: None,
            reveal: false,
            exile_rider: None,
        }
    }
}

fn graveyard_redirect_outcome(
    owner: PlayerId,
    destination: baylee_cards_dsl::GraveyardRedirectDest,
    reveal: bool,
) -> GraveyardReplacement {
    use crate::zone::{ZoneLocation, ZonePosition};
    match destination {
        baylee_cards_dsl::GraveyardRedirectDest::Hand => GraveyardReplacement {
            destination: ZoneLocation::Hand(owner),
            reveal,
            ..GraveyardReplacement::unchanged(ZoneLocation::Hand(owner))
        },
        baylee_cards_dsl::GraveyardRedirectDest::TopOfLibrary => GraveyardReplacement {
            destination: ZoneLocation::Library(owner),
            position: Some(ZonePosition::Top),
            reveal,
            ..GraveyardReplacement::unchanged(ZoneLocation::Library(owner))
        },
        baylee_cards_dsl::GraveyardRedirectDest::BottomOfLibrary => GraveyardReplacement {
            destination: ZoneLocation::Library(owner),
            position: Some(ZonePosition::Bottom),
            reveal,
            ..GraveyardReplacement::unchanged(ZoneLocation::Library(owner))
        },
        baylee_cards_dsl::GraveyardRedirectDest::ShuffleIntoLibrary => GraveyardReplacement {
            destination: ZoneLocation::Library(owner),
            shuffle_into_library: true,
            reveal_on_shuffle: reveal,
            reveal,
            ..GraveyardReplacement::unchanged(ZoneLocation::Library(owner))
        },
    }
}

fn graveyard_filter_matches(
    state: &GameState,
    card: &crate::object::GameObject,
    filter: &'static baylee_cards_dsl::Filter,
    controller: PlayerId,
    source: ObjectId,
) -> bool {
    use crate::zone::Zone;
    if !matches!(card.zone, Zone::Battlefield | Zone::Stack) && card.controller != card.owner {
        let mut normalized = card.clone();
        normalized.controller = card.owner;
        eval::matches(filter, state, &normalized, controller, source)
    } else {
        eval::matches(filter, state, card, controller, source)
    }
}

fn graveyard_damaged_by_holds(
    state: &GameState,
    id: ObjectId,
    card_version: u32,
    entry: &crate::state::ReplacementEntry,
    damaged_by: baylee_cards_dsl::DamagedByScope,
) -> bool {
    match damaged_by {
        baylee_cards_dsl::DamagedByScope::Any => true,
        baylee_cards_dsl::DamagedByScope::SourceThisTurn => {
            let Some(source_version) = replacement_source_version(state, entry.source) else {
                return false;
            };
            state.was_damaged_by_source_this_turn(id, card_version, entry.source, source_version)
        }
        baylee_cards_dsl::DamagedByScope::SourceControllerThisTurn => {
            state.was_damaged_by_controller_this_turn(id, card_version, entry.controller)
        }
        baylee_cards_dsl::DamagedByScope::AttachedCreatureThisTurn => {
            let Some(host) = state.object(entry.source).and_then(|o| o.attached_to) else {
                return false;
            };
            let Some(host_version) = replacement_source_version(state, host) else {
                return false;
            };
            state.was_damaged_by_source_this_turn(id, card_version, host, host_version)
        }
    }
}

/// Active or just-departed battlefield version of a registered replacement
/// source (CR 614.6: inside a simultaneous wipe, a source that moved earlier in
/// the same batch still retains its battlefield version in `ltb_versions`).
fn replacement_source_version(state: &GameState, source: ObjectId) -> Option<u32> {
    let obj = state.object(source)?;
    if obj.zone == crate::zone::Zone::Battlefield {
        Some(obj.version)
    } else {
        state
            .ltb_versions
            .iter()
            .find(|(id, _)| *id == source)
            .map(|(_, version)| *version)
    }
}

/// Replaces an object's arrival in a graveyard. Registered rules survive the
/// current simultaneous event even if their source moves first.
#[allow(clippy::too_many_lines)] // Flat replacement priority chain (self -> effect -> board rules).
pub(crate) fn graveyard_destination(
    state: &GameState,
    id: ObjectId,
    to: crate::zone::ZoneLocation,
) -> GraveyardReplacement {
    use crate::{
        object::{ObjectKind, Rider, Status},
        zone::{Zone, ZoneLocation},
    };
    let ZoneLocation::Graveyard(player) = to else {
        return GraveyardReplacement::unchanged(to);
    };
    let Some(card) = state.object(id) else {
        return GraveyardReplacement::unchanged(to);
    };
    // "If this creature would die, return Firestorm Phoenix to its owner's
    // hand instead. Until that player's next turn, that player plays with
    // that card revealed in their hand and can't play it."
    let return_hand_rule = |entry: &&crate::state::ReplacementEntry| {
        entry.source == id
            && entry.rule
                == baylee_cards_dsl::ReplacementRule::ReturnSelfToHandRevealedCantPlayUntilOwnersNextTurn
    };
    if card.zone == Zone::Battlefield
        && card.is_card()
        && card
            .characteristics()
            .types
            .contains(baylee_core::types::TypeSet::CREATURE)
        && state
            .replacement_rules
            .iter()
            .any(|entry| return_hand_rule(&entry))
    {
        #[cfg(any(test, feature = "test-support"))]
        if let Some(entry) = state.replacement_rules.iter().find(return_hand_rule) {
            crate::ability_log::replaced(entry);
        }
        return GraveyardReplacement {
            revealed_in_hand: true,
            ..GraveyardReplacement::unchanged(ZoneLocation::Hand(card.owner))
        };
    }
    // "If that creature would die this turn, exile it instead" (Mawloc),
    // for that object only. Before the token test below: a token is exiled
    // instead as well, and so does not die.
    if card.zone == Zone::Battlefield
        && (state.per_turn.exile_if_dies.contains(&(id, card.version))
            || card.counters.get(baylee_cards_dsl::CounterKind::Finality) != 0)
    {
        return GraveyardReplacement::unchanged(ZoneLocation::Exile(card.owner));
    }
    if card.is_card() {
        // Its own "exile it instead": on the stack as a rider, on the
        // battlefield as the rule its face registered.
        let own_rule = |entry: &&crate::state::ReplacementEntry| {
            entry.source == id
                && entry.rule == baylee_cards_dsl::ReplacementRule::ExileSelfInsteadOfGraveyard
        };
        if (card.zone == Zone::Stack && card.riders.contains(&Rider::ExileInsteadOfGraveyard))
            || state.replacement_rules.iter().any(|entry| own_rule(&entry))
        {
            #[cfg(any(test, feature = "test-support"))]
            if let Some(entry) = state.replacement_rules.iter().find(own_rule) {
                crate::ability_log::replaced(entry);
            }
            return GraveyardReplacement::unchanged(ZoneLocation::Exile(card.owner));
        }
        // "If a card would be put into your graveyard from anywhere this turn,
        // exile it instead" (Forgotten Cellar): a replacement a resolving ability
        // made, so an effect with a duration rather than a rule a permanent
        // registered (CR 614.1a, 611.2a), and the graveyard's owner's.
        if state.effects.iter().any(|fx| {
            fx.controller == player
                && fx.modifier == baylee_cards_dsl::Modifier::ExileInsteadOfYourGraveyard
        }) {
            return GraveyardReplacement::unchanged(ZoneLocation::Exile(card.owner));
        }
    }
    if !card.riders.contains(&Rider::SpellCopy) && card.kind != ObjectKind::AbilityOnStack {
        let effective_controller = if matches!(card.zone, Zone::Battlefield | Zone::Stack) {
            card.controller
        } else {
            card.owner
        };
        for fx in state.effects.iter() {
            let baylee_cards_dsl::Modifier::ExileMatchingInsteadOfGraveyard {
                filter,
                origin,
                controller_lki,
                rider,
            } = fx.modifier
            else {
                continue;
            };
            if let Some(req_origin) = origin
                && !card.zone.matches_ref(req_origin)
            {
                continue;
            }
            if let Some(req_controller) = controller_lki
                && !eval::players_for_source(req_controller, state, fx.controller, fx.source)
                    .is_some_and(|seats| seats.contains(&effective_controller))
            {
                continue;
            }
            let fx_source = fx.source.unwrap_or(id);
            if !graveyard_filter_matches(state, card, filter, fx.controller, fx_source) {
                continue;
            }
            let exile_rider = rider.map(|r| {
                let ver = fx.source.and_then(|s| replacement_source_version(state, s));
                (fx.controller, fx_source, ver, r)
            });
            return GraveyardReplacement {
                exile_rider,
                ..GraveyardReplacement::unchanged(ZoneLocation::Exile(card.owner))
            };
        }
    }
    // "If [this] would be put into a graveyard from anywhere, [reveal it and]
    // shuffle it into its owner's library instead" (Blightsteel Colossus,
    // Darksteel Colossus, Progenitus; CR 113.6, CR 614.6). Read off the
    // object's active abilities in its current zone so it works from the
    // battlefield, hand, library, stack and face-up exile while respecting
    // ability loss and face-down status.
    if !card.riders.contains(&Rider::SpellCopy)
        && card.kind != ObjectKind::AbilityOnStack
        && let Some(reveal) = card
            .abilities(state.bases.as_ref())
            .iter()
            .find_map(|a| match a {
                baylee_cards_dsl::AbilityDef::Replacement(
                    baylee_cards_dsl::ReplacementRule::ShuffleIntoLibraryInsteadOfGraveyard {
                        filter,
                        reveal,
                    },
                ) if **filter == baylee_cards_dsl::Filter::This => Some(*reveal),
                _ => None,
            })
    {
        #[cfg(any(test, feature = "test-support"))]
        crate::ability_log::replaced_on(
            state,
            id,
            baylee_cards_dsl::ReplacementRule::ShuffleIntoLibraryInsteadOfGraveyard {
                filter: &baylee_cards_dsl::Filter::This,
                reveal,
            },
        );
        return GraveyardReplacement {
            shuffle_into_library: true,
            reveal_on_shuffle: reveal,
            ..GraveyardReplacement::unchanged(ZoneLocation::Library(card.owner))
        };
    }
    if !card.riders.contains(&Rider::SpellCopy)
        && card.kind != ObjectKind::AbilityOnStack
        && let Some((destination, reveal)) =
            card.abilities(state.bases.as_ref())
                .iter()
                .find_map(|a| match a {
                    baylee_cards_dsl::AbilityDef::Replacement(
                        baylee_cards_dsl::ReplacementRule::RedirectGraveyard {
                            filter,
                            origin,
                            destination,
                            reveal,
                            ..
                        },
                    ) if **filter == baylee_cards_dsl::Filter::This
                        && origin.is_none_or(|req| card.zone.matches_ref(req)) =>
                    {
                        Some((*destination, *reveal))
                    }
                    _ => None,
                })
    {
        return graveyard_redirect_outcome(card.owner, destination, reveal);
    }
    for entry in &state.replacement_rules {
        match entry.rule {
            baylee_cards_dsl::ReplacementRule::ExileOpponentsGraveyard { counter }
                if card.is_card()
                    && state.is_opponent(player, entry.controller)
                    && !state
                        .object(entry.source)
                        .is_some_and(|o| o.status.contains(Status::PHASED_OUT)) =>
            {
                #[cfg(any(test, feature = "test-support"))]
                crate::ability_log::replaced(entry);
                return GraveyardReplacement {
                    exile_counter: counter,
                    ..GraveyardReplacement::unchanged(ZoneLocation::Exile(card.owner))
                };
            }
            baylee_cards_dsl::ReplacementRule::ExileInsteadOfGraveyard {
                filter,
                origin,
                owner,
                controller_lki,
                damaged_by,
                link_to_source,
                counter,
            } => {
                // Spell copies and stack abilities are neither cards nor tokens.
                if card.riders.contains(&Rider::SpellCopy)
                    || card.kind == ObjectKind::AbilityOnStack
                {
                    continue;
                }
                // Phased-out sources and live sources whose abilities/text are
                // currently lost do not apply (CR 702.26b, CR 613.1f).
                if state.object(entry.source).is_some_and(|o| {
                    o.zone == Zone::Battlefield
                        && (o.status.contains(Status::PHASED_OUT)
                            || o.characteristics().abilities_lost.is_some()
                            || o.characteristics().rules_text_lost)
                }) {
                    continue;
                }
                if let Some(req_origin) = origin
                    && !card.zone.matches_ref(req_origin)
                {
                    continue;
                }
                if let Some(req_owner) = owner
                    && !eval::players_for_source(
                        req_owner,
                        state,
                        entry.controller,
                        Some(entry.source),
                    )
                    .is_some_and(|seats| seats.contains(&player))
                {
                    continue;
                }
                // CR 109.4: only objects on the battlefield or stack have a
                // controller; elsewhere, use the object's owner.
                let effective_controller = if matches!(card.zone, Zone::Battlefield | Zone::Stack) {
                    card.controller
                } else {
                    card.owner
                };
                if let Some(req_controller) = controller_lki
                    && !eval::players_for_source(
                        req_controller,
                        state,
                        entry.controller,
                        Some(entry.source),
                    )
                    .is_some_and(|seats| seats.contains(&effective_controller))
                {
                    continue;
                }
                if !graveyard_damaged_by_holds(state, id, card.version, entry, damaged_by) {
                    continue;
                }
                if !graveyard_filter_matches(state, card, filter, entry.controller, entry.source) {
                    continue;
                }
                #[cfg(any(test, feature = "test-support"))]
                crate::ability_log::replaced(entry);
                let link_source = link_to_source
                    .then(|| {
                        let version = replacement_source_version(state, entry.source)?;
                        let acquisition = state
                            .source_object(baylee_core::ids::DamageSourceRef {
                                object: entry.source,
                                version,
                            })
                            .map_or_else(
                                || {
                                    state
                                        .object(entry.source)
                                        .map_or(0, crate::object::GameObject::rules_acquisition)
                                },
                                crate::object::GameObject::rules_acquisition,
                            );
                        Some((entry.source, version, acquisition))
                    })
                    .flatten();
                return GraveyardReplacement {
                    exile_counter: counter,
                    link_source,
                    ..GraveyardReplacement::unchanged(ZoneLocation::Exile(card.owner))
                };
            }
            baylee_cards_dsl::ReplacementRule::ShuffleIntoLibraryInsteadOfGraveyard {
                filter,
                reveal,
            } => {
                if card.riders.contains(&Rider::SpellCopy)
                    || card.kind == ObjectKind::AbilityOnStack
                {
                    continue;
                }
                if state.object(entry.source).is_some_and(|o| {
                    o.zone == Zone::Battlefield
                        && (o.status.contains(Status::PHASED_OUT)
                            || o.characteristics().abilities_lost.is_some()
                            || o.characteristics().rules_text_lost)
                }) {
                    continue;
                }
                let filter_ok = if !matches!(card.zone, Zone::Battlefield | Zone::Stack)
                    && card.controller != card.owner
                {
                    let mut normalized = card.clone();
                    normalized.controller = card.owner;
                    eval::matches(filter, state, &normalized, entry.controller, entry.source)
                } else {
                    eval::matches(filter, state, card, entry.controller, entry.source)
                };
                if !filter_ok {
                    continue;
                }
                #[cfg(any(test, feature = "test-support"))]
                crate::ability_log::replaced(entry);
                return GraveyardReplacement {
                    shuffle_into_library: true,
                    reveal_on_shuffle: reveal,
                    ..GraveyardReplacement::unchanged(ZoneLocation::Library(card.owner))
                };
            }
            baylee_cards_dsl::ReplacementRule::RedirectGraveyard {
                filter,
                origin,
                owner,
                controller_lki,
                destination,
                reveal,
            } => {
                if card.riders.contains(&Rider::SpellCopy)
                    || card.kind == ObjectKind::AbilityOnStack
                {
                    continue;
                }
                if state.object(entry.source).is_some_and(|o| {
                    o.zone == Zone::Battlefield
                        && (o.status.contains(Status::PHASED_OUT)
                            || o.characteristics().abilities_lost.is_some()
                            || o.characteristics().rules_text_lost)
                }) {
                    continue;
                }
                if let Some(req_origin) = origin
                    && !card.zone.matches_ref(req_origin)
                {
                    continue;
                }
                if let Some(req_owner) = owner
                    && !eval::players_for_source(
                        req_owner,
                        state,
                        entry.controller,
                        Some(entry.source),
                    )
                    .is_some_and(|seats| seats.contains(&player))
                {
                    continue;
                }
                let effective_controller = if matches!(card.zone, Zone::Battlefield | Zone::Stack) {
                    card.controller
                } else {
                    card.owner
                };
                if let Some(req_controller) = controller_lki
                    && !eval::players_for_source(
                        req_controller,
                        state,
                        entry.controller,
                        Some(entry.source),
                    )
                    .is_some_and(|seats| seats.contains(&effective_controller))
                {
                    continue;
                }
                if !graveyard_filter_matches(state, card, filter, entry.controller, entry.source) {
                    continue;
                }
                #[cfg(any(test, feature = "test-support"))]
                crate::ability_log::replaced(entry);
                return graveyard_redirect_outcome(card.owner, destination, reveal);
            }
            baylee_cards_dsl::ReplacementRule::ExileInsteadOfGraveyardWithRider {
                filter,
                origin,
                owner,
                controller_lki,
                damaged_by,
                link_to_source,
                rider,
            } => {
                if card.riders.contains(&Rider::SpellCopy)
                    || card.kind == ObjectKind::AbilityOnStack
                {
                    continue;
                }
                if state.object(entry.source).is_some_and(|o| {
                    o.zone == Zone::Battlefield
                        && (o.status.contains(Status::PHASED_OUT)
                            || o.characteristics().abilities_lost.is_some()
                            || o.characteristics().rules_text_lost)
                }) {
                    continue;
                }
                if let Some(req_origin) = origin
                    && !card.zone.matches_ref(req_origin)
                {
                    continue;
                }
                if let Some(req_owner) = owner
                    && !eval::players_for_source(
                        req_owner,
                        state,
                        entry.controller,
                        Some(entry.source),
                    )
                    .is_some_and(|seats| seats.contains(&player))
                {
                    continue;
                }
                let effective_controller = if matches!(card.zone, Zone::Battlefield | Zone::Stack) {
                    card.controller
                } else {
                    card.owner
                };
                if let Some(req_controller) = controller_lki
                    && !eval::players_for_source(
                        req_controller,
                        state,
                        entry.controller,
                        Some(entry.source),
                    )
                    .is_some_and(|seats| seats.contains(&effective_controller))
                {
                    continue;
                }
                if !graveyard_damaged_by_holds(state, id, card.version, entry, damaged_by) {
                    continue;
                }
                if !graveyard_filter_matches(state, card, filter, entry.controller, entry.source) {
                    continue;
                }
                #[cfg(any(test, feature = "test-support"))]
                crate::ability_log::replaced(entry);
                let src_ver = replacement_source_version(state, entry.source);
                let link_source = link_to_source
                    .then(|| {
                        let version = src_ver?;
                        let acquisition = state
                            .source_object(baylee_core::ids::DamageSourceRef {
                                object: entry.source,
                                version,
                            })
                            .map_or_else(
                                || {
                                    state
                                        .object(entry.source)
                                        .map_or(0, crate::object::GameObject::rules_acquisition)
                                },
                                crate::object::GameObject::rules_acquisition,
                            );
                        Some((entry.source, version, acquisition))
                    })
                    .flatten();
                return GraveyardReplacement {
                    link_source,
                    exile_rider: Some((entry.controller, entry.source, src_ver, rider)),
                    ..GraveyardReplacement::unchanged(ZoneLocation::Exile(card.owner))
                };
            }
            _ => {}
        }
    }
    GraveyardReplacement::unchanged(to)
}

/// Drops the rules of every source that is no longer on the battlefield, at
/// the boundary between two events of one resolution (#291).
///
/// A replacement or trigger-multiplier rule is a static ability of its
/// source, and a source that has left has no abilities left to apply to an
/// event that happens after it left: "destroy it, then create two tokens"
/// with the Doubling Season as the "it" creates two tokens, not four. Inside
/// one event the rule still applies to all of it, by last-known information,
/// even when the engine moves the objects of a simultaneous event one at a
/// time and the source goes first — a Dauthi Voidwalker destroyed in a wipe
/// still exiles the cards that die beside it. So `resolve::run` calls this
/// before each instruction and once after the last (the resolving spell's
/// own departure is an event of its own), never inside one.
///
/// The engine's sync (`Engine::sync_static_effects`) did this already, but
/// only between engine steps, so the rule outlived its source for the rest
/// of the resolution. A phased-out source is still on the battlefield, and
/// keeps its rules until that sync drops them; `graveyard_destination`
/// excludes them meanwhile.
pub(crate) fn expire_departed_rules(state: &mut GameState) {
    let rules = &state.replacement_rules;
    let on_battlefield = |source: ObjectId| {
        state
            .object(source)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
    };
    // One lookup per rule, and no write unless something left: this runs
    // before every instruction of every resolution.
    if rules.iter().all(|entry| on_battlefield(entry.source)) {
        return;
    }
    let departed: Vec<ObjectId> = rules
        .iter()
        .map(|entry| entry.source)
        .filter(|&source| !on_battlefield(source))
        .collect();
    state
        .replacement_rules
        .retain(|entry| !departed.contains(&entry.source));
}

/// How many times over an effect creating tokens under `recipient`'s
/// control actually creates them (CR 614.1).
///
/// `controller_filter` is a filter over the **affected controller**, not
/// over the effect doing the creating: "if one or more tokens would be
/// created under *your* control" is a statement about who ends up with them
/// and says nothing about whose spell put them there. So it is read against
/// the replacement's own source with the recipient as "you", which makes
/// `ControlledByYou` mean "the enchantment that recipient controls".
#[must_use]
pub fn token_multiplier(
    state: &GameState,
    recipient: PlayerId,
    types: baylee_core::types::TypeSet,
) -> Option<u32> {
    use baylee_cards_dsl::ReplacementRule;
    let mut count = 1u32;
    for entry in &state.replacement_rules {
        let (controller_filter, factor) = match entry.rule {
            ReplacementRule::DoubleTokenCreation { controller_filter } => (controller_filter, 2),
            ReplacementRule::MultiplyTypedTokens {
                controller_filter,
                types: required,
                factor,
            } if types.intersects(required) => (controller_filter, factor),
            _ => continue,
        };
        if let Some(source_obj) = state.object(entry.source)
            && eval::matches(
                controller_filter,
                state,
                source_obj,
                recipient,
                entry.source,
            )
        {
            #[cfg(any(test, feature = "test-support"))]
            crate::ability_log::replaced(entry);
            count = count.checked_mul(factor)?;
        }
    }
    Some(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::ObjectKind;
    use crate::state::{CardLookup, ReplacementEntry};
    use crate::zone::ZoneLocation;
    use baylee_cards_dsl::{CounterKind, Filter, ReplacementRule};
    use baylee_core::ids::CardIndex;
    use baylee_core::preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
        SeatController, SeatSpec,
    };

    struct RegistryLookup;
    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn me() -> PlayerId {
        PlayerId::new(0)
    }
    fn them() -> PlayerId {
        PlayerId::new(1)
    }

    /// Two seats, empty boards, nothing on the stack. Every rule here is
    /// pushed by hand: `replacement_rules` is rebuilt only by the engine's
    /// static-ability pass (`engine::progress`), which no test in this
    /// module runs, so an entry put here stays put.
    fn state() -> GameState {
        let forest = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
            .expect("registry contains Forest")
            .index;
        let deck: Vec<DeckEntry> = (0..60)
            .map(|_| DeckEntry {
                rule_print: None,
                choices: baylee_core::card_metadata::DeckChoices::default(),
                card: forest,
                print: baylee_core::ids::PrintRef::new(0),
            })
            .collect();
        let seat = || SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: SeatCapabilities::default(),
            deck: deck.clone(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            rules: baylee_core::card_metadata::RulesMetadata::default(),
            format: FormatId::Freeform,
            seed: 9,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: baylee_core::preset::Finish::Normal,
            }],
            seats: vec![seat(), seat()],
        };
        GameState::from_preset(&preset, &RegistryLookup).expect("game starts")
    }

    /// A card-less permanent on `owner`'s battlefield. These two rules ask
    /// one question of an object — who controls it — so a bare one is the
    /// whole population a filter over `ControlledByYou` can see.
    fn permanent(state: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        let name = state.names.intern(name);
        state.create_bare(
            owner,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        )
    }

    fn doubles_tokens(state: &mut GameState, controller: PlayerId) {
        let source = permanent(state, controller, "Doubling Season");
        state.replacement_rules.push(ReplacementEntry {
            source,
            controller,
            rule: ReplacementRule::DoubleTokenCreation {
                controller_filter: &Filter::ControlledByYou,
            },
        });
    }

    fn doubles_counters(state: &mut GameState, controller: PlayerId) {
        let source = permanent(state, controller, "Doubling Season");
        state.replacement_rules.push(ReplacementEntry {
            source,
            controller,
            rule: ReplacementRule::DoubleCounterPlacement {
                object_filter: &Filter::ControlledByYou,
            },
        });
    }

    /// The bug this module was collected around, from the side it was seen
    /// on: "if one or more tokens would be created under **your** control"
    /// is a statement about who ends up with them, so it is read against the
    /// recipient. Asked instead of the resolving effect's own controller it
    /// answers yes for everybody, and my Doubling Season doubles an
    /// opponent's tokens.
    #[test]
    fn a_token_doubler_reads_its_filter_against_the_recipient() {
        let mut state = state();
        doubles_tokens(&mut state, me());
        assert_eq!(
            token_multiplier(&state, me(), baylee_core::types::TypeSet::CREATURE),
            Some(2),
            "my tokens are doubled"
        );
        assert_eq!(
            token_multiplier(&state, them(), baylee_core::types::TypeSet::CREATURE),
            Some(1),
            "and an opponent's are not — the filter is over the affected \
             controller, not over whose effect is creating them"
        );
    }

    /// The mirror, and read the other way round on purpose: this filter is
    /// over the permanent *receiving* the counters. Neither
    /// [`counter_multiplier`] nor [`put_counters`] takes a controller
    /// argument at all, which is the same sentence said in the signature —
    /// an opponent's spell putting a counter on my creature is doubled by my
    /// Doubling Season, because the door cannot know whose spell it was.
    #[test]
    fn a_counter_doubler_reads_its_filter_against_the_permanent() {
        let mut state = state();
        doubles_counters(&mut state, me());
        let mine = permanent(&mut state, me(), "Mine");
        let theirs = permanent(&mut state, them(), "Theirs");

        assert_eq!(counter_multiplier(&state, mine), 2);
        assert_eq!(counter_multiplier(&state, theirs), 1);

        put_counters(&mut state, mine, CounterKind::P1P1, 1);
        put_counters(&mut state, theirs, CounterKind::P1P1, 1);
        assert_eq!(
            state
                .object(mine)
                .expect("still there")
                .counters
                .get(CounterKind::P1P1),
            2
        );
        assert_eq!(
            state
                .object(theirs)
                .expect("still there")
                .counters
                .get(CounterKind::P1P1),
            1
        );
    }

    /// An object that is gone is not a permanent under anybody's
    /// replacement, and the answer is one rather than a panic: the door is
    /// called from resolution, where a target can have left since the
    /// effect was put on the stack.
    #[test]
    fn a_counter_multiplier_for_an_object_that_is_gone_is_one() {
        let mut state = state();
        doubles_counters(&mut state, me());
        let mine = permanent(&mut state, me(), "Mine");
        state.arena.remove(mine);
        assert_eq!(counter_multiplier(&state, mine), 1);
    }

    /// Two of them multiply rather than either one winning: CR 614.1 applies
    /// each replacement once, so the second one sees four where the first
    /// made two. Both loops are written this way and neither had a test.
    #[test]
    fn two_doublers_multiply() {
        let mut state = state();
        doubles_tokens(&mut state, me());
        doubles_tokens(&mut state, me());
        doubles_counters(&mut state, me());
        doubles_counters(&mut state, me());
        let mine = permanent(&mut state, me(), "Mine");

        assert_eq!(
            token_multiplier(&state, me(), baylee_core::types::TypeSet::CREATURE),
            Some(4)
        );
        assert_eq!(counter_multiplier(&state, mine), 4);

        put_counters(&mut state, mine, CounterKind::P1P1, 3);
        assert_eq!(
            state
                .object(mine)
                .expect("still there")
                .counters
                .get(CounterKind::P1P1),
            12,
            "the multiplier applies to the whole placement, not to one counter"
        );
    }

    /// The reason [`record_counters`] exists beside [`put_counters`].
    /// CR 614.16 doubles what a resolving spell or ability places and what
    /// another replacement places — and a **turn-based action** is neither,
    /// so the lore counter a Saga takes as its controller's main phase
    /// begins (CR 715.3c) lands as one counter under any number of Doubling
    /// Seasons.
    #[test]
    fn a_turn_based_counter_is_not_doubled() {
        let mut state = state();
        doubles_counters(&mut state, me());
        let saga = permanent(&mut state, me(), "Saga");

        record_counters(&mut state, saga, CounterKind::Lore, 1);
        assert_eq!(
            state
                .object(saga)
                .expect("still there")
                .counters
                .get(CounterKind::Lore),
            1,
            "the turn-based action puts one"
        );

        put_counters(&mut state, saga, CounterKind::Lore, 1);
        assert_eq!(
            state
                .object(saga)
                .expect("still there")
                .counters
                .get(CounterKind::Lore),
            3,
            "and the same counter placed by an effect is doubled, which is \
             what makes the two doors different rather than redundant"
        );
    }

    /// Magic prints nothing that multiplies a removal, which is why
    /// [`remove_counters`] takes no multiplier. It saturates rather than
    /// refusing, and answers how many actually came off — the number a cost
    /// or an effect paid with.
    #[test]
    fn nothing_multiplies_a_removal() {
        let mut state = state();
        doubles_counters(&mut state, me());
        let mine = permanent(&mut state, me(), "Mine");
        record_counters(&mut state, mine, CounterKind::P1P1, 3);

        assert_eq!(
            remove_counters(&mut state, mine, CounterKind::P1P1, 1),
            1,
            "one comes off under two doublings of its placement"
        );
        assert_eq!(
            remove_counters(&mut state, mine, CounterKind::P1P1, 5),
            2,
            "and asking for more than is there takes what is there"
        );
        assert_eq!(
            state
                .object(mine)
                .expect("still there")
                .counters
                .get(CounterKind::P1P1),
            0
        );
        assert_eq!(
            remove_counters(&mut state, mine, CounterKind::P1P1, 1),
            0,
            "a removal from nothing removes nothing"
        );
    }

    /// The journal entry and the projection invalidation are part of the
    /// door and not of the caller: a counter is a characteristic input
    /// (CR 613.4c), so one placed without invalidating leaves the creature
    /// drawn — and read by the state-based action that kills it — at its old
    /// size. All three doors owe both.
    #[test]
    fn every_counter_door_records_the_change_and_invalidates_the_projection() {
        let mut state = state();
        let mine = permanent(&mut state, me(), "Mine");

        // The three doors run on one object in turn and this state carries
        // no doubling rule, so `n` is simply what the store reads after
        // each: 2 placed, 3 after one more, 0 once three come off.
        for (label, n) in [("put", 2u16), ("record", 3), ("remove", 0)] {
            let before = state.journal.len();
            // `from_preset` leaves this at the invalid sentinel, so a fresh
            // state would pass the assertion below having done nothing.
            state.characteristics_generation = 0;
            match label {
                "put" => put_counters(&mut state, mine, CounterKind::P1P1, 2),
                "record" => record_counters(&mut state, mine, CounterKind::P1P1, 1),
                _ => {
                    remove_counters(&mut state, mine, CounterKind::P1P1, 3);
                }
            }
            assert_eq!(
                state.characteristics_generation,
                u64::MAX,
                "{label} left a cached projection standing"
            );
            assert_eq!(state.journal.len(), before + 1, "{label} recorded nothing");
            let last = state.journal.entries().last().expect("an entry");
            assert!(
                matches!(
                    last.event,
                    GameEvent::CounterChanged {
                        object,
                        kind: CounterKind::P1P1,
                        new,
                        ..
                    } if object == mine && new == n
                ),
                "{label} recorded {:?}",
                last.event
            );
        }
    }
}
