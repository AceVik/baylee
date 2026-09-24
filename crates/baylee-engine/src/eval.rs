//! Evaluation of DSL data: filters, amounts, target options.
//!
//! All evaluation is pure read access to [`GameState`]; `you` is the
//! ability/spell controller, `this` its source object.

use crate::object::{GameObject, Status};
use crate::state::GameState;
use crate::zone::ZoneLocation;
use baylee_cards_dsl::{Amount, Condition, Filter, PlayerRel, TargetSpec, ZoneSel};
use baylee_core::ids::{ObjectId, PlayerId};

/// Evaluates a [`Filter`] against an object.
#[must_use]
pub fn matches(
    filter: &Filter,
    state: &GameState,
    obj: &GameObject,
    you: PlayerId,
    this: ObjectId,
) -> bool {
    matches_projected(filter, state, obj, obj.characteristics(), you, this)
}

/// Evaluates a [`Filter`] against an object whose characteristics are
/// supplied separately.
///
/// The layer system needs this: CR 613.1 evaluates each layer against the
/// characteristics as modified by every *earlier* layer — a value that
/// exists only mid-projection and is not yet in the object's cache.
#[must_use]
pub fn matches_projected(
    filter: &Filter,
    state: &GameState,
    obj: &GameObject,
    chars: &crate::object::Characteristics,
    you: PlayerId,
    this: ObjectId,
) -> bool {
    let matches = |f: &Filter| matches_projected(f, state, obj, chars, you, this);
    match filter {
        Filter::Any => true,
        Filter::This => obj.id == this,
        Filter::Another => obj.id != this,
        Filter::And(parts) => parts.iter().all(&matches),
        Filter::Or(parts) => parts.iter().any(&matches),
        Filter::Not(f) => !matches(f),
        Filter::HasType(t) => chars.types.intersects(*t),
        Filter::LacksType(t) => !chars.types.intersects(*t),
        Filter::HasSupertype(t) => chars.supertypes.contains(*t),
        Filter::HasSubtype(s) => chars.subtypes.contains(*s),
        Filter::HasColor(c) => chars.colors.intersects(*c),
        Filter::IsColorless => chars.colors.is_colorless(),
        Filter::Monocolored => chars.colors.len() == 1,
        // CR 201.2: the name an object has *now*, read off its projected
        // characteristics, so a clone answers to what it copied.
        Filter::Named(name) => state.names.get(chars.name) == *name,
        Filter::IsToken => obj.card.is_none(),
        Filter::ControlledByYou => obj.controller == you,
        Filter::ControlledByOpponent => state.is_opponent(obj.controller, you),
        Filter::OwnedByYou => obj.owner == you,
        Filter::Tapped => obj.status.contains(Status::TAPPED),
        Filter::Untapped => !obj.status.contains(Status::TAPPED),
        Filter::Attacking => state
            .combat
            .attackers
            .iter()
            .any(|info| info.creature == obj.id),
        // The turn's record of arrivals. It is written where a `ZoneChanged`
        // into the battlefield is journaled, and it holds the id
        // `move_object` returns, so the handle a permanent has now is the one
        // written on the way in. It used to be a scan of the journal from
        // the turn's start, which the snapshot hash does not read (#241).
        //
        // No `Cause` is filtered out, and `Cause::Setup` is the one worth
        // saying so about. A seeded battlefield is recorded like any other
        // arrival while the game is built, and the first turn start, after
        // the mulligans, clears the record. An exclusion here was written
        // first and removed when injecting its removal left the test green;
        // `arrival_tests::a_permanent_entered_this_turn_only_on_the_turn_it_was_played`
        // asserts both halves of what makes it unnecessary.
        Filter::EnteredThisTurn => state.per_turn.entered_battlefield.contains(&obj.id),
        Filter::MatchesChosenTypeOfSource => state
            .object(this)
            .and_then(|src| src.chosen_subtype)
            .is_some_and(|s| chars.subtypes.contains(s)),
        // The live attachment first, then what `this` was wearing the moment
        // `obj` left the battlefield (CR 603.10a). The fallback is what lets
        // "whenever equipped creature dies" fire at all: the state-based
        // actions unattach the Equipment in the same `sba::run` fixpoint that
        // killed its host, and triggers are collected after it. It cannot
        // reach anything else — `ltb_attachments` only ever names a host that
        // is off the battlefield, and the move that brings one back clears
        // its entry first.
        Filter::AttachedToBySource => {
            state
                .object(this)
                .and_then(|src| src.attached_to)
                .is_some_and(|attached| attached == obj.id)
                || state
                    .ltb_attachments
                    .iter()
                    .any(|(host, worn)| *host == obj.id && worn.contains(&this))
        }
        Filter::SharesSubtypeWithCommander => {
            // Eight `AND`s per commander, not one probe per subtype id.
            // The marker list rather than the command zone, for the reason
            // `resolve::mana` gives: Path of Ancestry has to keep working
            // once the commander it names is on the battlefield.
            let obj_subs = chars.subtypes;
            state
                .commanders
                .get(you.get() as usize)
                .into_iter()
                .flatten()
                .filter_map(|c| state.object(c.object))
                .any(|commander| obj_subs.intersects(commander.characteristics().subtypes))
        }
        Filter::HasKeyword(k) => chars.keywords.contains(*k),
        Filter::CmcAtMost(n) => chars.mana_cost.cmc() <= *n,
        // The bound is the announced X on the ability's own source, which is
        // where `cast_wizard` writes it and what `res.x` is read from one
        // layer up. A source that is gone, or that announced nothing, bounds
        // at 0 rather than at everything: an unreadable bound that found the
        // whole library would be a tutor with no price.
        Filter::CmcAtMostX => {
            let x = state.object(this).map_or(0, |o| o.x_value);
            chars.mana_cost.cmc() <= x
        }
        Filter::CmcAtLeast(n) => chars.mana_cost.cmc() >= *n,
        Filter::ToughnessAtMost(n) => chars.toughness.is_some_and(|t| t <= *n),
        Filter::ToughnessAtLeast(n) => chars.toughness.is_some_and(|t| t >= *n),
        // `is_some_and`, so an object with no power at all — a land, an
        // instant on the stack — is not "a creature with power 4 or
        // greater" by default. The three neighbours above answer the same
        // way and for the same reason.
        Filter::PowerAtLeast(n) => chars.power.is_some_and(|p| p >= *n),
        Filter::PowerAtMost(n) => chars.power.is_some_and(|p| p <= *n),
        Filter::InZone(z) => {
            use baylee_cards_dsl::ZoneRef;
            match z {
                ZoneRef::Battlefield => obj.zone == crate::zone::Zone::Battlefield,
                ZoneRef::Stack => obj.zone == crate::zone::Zone::Stack,
                ZoneRef::Library => obj.zone == crate::zone::Zone::Library,
                ZoneRef::Hand => obj.zone == crate::zone::Zone::Hand,
                ZoneRef::Graveyard => obj.zone == crate::zone::Zone::Graveyard,
                ZoneRef::Exile => obj.zone == crate::zone::Zone::Exile,
                ZoneRef::Command => obj.zone == crate::zone::Zone::Command,
                // Cards outside the game are in no zone at all, so "not on the
                // battlefield" must not sweep the sideboard in with them.
                ZoneRef::NotBattlefield => {
                    obj.zone != crate::zone::Zone::Battlefield
                        && obj.zone != crate::zone::Zone::OutsideGame
                }
            }
        }
    }
}

/// Resolves a relative player reference to concrete players — or `None` for
/// the two relations the game state alone cannot answer.
///
/// `Chosen` is the player a spell or ability targeted and `ControllerOfTarget`
/// is read off its first object target, so both need the *resolution's* own
/// context and only [`crate::resolve::players_of`] has it. The `None` is the
/// whole point of the signature: this used to answer `vec![]` there, which
/// reads exactly like "no seats matched" and is what shipped Abraded Bluffs
/// as a land that deals no damage, Twining Twins' ward as a keyword that never
/// asks for its tax, Path to Exile without its ramp and Bojuka Bog as a swamp.
/// A caller that has a `Resolution` must not be able to swallow that silently.
#[must_use]
pub fn players(rel: PlayerRel, state: &GameState, you: PlayerId) -> Option<Vec<PlayerId>> {
    Some(match rel {
        PlayerRel::You => vec![you],
        PlayerRel::Opponent | PlayerRel::EachOpponent => state
            .players
            .iter()
            .filter(|p| state.is_opponent(p.id, you) && !p.has_lost())
            .map(|p| p.id)
            .collect(),
        PlayerRel::EachPlayer => state
            .players
            .iter()
            .filter(|p| !p.has_lost())
            .map(|p| p.id)
            .collect(),
        PlayerRel::ControllerOfTarget | PlayerRel::Chosen => return None,
    })
}

/// The graveyard cards a `CardInGraveyard` spec may point at.
///
/// Legality is enumerated *before* any resolution exists, so the two context
/// relations have no answer here and an empty list is the honest one — this
/// is the **only** caller allowed to read [`players`]' `None` as "nobody".
/// [`players`] has four callers in all and the other three
/// (`resolve::players_of` and two in `team_tests`) `expect` a relation the
/// state can answer. Every `CardInGraveyard` in the pool names `You` or
/// `EachPlayer`; one naming `Chosen` would be a bug in the card.
fn graveyard_options(
    filter: &Filter,
    rel: PlayerRel,
    state: &GameState,
    you: PlayerId,
    this: ObjectId,
) -> Vec<ObjectId> {
    let Some(seats) = players(rel, state, you) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for player in seats {
        out.extend(
            state
                .zones
                .list(ZoneLocation::Graveyard(player))
                .iter()
                .filter(|id| {
                    state
                        .object(**id)
                        .is_some_and(|o| matches(filter, state, o, you, this))
                })
                .copied(),
        );
    }
    out
}

/// Evaluates an [`Amount`].
#[must_use]
pub fn amount(
    amount: &Amount,
    state: &GameState,
    you: PlayerId,
    this: ObjectId,
    x: Option<u32>,
) -> u32 {
    match amount {
        Amount::Fixed(n) | Amount::NegXFixed(n) => *n,
        Amount::X | Amount::NegX => x.unwrap_or(0),
        // The magnitude, like every other arm here: the sign is
        // `Amount::is_negative`'s question and no caller of this reads one.
        Amount::Negated(inner) => self::amount(inner, state, you, this, x),
        Amount::Plus { base, offset } => {
            self::amount(base, state, you, this, x).saturating_add(*offset)
        }
        Amount::DoubleX => x.unwrap_or(0).saturating_mul(2),
        Amount::XPlusCommanderCasts => {
            x.unwrap_or(0)
                + state
                    .commander_casts
                    .get(you.get() as usize)
                    .copied()
                    .unwrap_or(0)
        }
        Amount::DistinctColorsAmong(filter) => {
            let mut colors = baylee_core::color::ColorSet::EMPTY;
            for id in state.zones.list(ZoneLocation::Battlefield) {
                if let Some(obj) = state.object(*id)
                    && matches(filter, state, obj, you, this)
                {
                    colors = colors.union(obj.characteristics().colors);
                }
            }
            u32::from(colors.len())
        }
        // `SubtypeSet::BASIC_LANDS` is CR 305.6's five and is already the
        // pool's one spelling of them, so this asks that set rather than
        // writing a sixth list of Plains/Island/Swamp/Mountain/Forest.
        // Union first and intersect once: a domain count is over *types*,
        // so two Forests are one and a Tundra is two.
        Amount::BasicLandTypesAmong(filter) => {
            let mut seen = baylee_core::types::SubtypeSet::EMPTY;
            for id in state.zones.list(ZoneLocation::Battlefield) {
                if let Some(obj) = state.object(*id)
                    && matches(filter, state, obj, you, this)
                {
                    seen.union_with(obj.characteristics().subtypes);
                }
            }
            baylee_core::types::SubtypeSet::BASIC_LANDS
                .iter()
                .filter(|t| seen.contains(*t))
                .count() as u32
        }
        Amount::SourcePower => state
            .object(this)
            .and_then(|o| o.characteristics().power)
            .map_or(0, |p| p.max(0) as u32),
        Amount::TargetPower | Amount::TargetCmc => 0, // resolved in resolve.rs
        Amount::CountOf { filter, zone } => {
            let objects: Vec<ObjectId> = match zone {
                ZoneSel::Battlefield => state.zones.list(ZoneLocation::Battlefield).clone(),
                ZoneSel::LibraryYou => state.zones.list(ZoneLocation::Library(you)).clone(),
                ZoneSel::GraveyardYou => state.zones.list(ZoneLocation::Graveyard(you)).clone(),
                ZoneSel::HandYou => state.zones.list(ZoneLocation::Hand(you)).clone(),
                ZoneSel::GraveyardAll => state
                    .players
                    .iter()
                    .flat_map(|p| {
                        state
                            .zones
                            .list(ZoneLocation::Graveyard(p.id))
                            .iter()
                            .copied()
                    })
                    .collect(),
            };
            objects
                .iter()
                .filter(|id| {
                    state
                        .object(**id)
                        .is_some_and(|o| matches(filter, state, o, you, this))
                })
                .count() as u32
        }
    }
}

/// Whether a card's stated [`Condition`] holds right now.
///
/// One vocabulary and one reading, for every ability kind that states a
/// condition. It began as `Engine::check_activation_condition`, a method
/// answering it for `AbilityDef::ActivatedConditional` alone — but a
/// condition is a sentence about the game and not about how the ability
/// gets used, and the intervening-`if` clause of CR 603.4 asks the same
/// sentence of a *triggered* ability. A second copy over there would have
/// been the third byte-identical reading of a DSL predicate this engine has
/// grown, which is the shape [`crate::effects::applies_to`] was extracted
/// to stop.
///
/// A free function over [`GameState`] rather than a method, because the
/// callers no longer share a receiver: `crate::trigger::collect` is handed
/// a state and a lookup and has no `Engine` to ask.
///
/// `you` is the ability's controller and `source` the object it is printed
/// on; both matter — `ControlCount` counts one player's battlefield and
/// `CountersOnSelf` reads one permanent. A `source` that has left the game
/// answers **false** for the counter conditions, which is the honest
/// answer: a permanent that is gone has no counters on it.
///
/// **It reads the state it is called in**, which is what the two CR 603.4
/// checks want and is worth saying because the first of them is not quite
/// the trigger event's own moment: triggers are collected in a sweep over
/// new journal entries, so a condition is read when the sweep runs rather
/// than at the instant the event happened. For a turn-based trigger — the
/// upkeep, which is where this clause is commonest — the sweep is the next
/// thing that runs and the two are the same moment.
#[must_use]
pub fn condition_holds(
    state: &GameState,
    you: PlayerId,
    source: ObjectId,
    condition: Condition,
) -> bool {
    match condition {
        Condition::ControlCount(filter, min) => {
            let count = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .filter(|id| {
                    state.object(**id).is_some_and(|o| {
                        o.controller == you && matches(filter, state, o, you, **id)
                    })
                })
                .count();
            count >= min as usize
        }
        // The same walk as above with the comparison turned round, written
        // out rather than shared: `count >= min` and `count <= max` read
        // the identical board and a helper taking an ordering would put the
        // one thing that differs behind a parameter.
        Condition::ControlCountAtMost(filter, max) => {
            let count = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .filter(|id| {
                    state.object(**id).is_some_and(|o| {
                        o.controller == you && matches(filter, state, o, you, **id)
                    })
                })
                .count();
            count <= max as usize
        }
        // `you` in the filter is the **opponent** being counted, not the
        // ability's controller: "an opponent controls four or more lands"
        // asks the question of each seat in turn, and a filter evaluated
        // with the asker's seat would read `ControlledByYou` backwards.
        Condition::OpponentControlCount(filter, min) => (0..state.players.len())
            .map(|i| PlayerId::new(i as u8))
            .filter(|id| state.is_opponent(*id, you))
            .any(|them| {
                state
                    .zones
                    .list(ZoneLocation::Battlefield)
                    .iter()
                    .filter(|id| {
                        state.object(**id).is_some_and(|o| {
                            o.controller == them && matches(filter, state, o, you, **id)
                        })
                    })
                    .count()
                    >= min as usize
            }),
        Condition::HandSizeAtMost(max) => {
            state.zones.list(ZoneLocation::Hand(you)).len() <= max as usize
        }
        Condition::HandSizeExactly(n) => {
            state.zones.list(ZoneLocation::Hand(you)).len() == n as usize
        }
        Condition::OpponentGraveyardCountAtLeast(min) => (0..state.players.len())
            .map(|i| PlayerId::new(i as u8))
            .filter(|id| state.is_opponent(*id, you))
            .any(|id| state.zones.list(ZoneLocation::Graveyard(id)).len() >= min as usize),
        // Your own graveyard, and only yours — threshold counts the cards
        // the *controller* of the ability has, so the seat is `you` and not
        // a scan. The line above is the same question asked of an opponent
        // and answers `any`, because "an opponent has seven" is true of a
        // table where one of three does.
        Condition::GraveyardCountAtLeast(min) => {
            state.zones.list(ZoneLocation::Graveyard(you)).len() >= min as usize
        }
        Condition::CountersOnSelf(kind, min) => state
            .object(source)
            .is_some_and(|o| o.counters.get(kind) >= u16::from(min)),
        Condition::CountersOnSelfExactly(kind, n) => state
            .object(source)
            .is_some_and(|o| o.counters.get(kind) == u16::from(n)),
        // `is_some_and`, so a source that is no longer in the arena does not
        // match: an ability is a separate object from its source the moment
        // it goes on the stack (CR 113.7a), and "if this land is tapped"
        // asked of a land that has left the battlefield has nothing to be
        // true of. Every other sentence here already fails the same way.
        Condition::Any(parts) => parts
            .iter()
            .any(|part| condition_holds(state, you, source, *part)),
        Condition::SourceMatches(filter) => state
            .object(source)
            .is_some_and(|o| matches(filter, state, o, you, source)),
    }
}

/// The intervening-`if` clause of a triggered ability (CR 603.4), asked of
/// an ability that may not print one at all.
///
/// An absent clause is the trivially true one, so this is what both of the
/// rule's two checks call: the clause is asked once where the ability would
/// trigger and once where it would resolve, and the two must be the same
/// question or a card would trigger on one reading and vanish on the other.
#[must_use]
pub fn intervening_if(
    state: &GameState,
    condition: Option<Condition>,
    you: PlayerId,
    source: ObjectId,
) -> bool {
    condition.is_none_or(|c| condition_holds(state, you, source, c))
}

/// Protection (CR 702.16): does `object` have protection from a filter
/// that `source` matches? Checked for damage, targeting, and blocking.
#[must_use]
pub fn protected_from(state: &GameState, object: ObjectId, source: ObjectId) -> bool {
    let (Some(obj), Some(src)) = (state.object(object), state.object(source)) else {
        return false;
    };
    state.effects.iter().any(|fx| {
        let baylee_cards_dsl::Modifier::ProtectionFrom(f) = fx.modifier else {
            return false;
        };
        crate::effects::applies_to(state, fx, obj)
            && matches(f, state, src, fx.controller, fx.source.unwrap_or(source))
    })
}

/// Hexproof (CR 702.11b) and shroud (CR 702.18a): does `object` refuse to
/// be targeted by a spell or ability `you` control?
///
/// Both keywords function only while the object is on the battlefield —
/// a card in hand printed with hexproof is targetable in the graveyard,
/// and every other zone. Player hexproof is a different thing entirely
/// (`Modifier::PlayerHexproof`, checked when a spell chooses a player).
#[must_use]
pub fn untargetable_by(state: &GameState, object: ObjectId, you: PlayerId) -> bool {
    let Some(obj) = state.object(object) else {
        return false;
    };
    if obj.zone != crate::zone::Zone::Battlefield {
        return false;
    }
    let keywords = obj.characteristics().keywords;
    // Shroud stops everyone, its own controller included.
    if keywords.contains(baylee_cards_dsl::KeywordSet::SHROUD) {
        return true;
    }
    // Hexproof stops opponents only (CR 702.11a), so a teammate may
    // target it — and in a game with no teams that is everyone else.
    keywords.contains(baylee_cards_dsl::KeywordSet::HEXPROOF)
        && state.is_opponent(obj.controller, you)
}

/// The players a spell or ability may target.
///
/// A player is untargetable only through an effect, never through a
/// characteristic — there is nothing on a player for a `Filter` to match —
/// so this is a list, not a filtered enumeration like the object half.
#[must_use]
pub fn target_player_options(state: &GameState, spec: &TargetSpec, you: PlayerId) -> Vec<PlayerId> {
    if !matches!(
        spec,
        TargetSpec::AnyTarget | TargetSpec::AnyPlayer | TargetSpec::AnyOpponent
    ) {
        return Vec::new();
    }
    let opponents_only = matches!(spec, TargetSpec::AnyOpponent);
    state
        .players
        .iter()
        .filter(|p| {
            if p.has_lost() {
                return false;
            }
            // "Target opponent" is a choice over a smaller set, not a
            // different kind of choice (CR 115.1) — and the set is the
            // opponents, so a teammate is out of it as surely as you are.
            if opponents_only && !state.is_opponent(p.id, you) {
                return false;
            }
            // Player hexproof (Everybody Lives!): can't be targeted by spells
            // or abilities at all.
            !state.effects.iter().any(|fx| {
                matches!(fx.modifier, baylee_cards_dsl::Modifier::PlayerHexproof)
                    && fx.controller == p.id
            })
        })
        .map(|p| p.id)
        .collect()
}

/// The object half of "any target" (CR 115.4): every creature, planeswalker
/// and battle on the battlefield.
///
/// Filtering by type rather than by a `Filter` is deliberate — the printed
/// words are a fixed list the rules maintain, not a card's own predicate.
fn any_target_objects(state: &GameState) -> Vec<ObjectId> {
    state
        .battlefield_view()
        .iter()
        .filter(|id| {
            state.object(**id).is_some_and(|o| {
                let types = o.characteristics().types;
                types.contains(baylee_core::types::TypeSet::CREATURE)
                    || types.contains(baylee_core::types::TypeSet::PLANESWALKER)
                    || types.contains(baylee_core::types::TypeSet::BATTLE)
            })
        })
        .copied()
        .collect()
}

/// Legal target options for a [`TargetSpec`] (empty = cannot be chosen).
#[must_use]
pub fn target_options(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    this: ObjectId,
) -> Vec<ObjectId> {
    let options = match spec {
        TargetSpec::Object(filter) => state
            .battlefield_view()
            .iter()
            .filter(|id| {
                state
                    .object(**id)
                    .is_some_and(|o| matches(filter, state, o, you, this))
            })
            .copied()
            .collect(),
        TargetSpec::Spell(filter) => state
            .zones
            .list(ZoneLocation::Stack)
            .iter()
            .filter(|id| {
                state.object(**id).is_some_and(|o| {
                    o.kind == crate::object::ObjectKind::Spell
                        && !o
                            .characteristics()
                            .keywords
                            .contains(baylee_cards_dsl::KeywordSet::UNCOUNTERABLE)
                        && matches(filter, state, o, you, this)
                })
            })
            .copied()
            .collect(),
        TargetSpec::CardInGraveyard(filter, rel) => {
            graveyard_options(filter, *rel, state, you, this)
        }
        TargetSpec::StackOrBattlefield(filter) => {
            let mut out: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Stack)
                .iter()
                .chain(state.battlefield_view().iter())
                .filter(|id| {
                    state
                        .object(**id)
                        .is_some_and(|o| matches(filter, state, o, you, this))
                })
                .copied()
                .collect();
            out.sort();
            out.dedup();
            out
        }
        TargetSpec::ThisObject => vec![this],
        TargetSpec::AbilityOnStack(filter) => state
            .zones
            .list(ZoneLocation::Stack)
            .iter()
            .filter(|id| {
                state.object(**id).is_some_and(|o| {
                    o.kind == crate::object::ObjectKind::AbilityOnStack
                        && matches(filter, state, o, you, this)
                })
            })
            .copied()
            .collect(),
        TargetSpec::SpellOrAbility(filter) => state
            .zones
            .list(ZoneLocation::Stack)
            .iter()
            .filter(|id| {
                state.object(**id).is_some_and(|o| {
                    matches!(
                        o.kind,
                        crate::object::ObjectKind::Spell
                            | crate::object::ObjectKind::AbilityOnStack
                    ) && matches(filter, state, o, you, this)
                })
            })
            .copied()
            .collect(),
        // "Any target" (CR 115.4) is a creature, a planeswalker, a battle
        // or a player. Only the object half is enumerated here; the players
        // come from `target_player_options`, and the two lists are offered
        // as one choice.
        TargetSpec::AnyTarget => any_target_objects(state),
        // EventObject is implicit (no player choice); player targeting
        // resolves via ChoosePlayer in the casting wizard.
        TargetSpec::EventObject
        | TargetSpec::Player(_)
        | TargetSpec::AnyPlayer
        | TargetSpec::AnyOpponent => vec![],
    };
    // Protection (CR 702.16b) keeps out matching sources; hexproof and
    // shroud (CR 702.11b/702.18b) keep out whole classes of chooser.
    options
        .into_iter()
        .filter(|id| !protected_from(state, *id, this) && !untargetable_by(state, *id, you))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::ObjectKind;
    use crate::state::CardLookup;
    use baylee_cards_dsl::KeywordSet;
    use baylee_core::ids::{CardIndex, SubtypeId};
    use baylee_core::preset::{FormatId, GamePreset, HouseRules, SeatController, SeatSpec};
    use baylee_core::types::{SubtypeSet, TypeSet};

    /// No cards: these tests are about keywords, not about whichever
    /// printed card happens to carry one.
    struct NoCards;
    impl CardLookup for NoCards {
        fn card(&self, _: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            None
        }
    }

    const P0: PlayerId = PlayerId::new(0);
    const P1: PlayerId = PlayerId::new(1);

    static ANY_CREATURE: Filter = Filter::HasType(TypeSet::CREATURE);

    fn empty_state() -> GameState {
        let seat = || SeatSpec {
            controller: SeatController::Open,
            capabilities: baylee_core::preset::SeatCapabilities::default(),
            deck: vec![],
            sideboard: vec![],
            commanders: vec![],
            starting_life: Some(20),
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        GameState::from_preset(
            &GamePreset {
                format: FormatId::Freeform,
                seed: 1,
                house_rules: HouseRules::default(),
                modifiers: vec![],
                prints: vec![],
                seats: vec![seat(), seat()],
            },
            &NoCards,
        )
        .expect("an empty two-seat board")
    }

    fn creature(state: &mut GameState, controller: PlayerId, keywords: KeywordSet) -> ObjectId {
        let name = state.names.intern("Test Creature");
        let id = state.create_bare(
            controller,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let b = state.object_mut(id).expect("just created").base_mut();
        b.types = TypeSet::CREATURE;
        b.power = Some(1);
        b.toughness = Some(1);
        b.keywords = keywords;
        id
    }

    /// A land on `controller`'s battlefield carrying `subtypes`.
    fn land(state: &mut GameState, controller: PlayerId, subtypes: &[SubtypeId]) -> ObjectId {
        let name = state.names.intern("Test Land");
        let id = state.create_bare(
            controller,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let b = state.object_mut(id).expect("just created").base_mut();
        b.types = TypeSet::LAND;
        b.subtypes = SubtypeSet::from_slice(subtypes);
        id
    }

    /// Domain counts **types**, and that is the whole reason it is a variant
    /// of its own rather than either of the two amounts that were already
    /// there. `Amount::CountOf` counts objects, so two Forests would answer
    /// 2 where the card wants 1 and a Tundra 1 where it wants 2; a land is
    /// colourless, so `Amount::DistinctColorsAmong` answers 0 for any board
    /// of them. Both readings are asserted here beside the right one, so a
    /// later simplification back onto either is a red test rather than a
    /// card that quietly pumps by the wrong number.
    #[test]
    fn a_domain_count_is_over_basic_land_types_and_not_over_lands() {
        use baylee_core::generated::subtypes::land;
        static DOMAIN: Amount = Amount::BasicLandTypesAmong(&Filter::YOUR_LAND);
        static YOUR_LANDS: Filter = Filter::YOUR_LAND;

        let mut state = empty_state();
        let forest = land(&mut state, P0, &[land::FOREST]);
        let count = |state: &GameState| self::amount(&DOMAIN, state, P0, forest, None);

        assert_eq!(count(&state), 1, "one Forest is one basic land type");
        land(&mut state, P0, &[land::FOREST]);
        assert_eq!(
            count(&state),
            1,
            "a second Forest adds no type — this is where a count of objects parts company"
        );
        land(&mut state, P0, &[land::PLAINS, land::ISLAND]);
        assert_eq!(count(&state), 3, "a Tundra is two types on one land");
        land(&mut state, P1, &[land::MOUNTAIN, land::SWAMP]);
        assert_eq!(
            count(&state),
            3,
            "and the filter is `lands you control`: an opponent's Badlands is not yours"
        );

        // The two amounts this is not, over the same board.
        assert_eq!(
            self::amount(
                &Amount::CountOf {
                    filter: &YOUR_LANDS,
                    zone: ZoneSel::Battlefield
                },
                &state,
                P0,
                forest,
                None
            ),
            3,
            "three lands, four types — a count of objects is not domain"
        );
        assert_eq!(
            self::amount(
                &Amount::DistinctColorsAmong(&YOUR_LANDS),
                &state,
                P0,
                forest,
                None
            ),
            0,
            "and a land is colourless, whatever mana it makes"
        );

        // CR 305.6 names five and no other land type joins them.
        land(&mut state, P0, &[land::DESERT, land::GATE]);
        assert_eq!(
            count(&state),
            3,
            "Desert and Gate are land types (CR 205.3i) and not basic ones"
        );
    }

    /// `Filter::CmcAtMostX` reads its bound off the **source**, which is the
    /// only place the announced number exists at match time: a filter is
    /// evaluated with `this` in hand and with no `x` beside it. The pool
    /// cards that print "mana value X or less" are searches, so the bound
    /// being wrong is a tutor that finds a card it may not find.
    ///
    /// Zero is asserted beside the two live bounds because that is the
    /// answer for an ability that announced nothing, and it has to *mean*
    /// zero: a filter that could not read its bound and matched everything
    /// would be a tutor with no price at all.
    #[test]
    fn a_mana_value_bound_of_x_is_read_off_the_ability_source() {
        static WITHIN_X: Filter = Filter::CmcAtMostX;
        let mut state = empty_state();
        let source = creature(&mut state, P0, KeywordSet::EMPTY);
        let free = creature(&mut state, P0, KeywordSet::EMPTY);
        let two = creature(&mut state, P0, KeywordSet::EMPTY);
        state
            .object_mut(two)
            .expect("just created")
            .base_mut()
            .mana_cost = baylee_core::mana::ManaCost::parse("{1}{G}");

        let reaches = |state: &GameState, id: ObjectId| {
            let obj = state.object(id).expect("on the battlefield");
            matches(&WITHIN_X, state, obj, P0, source)
        };

        assert!(
            reaches(&state, free),
            "X is 0 and a cost of nothing is 0 or less"
        );
        assert!(
            !reaches(&state, two),
            "and a two-drop is not — an unreadable bound that matched would              be a tutor with no price"
        );

        state.object_mut(source).expect("just created").x_value = 2;
        assert!(reaches(&state, two), "X = 2 reaches a two-drop");

        state.object_mut(source).expect("just created").x_value = 1;
        assert!(
            !reaches(&state, two),
            "and X = 1 does not: the bound is the announced number and not              merely whether one was announced"
        );
    }

    /// Who `chooser` may point a creature-targeting spell at.
    fn targets(state: &GameState, chooser: PlayerId, source: ObjectId) -> Vec<ObjectId> {
        target_options(&TargetSpec::Object(&ANY_CREATURE), state, chooser, source)
    }

    /// Gives `object` protection from `filter`, as a continuous effect the
    /// seat `controller` controls.
    fn protect(
        state: &mut GameState,
        controller: PlayerId,
        object: ObjectId,
        filter: &'static Filter,
    ) {
        let filter_of = crate::effects::EffectFilter::object(state, object);
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(object),
            controller,
            layer: baylee_cards_dsl::Modifier::ProtectionFrom(filter).layer(),
            timestamp: 1,
            duration: baylee_cards_dsl::Duration::Indefinitely,
            filter: filter_of,
            modifier: baylee_cards_dsl::Modifier::ProtectionFrom(filter),
        });
    }

    /// Protection (CR 702.16) is two questions and both are asked of a
    /// different object: which permanent *has* it, and whether the thing
    /// coming at it matches the filter. An effect naming one creature does
    /// not protect the one beside it, and a creature with protection from
    /// artifacts is not protected from a creature.
    #[test]
    fn protection_is_read_off_the_protected_object_and_the_thing_it_faces() {
        static ARTIFACTS: Filter = Filter::ARTIFACT;
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::EMPTY);
        let beside = creature(&mut state, P0, KeywordSet::EMPTY);
        let theirs = creature(&mut state, P1, KeywordSet::EMPTY);

        assert!(
            !protected_from(&state, mine, theirs),
            "nothing is protected from anything to begin with"
        );
        protect(&mut state, P0, mine, &ANY_CREATURE);
        assert!(protected_from(&state, mine, theirs));
        assert!(
            protected_from(&state, mine, beside),
            "protection from creatures is from all of them, mine included"
        );
        assert!(
            !protected_from(&state, beside, theirs),
            "and the creature standing next to it has none"
        );

        let other = creature(&mut state, P0, KeywordSet::EMPTY);
        protect(&mut state, P0, other, &ARTIFACTS);
        assert!(
            !protected_from(&state, other, theirs),
            "protection from artifacts is not protection from a creature"
        );
    }

    /// **Whose opponent** is read off the seat that controls the protection
    /// effect and not off the permanent that carries it (CR 109.5: "you" is
    /// the controller of the ability). The two are the same on every printed
    /// card, which is exactly why a reader that used the permanent's
    /// controller would look right — so the case that tells them apart is
    /// the one worth writing down.
    #[test]
    fn protection_from_an_opponent_means_the_effects_controllers_opponent() {
        static OPPONENTS: Filter = Filter::ControlledByOpponent;
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::EMPTY);
        let beside = creature(&mut state, P0, KeywordSet::EMPTY);
        let theirs = creature(&mut state, P1, KeywordSet::EMPTY);

        protect(&mut state, P0, mine, &OPPONENTS);
        assert!(protected_from(&state, mine, theirs));
        assert!(
            !protected_from(&state, mine, beside),
            "a creature I control is not one my opponent controls"
        );

        // The same protection, granted by the seat across the table: the
        // filter now reads from their side, so it is my own creature the
        // permanent is protected from.
        let odd = creature(&mut state, P0, KeywordSet::EMPTY);
        protect(&mut state, P1, odd, &OPPONENTS);
        assert!(
            protected_from(&state, odd, beside),
            "from their seat, a creature I control is an opponent's"
        );
        assert!(
            !protected_from(&state, odd, theirs),
            "and their own creature is not"
        );
    }

    /// An object that is not in the arena is protected from nothing and
    /// protects against nothing: both halves answer `false` rather than
    /// panicking, which is what lets a damage step ask about a creature that
    /// a state-based action has already taken away.
    #[test]
    fn protection_asked_about_something_that_is_gone_is_simply_false() {
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::EMPTY);
        let theirs = creature(&mut state, P1, KeywordSet::EMPTY);
        protect(&mut state, P0, mine, &ANY_CREATURE);
        assert!(protected_from(&state, mine, theirs));

        let gone = ObjectId::new(9_999, 0);
        assert!(!protected_from(&state, gone, theirs));
        assert!(!protected_from(&state, mine, gone));
    }

    /// Hexproof (CR 702.11b) stops opponents and nobody else.
    #[test]
    fn hexproof_hides_a_creature_from_its_controllers_opponents() {
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::HEXPROOF);
        let plain = creature(&mut state, P0, KeywordSet::EMPTY);
        let source = creature(&mut state, P1, KeywordSet::EMPTY);

        let theirs = targets(&state, P1, source);
        assert!(
            !theirs.contains(&mine),
            "an opponent could target a hexproof creature"
        );
        assert!(
            theirs.contains(&plain),
            "the opponent lost sight of an ordinary creature too"
        );

        assert!(
            targets(&state, P0, mine).contains(&mine),
            "hexproof stopped its own controller"
        );
    }

    /// Shroud (CR 702.18a) stops everyone, controller included — that is
    /// the whole difference between the two keywords.
    #[test]
    fn shroud_hides_a_creature_from_everyone() {
        let mut state = empty_state();
        let shrouded = creature(&mut state, P0, KeywordSet::SHROUD);
        let source = creature(&mut state, P1, KeywordSet::EMPTY);

        assert!(!targets(&state, P1, source).contains(&shrouded));
        assert!(
            !targets(&state, P0, shrouded).contains(&shrouded),
            "shroud let its own controller target it"
        );
    }

    /// Both keywords are battlefield-only (CR 702.11b): a card printed
    /// with hexproof is an ordinary target in any other zone.
    #[test]
    fn hexproof_does_not_function_outside_the_battlefield() {
        let mut state = empty_state();
        let card = creature(&mut state, P0, KeywordSet::HEXPROOF);
        state
            .move_object(
                card,
                ZoneLocation::Graveyard(P0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::TurnBased,
            )
            .expect("the creature dies");
        assert!(
            !untargetable_by(&state, card, P1),
            "hexproof kept working in the graveyard"
        );
    }

    /// `LacksType(t)` and `Not(&HasType(t))` are the same predicate, which
    /// is what lets the pool spell "not a creature" exactly one way.
    ///
    /// The two arms of the `match` above are literal negations of each
    /// other, so this could be read off the source — and that is precisely
    /// why it is worth a test: the equivalence is an *invariant* the card
    /// pool now depends on, not an accident of how those two lines happen
    /// to be written today. Every type, and both answers, so a rule that
    /// stopped negating would fail here rather than in a card.
    #[test]
    fn lacking_a_type_is_not_having_it() {
        // Written out in pairs rather than built in the loop: `Not` holds a
        // `&'static Filter`, and a reference taken to a value made from a
        // loop variable cannot be promoted (E0716). The pairing is the
        // assertion, so writing it is no loss.
        static NEGATIONS: &[(TypeSet, Filter, Filter)] = &[
            (
                TypeSet::ARTIFACT,
                Filter::LacksType(TypeSet::ARTIFACT),
                Filter::Not(&Filter::HasType(TypeSet::ARTIFACT)),
            ),
            (
                TypeSet::CREATURE,
                Filter::LacksType(TypeSet::CREATURE),
                Filter::Not(&Filter::HasType(TypeSet::CREATURE)),
            ),
            (
                TypeSet::ENCHANTMENT,
                Filter::LacksType(TypeSet::ENCHANTMENT),
                Filter::Not(&Filter::HasType(TypeSet::ENCHANTMENT)),
            ),
            (
                TypeSet::INSTANT,
                Filter::LacksType(TypeSet::INSTANT),
                Filter::Not(&Filter::HasType(TypeSet::INSTANT)),
            ),
            (
                TypeSet::KINDRED,
                Filter::LacksType(TypeSet::KINDRED),
                Filter::Not(&Filter::HasType(TypeSet::KINDRED)),
            ),
            (
                TypeSet::LAND,
                Filter::LacksType(TypeSet::LAND),
                Filter::Not(&Filter::HasType(TypeSet::LAND)),
            ),
            (
                TypeSet::PLANESWALKER,
                Filter::LacksType(TypeSet::PLANESWALKER),
                Filter::Not(&Filter::HasType(TypeSet::PLANESWALKER)),
            ),
            (
                TypeSet::SORCERY,
                Filter::LacksType(TypeSet::SORCERY),
                Filter::Not(&Filter::HasType(TypeSet::SORCERY)),
            ),
            (
                TypeSet::BATTLE,
                Filter::LacksType(TypeSet::BATTLE),
                Filter::Not(&Filter::HasType(TypeSet::BATTLE)),
            ),
        ];

        let mut state = empty_state();
        let obj = creature(&mut state, P0, KeywordSet::EMPTY);
        let object = state.object(obj).expect("just created");
        for (t, lacks, negated) in NEGATIONS {
            assert_eq!(
                matches(lacks, &state, object, P0, obj),
                matches(negated, &state, object, P0, obj),
                "LacksType and Not(HasType) disagreed about {t:?}"
            );
        }

        // The counter-test: the object really is a creature and really is
        // not a land, so the loop above compared both answers and not one
        // answer nine times.
        assert!(matches(&Filter::CREATURE, &state, object, P0, obj));
        assert!(matches(&Filter::NONLAND, &state, object, P0, obj));
        assert!(!matches(&Filter::NONCREATURE, &state, object, P0, obj));
    }

    /// The `None` is the whole point of the signature. Answering `vec![]`
    /// for a relation this function cannot resolve reads exactly like "no
    /// seats matched", and that is what shipped Abraded Bluffs as a land
    /// that deals no damage and Bojuka Bog as a swamp: only a caller
    /// holding a `Resolution` can answer these two, and it must not be able
    /// to swallow them silently.
    #[test]
    fn a_relation_the_state_cannot_answer_is_none_rather_than_nobody() {
        let state = empty_state();
        assert_eq!(players(PlayerRel::ControllerOfTarget, &state, P0), None);
        assert_eq!(players(PlayerRel::Chosen, &state, P0), None);

        assert_eq!(players(PlayerRel::You, &state, P0), Some(vec![P0]));
        assert_eq!(players(PlayerRel::Opponent, &state, P0), Some(vec![P1]));
        assert_eq!(
            players(PlayerRel::EachPlayer, &state, P0),
            Some(vec![P0, P1])
        );
        assert_eq!(
            players(PlayerRel::EachOpponent, &state, P1),
            Some(vec![P0]),
            "the relation is read from whoever is asking"
        );
    }

    /// A player who has lost is not a player the game asks anything of —
    /// "each opponent loses 1 life" resolving after somebody conceded must
    /// not find them.
    #[test]
    fn a_player_who_has_lost_is_no_longer_each_player() {
        let mut state = empty_state();
        state.players[1].loss = Some(crate::event::LossReason::Conceded);
        assert_eq!(players(PlayerRel::EachPlayer, &state, P0), Some(vec![P0]));
        assert_eq!(players(PlayerRel::EachOpponent, &state, P0), Some(vec![]));
        assert_eq!(
            players(PlayerRel::You, &state, P0),
            Some(vec![P0]),
            "and you are still you"
        );
    }

    /// "If you control three or more creatures" is about the asker's own
    /// board, whoever else has one.
    #[test]
    fn a_control_count_counts_the_askers_own_permanents() {
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::EMPTY);
        creature(&mut state, P0, KeywordSet::EMPTY);
        creature(&mut state, P1, KeywordSet::EMPTY);

        let two = Condition::ControlCount(&ANY_CREATURE, 2);
        let three = Condition::ControlCount(&ANY_CREATURE, 3);
        assert!(condition_holds(&state, P0, mine, two));
        assert!(
            !condition_holds(&state, P0, mine, three),
            "theirs is theirs"
        );
        assert!(!condition_holds(&state, P1, mine, two));
    }

    /// A power comparison reads the **projected** number, and an object
    /// with no power at all is not "a creature with power 4 or greater".
    ///
    /// Six cards in this pool print a power or toughness comparison and
    /// every one of them had the ability that states it taken off the card
    /// while `Filter` could not say it — Bonders' Enclave was a land with
    /// no draw, Access Tunnel a land with one ability. The `is_some_and` is
    /// the half that is easy to get wrong in the other direction: a land
    /// answering `0 <= 3` would make Access Tunnel target one.
    #[test]
    fn a_power_comparison_reads_a_projected_number_and_an_object_that_has_none_is_not_small() {
        let mut state = empty_state();
        let c = creature(&mut state, P0, KeywordSet::EMPTY);
        let l = land(&mut state, P0, &[]);
        let ask = |state: &GameState, f: &Filter, id: ObjectId| {
            matches(f, state, state.object(id).expect("still here"), P0, id)
        };

        // A 1/1 to begin with.
        assert!(ask(&state, &Filter::PowerAtMost(3), c));
        assert!(!ask(&state, &Filter::PowerAtLeast(4), c));
        assert!(!ask(&state, &Filter::ToughnessAtLeast(4), c));

        // The land has neither number, so it is on **no** side of either
        // comparison — the bound is not a default of nought.
        assert!(!ask(&state, &Filter::PowerAtMost(3), l));
        assert!(!ask(&state, &Filter::PowerAtLeast(4), l));
        assert!(!ask(&state, &Filter::ToughnessAtLeast(4), l));
        assert!(
            !ask(&state, &Filter::ToughnessAtMost(3), l),
            "the predicate that was already here answers the same way, \
             which is what makes the three new ones its siblings"
        );

        // Grown to a 4/4 — through the base characteristics, because this
        // module tests the predicate and not the layer system. A card that
        // reached 4 power under an anthem reaches it here the same way,
        // since `matches` is handed the projected characteristics either
        // way.
        {
            let b = state.object_mut(c).expect("still here").base_mut();
            b.power = Some(4);
            b.toughness = Some(4);
        }
        state.invalidate_projections();
        assert!(ask(&state, &Filter::PowerAtLeast(4), c));
        assert!(ask(&state, &Filter::ToughnessAtLeast(4), c));
        assert!(
            !ask(&state, &Filter::PowerAtMost(3), c),
            "and the creature has grown out of the other card's restriction"
        );
    }

    /// "You control no artifacts" is not the negation of a minimum with the
    /// number moved: it is a different comparison, and Glimmervoid and
    /// Thran Quarry both shipped with the clause **off** while it could not
    /// be said — which is a land that never sacrifices itself.
    #[test]
    fn a_control_count_downwards_is_a_different_question_from_one_upwards() {
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::EMPTY);

        let none = Condition::ControlCountAtMost(&ANY_CREATURE, 0);
        let one = Condition::ControlCountAtMost(&ANY_CREATURE, 1);
        assert!(!condition_holds(&state, P0, mine, none));
        assert!(condition_holds(&state, P0, mine, one));
        assert!(
            condition_holds(&state, P1, mine, none),
            "the other seat controls none of it"
        );

        // On a board with none of them the sentence turns over, and its
        // upward twin turns over with it — the two really are reading one
        // count and not two.
        let bare = empty_state();
        assert!(condition_holds(&bare, P0, mine, none));
        assert!(!condition_holds(
            &bare,
            P0,
            mine,
            Condition::ControlCount(&ANY_CREATURE, 1)
        ));
    }

    /// "An opponent controls four or more lands" is `any` over the seats
    /// and never your own board: Tectonic Edge had been activating on a
    /// table where nobody else had a land at all, which is a Wasteland.
    #[test]
    fn an_opponent_control_count_is_any_other_seat_and_never_your_own() {
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::EMPTY);
        creature(&mut state, P0, KeywordSet::EMPTY);
        let two = Condition::OpponentControlCount(&ANY_CREATURE, 2);

        assert!(
            !condition_holds(&state, P0, mine, two),
            "two of your own are not two of an opponent's"
        );
        assert!(
            condition_holds(&state, P1, mine, two),
            "and they are, asked from the other side"
        );

        creature(&mut state, P1, KeywordSet::EMPTY);
        creature(&mut state, P1, KeywordSet::EMPTY);
        assert!(condition_holds(&state, P0, mine, two));
    }

    /// Hellbent and Library of Alexandria are one word apart and never the
    /// same board: "at most nought" is true of an empty hand and "exactly
    /// seven" stops being true the moment the seventh card is drawn on.
    #[test]
    fn a_hand_size_is_counted_on_the_asking_seats_own_hand() {
        let mut state = empty_state();
        let source = creature(&mut state, P0, KeywordSet::EMPTY);
        let deal = |state: &mut GameState, who: PlayerId, n: usize| {
            for _ in 0..n {
                let name = state.names.intern("Test Card");
                state.create_bare(who, ObjectKind::Card, name, ZoneLocation::Hand(who));
            }
        };

        let empty = Condition::HandSizeAtMost(0);
        let seven = Condition::HandSizeExactly(7);
        assert!(condition_holds(&state, P0, source, empty));
        assert!(!condition_holds(&state, P0, source, seven));

        deal(&mut state, P1, 7);
        assert!(
            condition_holds(&state, P0, source, empty),
            "their hand is not yours"
        );
        assert!(!condition_holds(&state, P0, source, seven));
        assert!(condition_holds(&state, P1, source, seven));

        deal(&mut state, P0, 7);
        assert!(!condition_holds(&state, P0, source, empty));
        assert!(condition_holds(&state, P0, source, seven));

        deal(&mut state, P0, 1);
        assert!(
            !condition_holds(&state, P0, source, seven),
            "the eighth card is what makes this land stop working"
        );
    }

    /// Two sentences that look alike and are not: "if it has three or more"
    /// against "if it has exactly three". A card that removes counters as a
    /// cost sits on both sides of that difference.
    #[test]
    fn at_least_and_exactly_are_different_questions() {
        let mut state = empty_state();
        let source = creature(&mut state, P0, KeywordSet::EMPTY);
        state
            .object_mut(source)
            .expect("just made it")
            .counters
            .add(baylee_cards_dsl::CounterKind::P1P1, 3);

        let kind = baylee_cards_dsl::CounterKind::P1P1;
        assert!(condition_holds(
            &state,
            P0,
            source,
            Condition::CountersOnSelf(kind, 3)
        ));
        assert!(condition_holds(
            &state,
            P0,
            source,
            Condition::CountersOnSelf(kind, 2)
        ));
        assert!(!condition_holds(
            &state,
            P0,
            source,
            Condition::CountersOnSelf(kind, 4)
        ));
        assert!(condition_holds(
            &state,
            P0,
            source,
            Condition::CountersOnSelfExactly(kind, 3)
        ));
        assert!(!condition_holds(
            &state,
            P0,
            source,
            Condition::CountersOnSelfExactly(kind, 2)
        ));
    }

    /// CR 113.7a: an ability is a separate object from its source the
    /// moment it goes on the stack, so "if this land is tapped" asked of a
    /// land that has left the battlefield has nothing to be true of.
    #[test]
    fn a_condition_about_a_source_that_is_gone_is_false() {
        let mut state = empty_state();
        let source = creature(&mut state, P0, KeywordSet::EMPTY);
        let about_itself = Condition::SourceMatches(&ANY_CREATURE);
        assert!(condition_holds(&state, P0, source, about_itself));

        state.arena.remove(source);
        assert!(!condition_holds(&state, P0, source, about_itself));
        assert!(
            !condition_holds(
                &state,
                P0,
                source,
                Condition::CountersOnSelf(baylee_cards_dsl::CounterKind::P1P1, 0)
            ),
            "every other sentence about the source fails the same way, \
             including the one a zero would otherwise make trivially true"
        );
    }

    /// CR 603.4 asks the clause twice — once where the ability would
    /// trigger and once where it would resolve — so both readings go
    /// through one function. An ability that prints no clause at all is the
    /// trivially true one, and an absent clause answering `false` would
    /// silence every trigger in the pool that does not print one.
    #[test]
    fn an_ability_with_no_intervening_if_has_a_true_one() {
        let mut state = empty_state();
        let source = creature(&mut state, P0, KeywordSet::EMPTY);

        assert!(intervening_if(&state, None, P0, source));
        assert!(intervening_if(
            &state,
            Some(Condition::ControlCount(&ANY_CREATURE, 1)),
            P0,
            source
        ));
        assert!(!intervening_if(
            &state,
            Some(Condition::ControlCount(&ANY_CREATURE, 2)),
            P0,
            source
        ));
    }
}
