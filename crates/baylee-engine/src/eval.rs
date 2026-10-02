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
#[allow(clippy::too_many_lines)] // one arm per `Filter` variant, and no wildcard
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
        Filter::WithSingleTarget => {
            obj.targets.len() + obj.second_targets().len() + obj.target_players.len() == 1
        }
        Filter::ControlledByYou => obj.controller == you,
        Filter::ControlledByOpponent => state.is_opponent(obj.controller, you),
        Filter::ControlledByActivePlayer => obj.controller == state.turn.active,
        Filter::ControlledByDefendingPlayer => {
            state.turn.phase == crate::turn::Phase::Combat
                && state.is_opponent(obj.controller, state.turn.active)
        }
        Filter::OwnedByYou => obj.owner == you,
        Filter::Tapped => obj.status.contains(Status::TAPPED),
        Filter::Untapped => !obj.status.contains(Status::TAPPED),
        // A lookup and not a scan: this arm runs once per object whenever a
        // filter is walked over the battlefield (`CombatState`'s doc).
        Filter::Attacking => state.combat.is_attacking(obj.id),
        Filter::Blocking => state.combat.blockers.iter().any(|b| b.blocker == obj.id),
        // Blocked or unblocked is settled as blockers are declared (CR
        // 509.1h), which is the declare-blockers step's first act; from
        // there to the end of combat an attacker is one or the other.
        Filter::Unblocked => {
            state.combat.is_attacking(obj.id)
                && !state.combat.is_blocked(obj.id)
                && matches!(
                    state.turn.step,
                    crate::turn::Step::DeclareBlockers
                        | crate::turn::Step::CombatDamageFirst
                        | crate::turn::Step::CombatDamage
                        | crate::turn::Step::CombatEnd
                )
        }
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
        Filter::AttackedThisTurn => state.per_turn.attacked.contains(&(obj.id, obj.version)),
        Filter::ControlledSinceTurnBegan => {
            obj.zone == crate::zone::Zone::Battlefield
                && state
                    .players
                    .get(obj.controller.get() as usize)
                    .is_some_and(|p| obj.controlled_since <= p.turn_start_timestamp)
        }
        Filter::PutIntoGraveyardThisTurn => state.per_turn.entered_graveyard.contains(&obj.id),
        // The object's own counters. A leaves-the-battlefield trigger asks
        // the object as it last existed there, and `trigger::departed_matches`
        // hands in that object, counters and all.
        Filter::HasCounter(kind) => obj.counters.get(*kind) > 0,
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
        Filter::IsAttached => obj.attached_to.is_some(),
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
        Filter::CmcAtMost(n) => chars.mana_value() <= *n,
        // The bound is the announced X on the ability's own source, which is
        // where `cast_wizard` writes it and what `res.x` is read from one
        // layer up. A source that is gone, or that announced nothing, bounds
        // at 0 rather than at everything: an unreadable bound that found the
        // whole library would be a tutor with no price.
        Filter::CmcAtMostX => chars.mana_value() <= announced_x(state, this),
        Filter::CmcExactlyX => chars.mana_value() == announced_x(state, this),
        // Converge's number, off the source where the payment wrote it; no
        // record is no mana spent, and so no colors.
        Filter::CmcAtMostColorsSpent => {
            let colors = state
                .object(this)
                .and_then(|o| o.paid.as_ref())
                .map_or(0, |p| p.colors_spent.len());
            chars.mana_value() <= u32::from(colors)
        }
        Filter::CmcAtLeast(n) => chars.mana_value() >= *n,
        Filter::ToughnessAtMost(n) => chars.toughness.is_some_and(|t| t <= *n),
        Filter::ToughnessAtLeast(n) => chars.toughness.is_some_and(|t| t >= *n),
        // `is_some_and`, so an object with no power at all — a land, an
        // instant on the stack — is not "a creature with power 4 or
        // greater" by default. The three neighbours above answer the same
        // way and for the same reason.
        Filter::PowerAtLeast(n) => chars.power.is_some_and(|p| p >= *n),
        Filter::PowerAtMost(n) => chars.power.is_some_and(|p| p <= *n),
        Filter::PowerLessThanSourcePower => below_source_power(chars.power, state, this),
        Filter::ToughnessLessThanSourcePower => below_source_power(chars.toughness, state, this),
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

/// Who controls the permanent `source` is attached to: "enchanted land's
/// controller", which need not be the Aura's own (CR 303.4e). `None` when the
/// source is gone or attached to nothing, or its host has left the game's
/// players.
#[must_use]
pub fn controller_of_attached(state: &GameState, source: ObjectId) -> Option<PlayerId> {
    let host = state.object(source)?.attached_to?;
    let seat = state.object(host)?.controller;
    (!state.has_left(seat)).then_some(seat)
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
        PlayerRel::ActivePlayer => state
            .players
            .iter()
            .filter(|p| p.id == state.turn.active && !p.has_lost())
            .map(|p| p.id)
            .collect(),
        PlayerRel::ControllerOfTarget
        | PlayerRel::ControllerOfEvent
        | PlayerRel::Chosen
        | PlayerRel::DamagedPlayer
        | PlayerRel::ControllerOfAttached => {
            return None;
        }
    })
}

/// The graveyard cards a `CardInGraveyard` spec may point at.
///
/// Legality is enumerated *before* any resolution exists, so the two context
/// relations have no answer here and an empty list is the honest one — this
/// is the **only** caller allowed to read [`players`]' `None` as "nobody".
/// [`players`] has four callers in all and the other three
/// (`resolve::players_of` and two in `team_tests`) `expect` a relation the
/// state can answer.
///
/// `Chosen` is the exception, and it is not a context relation here: it is
/// "from a single graveyard" (Unlicensed Hearse), and the graveyard is chosen
/// as part of choosing the targets. So every graveyard is enumerated — what
/// the offer counts, and what CR 608.2b re-checks a target against — and
/// `start_activation` narrows the list to the graveyard its player named.
fn graveyard_options(
    filter: &Filter,
    rel: PlayerRel,
    state: &GameState,
    you: PlayerId,
    this: ObjectId,
) -> Vec<ObjectId> {
    let rel = if rel == PlayerRel::Chosen {
        PlayerRel::EachPlayer
    } else {
        rel
    };
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

/// "The tapped creature's power" (station, CR 702.184a): the permanent the
/// payment of `paid_on`'s cost tapped, at its power now if it is still on
/// the battlefield as that object, and as it last existed there otherwise
/// (CR 608.2h). Negative power puts no counters (CR 107.1b); nothing
/// tapped, 0.
#[must_use]
pub fn tapped_power(state: &GameState, paid_on: ObjectId) -> u32 {
    let Some((tapped, version)) = state
        .object(paid_on)
        .and_then(|o| o.paid.as_ref())
        .and_then(|p| p.tapped)
    else {
        return 0;
    };
    let power = state
        .object(tapped)
        .filter(|o| o.zone == crate::zone::Zone::Battlefield && o.version == version)
        .map(|o| o.characteristics().power.unwrap_or(0))
        .or_else(|| {
            state
                .ltb_powers
                .iter()
                .find(|(id, _)| *id == tapped)
                .map(|(_, power)| *power)
        })
        .unwrap_or(0);
    u32::try_from(power.max(0)).unwrap_or(0)
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
        Amount::SaturatingSub { base, subtract } => {
            self::amount(base, state, you, this, x).saturating_sub(*subtract)
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
            for id in state.battlefield_seen() {
                if let Some(obj) = state.object(id)
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
            for id in state.battlefield_seen() {
                if let Some(obj) = state.object(id)
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
        Amount::CountersOnSource(kind) => state
            .object(this)
            .map_or(0, |o| u32::from(o.counters.get(*kind))),
        // Resolved in resolve.rs, which has the stack object these read.
        Amount::TargetPower
        | Amount::TargetCmc
        | Amount::EventLastToughness
        | Amount::EventAmount
        | Amount::TargetsPutIntoGraveyard => 0,
        // The object the payment wrote it on. A resolution asks
        // `resolve::amount2`, which reads the stack object: an activated
        // ability's source is the permanent and its payment is on the
        // ability. Here, with no resolution, `this` is all there is, which
        // answers for a spell (its own source) and 0 for anything else.
        Amount::SacrificedManaValue => state
            .object(this)
            .and_then(|o| o.paid.as_ref())
            .and_then(|p| p.sacrificed_mana_value)
            .unwrap_or(0),
        Amount::ManaSpentToCast => state
            .object(this)
            .and_then(|o| o.paid.as_ref())
            .map_or(0, |p| p.mana_spent),
        Amount::TappedPower => tapped_power(state, this),
        Amount::CreaturesDiedThisTurn => state.per_turn.creatures_died,
        Amount::UntappedLandsAtTurnStart => state.per_turn.untapped_lands_at_start,
        Amount::DamageDealtToYouThisTurn => state
            .per_turn
            .damage_dealt_to
            .get(you.get() as usize)
            .copied()
            .unwrap_or(0),
        Amount::CountOf { filter, zone } => count_in_zone(filter, *zone, state, you, this),
    }
}

/// Counts matching objects in the selected zone from this ability's perspective.
fn count_in_zone(
    filter: &Filter,
    zone: ZoneSel,
    state: &GameState,
    you: PlayerId,
    this: ObjectId,
) -> u32 {
    let objects: Vec<ObjectId> = match zone {
        ZoneSel::Battlefield => state.battlefield_view(),
        ZoneSel::LibraryYou => state.zones.list(ZoneLocation::Library(you)).clone(),
        ZoneSel::GraveyardYou => state.zones.list(ZoneLocation::Graveyard(you)).clone(),
        ZoneSel::HandActivePlayer => state
            .zones
            .list(ZoneLocation::Hand(state.turn.active))
            .clone(),
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
#[allow(clippy::too_many_lines)] // one arm per `Condition`: the match is the list
pub fn condition_holds(
    state: &GameState,
    you: PlayerId,
    source: ObjectId,
    condition: Condition,
) -> bool {
    match condition {
        Condition::YourTurn => state.turn.active == you,
        // The announced X on the source, where `cast_wizard` writes it and
        // where `Filter::CmcAtMostX` reads it.
        Condition::XAtLeast(n) => state.object(source).map_or(0, |o| o.x_value) >= n,
        // No last turn at the first upkeep of the game, so nothing was cast
        // in it and nothing wasn't: both sentences are false there.
        Condition::NoSpellsCastLastTurn => {
            state.previous_turn.is_some_and(|p| p.spells_by_all == 0)
        }
        Condition::YouCastNoSpellThisTurn => state.per_turn.spells_cast_by(you) == 0,
        Condition::APlayerCastLastTurnAtLeast(n) => state
            .previous_turn
            .is_some_and(|p| p.most_by_one >= u32::from(n)),
        Condition::ControlCount(filter, min) => {
            let count = state
                .battlefield_seen()
                .filter(|&id| {
                    state
                        .object(id)
                        .is_some_and(|o| o.controller == you && matches(filter, state, o, you, id))
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
                .battlefield_seen()
                .filter(|&id| {
                    state
                        .object(id)
                        .is_some_and(|o| o.controller == you && matches(filter, state, o, you, id))
                })
                .count();
            count <= max as usize
        }
        // The two walks above over the whole battlefield: nobody's side of
        // the table in particular.
        Condition::BattlefieldCount(filter, min) => {
            battlefield_count(state, you, filter) >= min as usize
        }
        Condition::BattlefieldCountAtMost(filter, max) => {
            battlefield_count(state, you, filter) <= max as usize
        }
        Condition::ControlDistinctNames(filter, min) => {
            // A phased-out land is treated as though it does not exist
            // (CR 702.26b), so its name is not one of yours.
            let mut names: Vec<baylee_core::ids::NameRef> = state
                .battlefield_seen()
                .filter_map(|id| state.object(id))
                .filter(|o| o.controller == you && matches(filter, state, o, you, o.id))
                .map(|o| o.characteristics().name)
                .filter(|name| *name != crate::state::NAMELESS)
                .collect();
            names.sort_unstable();
            names.dedup();
            names.len() >= min as usize
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
                    .battlefield_seen()
                    .filter(|&id| {
                        state.object(id).is_some_and(|o| {
                            o.controller == them && matches(filter, state, o, you, id)
                        })
                    })
                    .count()
                    >= min as usize
            }),
        Condition::LandsPlayedThisTurnAtLeast(n) => state
            .players
            .get(you.get() as usize)
            .is_some_and(|p| p.lands_played_this_turn >= n),
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
        Condition::CountersOnSelfBetween(kind, min, max) => state
            .object(source)
            .is_some_and(|o| (u16::from(min)..=u16::from(max)).contains(&o.counters.get(kind))),
        Condition::EnduringStory => state
            .players
            .get(usize::from(you.get()))
            .is_some_and(|p| p.enduring_story),
        Condition::CitysBlessing => state
            .players
            .get(usize::from(you.get()))
            .is_some_and(|p| p.citys_blessing),
        Condition::Station(min) => state.object(source).is_some_and(|o| {
            o.counters.get(baylee_cards_dsl::CounterKind::Charge) >= u16::from(min)
        }),
        // `is_some_and`, so a source that is no longer in the arena does not
        // match: an ability is a separate object from its source the moment
        // it goes on the stack (CR 113.7a), and "if this land is tapped"
        // asked of a land that has left the battlefield has nothing to be
        // true of. Every other sentence here already fails the same way.
        Condition::Any(parts) => parts
            .iter()
            .any(|part| condition_holds(state, you, source, *part)),
        Condition::Not(part) => !condition_holds(state, you, source, *part),
        // Dash's return (CR 702.109a): the rider the cast wrote goes with
        // the permanent the spell became and is given up by every other
        // move (`GameState::move_object`), so a permanent that left the
        // battlefield and came back is not the one the dash cost was paid
        // for (CR 400.7).
        Condition::DashCostPaid => state.object(source).is_some_and(|o| {
            o.zone == crate::zone::Zone::Battlefield
                && o.riders.contains(&crate::object::Rider::Dashed)
        }),
        // "Unless it escaped" (CR 702.138b): the rider the escape cast wrote,
        // kept by the spell and the permanent it becomes and by nothing
        // later (`GameState::move_object`).
        Condition::Escaped => state
            .object(source)
            .is_some_and(|o| o.riders.contains(&crate::object::Rider::Escaped)),
        Condition::SourceMatches(filter) => state
            .object(source)
            .is_some_and(|o| matches(filter, state, o, you, source)),
        Condition::DuringCombat => state.turn.phase == crate::turn::Phase::Combat,
        Condition::All(all) => all.iter().all(|c| condition_holds(state, you, source, *c)),
        Condition::OpponentsTurn => state.is_opponent(state.turn.active, you),
        Condition::DuringStep(kind) => state.turn.step.kind() == Some(kind),
        // CR 506.7: "before" a point of the turn is before that point's
        // place in it, whether or not the step itself happens (506.7e).
        Condition::BeforeStep(kind) => state.turn.position() < crate::turn::position_of(kind),
        Condition::CanSacrifice(filter) => {
            !controlled_matching(state, you, filter, you, source).is_empty()
        }
    }
}

/// The permanents `player` controls that `filter` matches: what a player
/// asked to sacrifice, destroy or return "a [filter]" may pick
/// (`Effect::SacrificeFilter` and its siblings), and what
/// [`Condition::CanSacrifice`] asks about.
///
/// `you` and `source` are the effect's own controller and source, which is
/// what a filter reads "you" and "this" as — not the player being asked. A
/// phased-out permanent is treated as though it does not exist (CR
/// 702.26b), so it is not one of them.
#[must_use]
pub fn controlled_matching(
    state: &GameState,
    player: PlayerId,
    filter: &Filter,
    you: PlayerId,
    source: ObjectId,
) -> Vec<ObjectId> {
    state
        .battlefield_seen()
        .filter(|id| {
            state
                .object(*id)
                .is_some_and(|o| o.controller == player && matches(filter, state, o, you, source))
        })
        .collect()
}

/// The permanents on the battlefield `filter` matches, whoever controls them,
/// each asked with its own id as the filter's object — the reading
/// [`Condition::ControlCount`] gives one side of the table.
///
/// A phased-out permanent is treated as though it does not exist (CR
/// 702.26b), so it is not on the battlefield this counts.
fn battlefield_count(state: &GameState, you: PlayerId, filter: &Filter) -> usize {
    state
        .battlefield_seen()
        .filter(|id| {
            state
                .object(*id)
                .is_some_and(|o| matches(filter, state, o, you, *id))
        })
        .count()
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

/// Does a static ability on `object` say it can't be the target of `source`
/// ([`baylee_cards_dsl::Modifier::CantBeTargetedBy`], Thrun, Breaker of
/// Silence)? `source` is the spell, or the source of the ability, doing the
/// targeting, and the filter is asked of it with the effect's controller as
/// "you" — the same reading [`protected_from`] gives protection, whose
/// targeting half this is.
#[must_use]
pub fn untargetable_by_source(state: &GameState, object: ObjectId, source: ObjectId) -> bool {
    let (Some(obj), Some(src)) = (state.object(object), state.object(source)) else {
        return false;
    };
    state.effects.iter().any(|fx| {
        let baylee_cards_dsl::Modifier::CantBeTargetedBy(f) = fx.modifier else {
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

/// Whether rules-modifying effects permit this Aura to enchant the host.
/// Targeting restrictions and the Aura's own enchant filter are separate.
#[must_use]
pub fn permits_enchantment(state: &GameState, host: ObjectId, aura: ObjectId) -> bool {
    let Some(object) = state.object(host) else {
        return false;
    };
    !state.effects.iter().any(|fx| {
        matches!(
            fx.modifier,
            baylee_cards_dsl::Modifier::CantBeEnchantedExceptSource
        ) && fx.source != Some(aura)
            && crate::effects::applies_to(state, fx, object)
    })
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
        TargetSpec::AnyTarget
            | TargetSpec::AnyPlayer
            | TargetSpec::AnyOpponent
            | TargetSpec::OpponentOrObject(_)
    ) {
        return Vec::new();
    }
    let opponents_only = matches!(
        spec,
        TargetSpec::AnyOpponent | TargetSpec::OpponentOrObject(_)
    );
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

/// [`TargetSpec::CardInGraveyardBelowValue`]'s options: the graveyard cards
/// matching `filter` whose mana value is less than `limit`.
fn graveyard_options_below(
    filter: &'static baylee_cards_dsl::Filter,
    rel: baylee_cards_dsl::PlayerRel,
    limit: u32,
    state: &GameState,
    you: PlayerId,
    this: ObjectId,
) -> Vec<ObjectId> {
    graveyard_options(filter, rel, state, you, this)
        .into_iter()
        .filter(|id| {
            state
                .object(*id)
                .is_some_and(|o| o.characteristics().mana_value() < limit)
        })
        .collect()
}

/// The object options of the specs that are about one player's permanents.
fn objects_of_a_player(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    this: ObjectId,
) -> Vec<ObjectId> {
    match *spec {
        // The object half of "target opponent or [filter]"; the opponents
        // come from `target_player_options`, offered beside it.
        TargetSpec::OpponentOrObject(filter) => {
            target_options(&TargetSpec::Object(filter), state, you, this)
        }
        TargetSpec::ObjectControlledBy(filter, player) => {
            let mut all = target_options(&TargetSpec::Object(filter), state, you, this);
            all.retain(|id| state.object(*id).is_some_and(|o| o.controller == player));
            all
        }
        // Unbound, "that player" is nobody yet: the engine asks these only
        // after binding them to `ObjectControlledBy`.
        _ => Vec::new(),
    }
}

/// [`TargetSpec::ObjectOfEachOpponent`]'s options: every opponent's at once,
/// which each question narrows to one (`Engine::ask_next_opponent`).
fn opponents_objects(
    filter: &'static baylee_cards_dsl::Filter,
    state: &GameState,
    you: PlayerId,
    this: ObjectId,
) -> Vec<ObjectId> {
    let mut all = target_options(&TargetSpec::Object(filter), state, you, this);
    all.retain(|id| {
        state
            .object(*id)
            .is_some_and(|o| state.is_opponent(o.controller, you))
    });
    all
}

/// The X announced for `this`, where `cast_wizard` and an activation write
/// it; 0 for a source that is gone or announced none.
fn announced_x(state: &GameState, this: ObjectId) -> u32 {
    state.object(this).map_or(0, |o| o.x_value)
}

/// Whether `stat` is less than the source's projected power. A source with
/// no power, or none at all, bounds nothing in, and neither does an object
/// with no such number.
fn below_source_power(stat: Option<i16>, state: &GameState, this: ObjectId) -> bool {
    let bound = state.object(this).and_then(|o| o.characteristics().power);
    stat.zip(bound).is_some_and(|(stat, bound)| stat < bound)
}

/// Whether `filter` reads the X announced for its source
/// ([`Filter::CmcAtMostX`], [`Filter::CmcExactlyX`]).
#[must_use]
pub fn reads_announced_x(filter: &Filter) -> bool {
    match filter {
        Filter::CmcAtMostX | Filter::CmcExactlyX => true,
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(reads_announced_x),
        Filter::Not(f) => reads_announced_x(f),
        _ => false,
    }
}

/// Whether some announced X would let `obj` match `filter`: the question an
/// offer asks of a spell like Spell Blast before its X exists. X is announced
/// (CR 601.2b) before targets are chosen (CR 601.2c), and the offer comes
/// before both, so "is there a target" can only mean "is there one for some
/// X".
///
/// The X atoms answer yes and every other atom is [`matches`]. A negated X
/// atom answers yes as well without being asked, which offers a spell whose
/// cast may then find nothing — the direction a wizard can reverse (CR
/// 601.2), where hiding a castable spell is not. No card negates one.
#[must_use]
pub fn matches_for_some_x(
    filter: &Filter,
    state: &GameState,
    obj: &GameObject,
    you: PlayerId,
    this: ObjectId,
) -> bool {
    match filter {
        Filter::CmcAtMostX | Filter::CmcExactlyX => true,
        Filter::And(parts) => parts
            .iter()
            .all(|f| matches_for_some_x(f, state, obj, you, this)),
        Filter::Or(parts) => parts
            .iter()
            .any(|f| matches_for_some_x(f, state, obj, you, this)),
        Filter::Not(f) if reads_announced_x(f) => true,
        other => matches(other, state, obj, you, this),
    }
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
        TargetSpec::ObjectOfEachOpponent(filter) => opponents_objects(filter, state, you, this),
        TargetSpec::OpponentOrObject(_)
        | TargetSpec::ObjectControlledBy(..)
        | TargetSpec::ObjectOfFirstTargetsPlayer(_)
        | TargetSpec::ObjectOfEventPlayer(_) => objects_of_a_player(spec, state, you, this),
        TargetSpec::Spell(filter) => state
            .zones
            .list(ZoneLocation::Stack)
            .iter()
            .filter(|id| {
                // Uncounterable spells are targets like any other (#243):
                // it is the counter that asks, as it resolves
                // (`GameObject::can_be_countered`).
                state.object(**id).is_some_and(|o| {
                    o.kind == crate::object::ObjectKind::Spell
                        && matches(filter, state, o, you, this)
                })
            })
            .copied()
            .collect(),
        TargetSpec::CardInGraveyard(filter, rel) => {
            graveyard_options(filter, *rel, state, you, this)
        }
        TargetSpec::CardInGraveyardBelowEvent(..) => Vec::new(),
        TargetSpec::CardInGraveyardBelowValue(filter, rel, limit) => {
            graveyard_options_below(filter, *rel, *limit, state, you, this)
        }
        // "Target spell or permanent" (Venser, Shaper Savant; the laces):
        // the spells on the stack and the permanents on the battlefield, and
        // nothing else the stack holds. An activated or triggered ability
        // there is neither: abilities on the stack "aren't spells"
        // (CR 113.9), and a permanent is a card or token on the battlefield
        // (CR 110.1). Offering them put each Venser trigger on the menu of
        // the next one stacked over it: in l29 game 1930 thirty token
        // Vensers were asked over a menu that grew by one a question.
        TargetSpec::StackOrBattlefield(filter) => {
            let spells = state.zones.list(ZoneLocation::Stack).iter().filter(|id| {
                state
                    .object(**id)
                    .is_some_and(|o| o.kind == crate::object::ObjectKind::Spell)
            });
            let mut out: Vec<ObjectId> = spells
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
    // Protection (CR 702.16b) keeps out matching sources, and so does a
    // printed "can't be the target of" sentence; hexproof and shroud
    // (CR 702.11b/702.18b) keep out whole classes of chooser.
    options
        .into_iter()
        .filter(|id| {
            !protected_from(state, *id, this)
                && !untargetable_by_source(state, *id, this)
                && !untargetable_by(state, *id, you)
        })
        .collect()
}

/// The players a spell or ability on the stack chose for its first instance
/// of "target" under `spec`.
///
/// They ride in two fields. `target_players` is the set "any target" named,
/// and `chosen_player` is the one seat a player target named, but it is also
/// written by choices that are not targets at all. So the second is read
/// only for the three specs that can name a player, which is exactly the
/// set [`target_player_options`] answers for: folding it in for any other
/// spec would put a chosen player in front of an enumeration that never
/// offered them.
#[must_use]
pub fn targeted_players(obj: &GameObject, spec: &TargetSpec) -> baylee_core::ids::SeatSet {
    let mut players = baylee_core::ids::SeatSet::new();
    if matches!(
        spec,
        TargetSpec::AnyTarget
            | TargetSpec::AnyPlayer
            | TargetSpec::AnyOpponent
            | TargetSpec::OpponentOrObject(_)
    ) {
        players = obj.target_players;
        if let Some(player) = obj.chosen_player {
            players.insert(player);
        }
    }
    players
}

/// What a spell or ability on the stack may legally target under `spec` now:
/// the objects and the players.
///
/// It is the enumeration that offered the targets in the first place,
/// [`target_options`] and [`target_player_options`], asked with the same
/// `(you, this)` the cast wizard passes: the object's controller, and the
/// object itself or, for an ability, its source. CR 608.2b's re-check
/// (`Engine::instance_legality`) and a change of targets (CR 115.7,
/// `resolve::retarget`) both ask it, so what one of them calls legal the
/// other does too.
///
/// The object itself is never among them (CR 115.5).
#[must_use]
pub fn stack_target_options(
    state: &GameState,
    obj: &GameObject,
    spec: &TargetSpec,
) -> (Vec<ObjectId>, Vec<PlayerId>) {
    let you = obj.controller;
    let this = if obj.kind == crate::object::ObjectKind::AbilityOnStack {
        obj.ability.map_or(obj.id, |loc| loc.source)
    } else {
        obj.id
    };
    let mut objects = target_options(spec, state, you, this);
    objects.retain(|id| *id != obj.id);
    (objects, target_player_options(state, spec, you))
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

    /// "That player" of a step trigger is whoever's turn it is (CR 102.1),
    /// read from the state alone; the enchanted permanent's controller needs
    /// the source and is the resolution's to answer.
    #[test]
    fn the_active_player_is_whoever_s_turn_it_is() {
        let mut state = empty_state();
        state.turn.active = P1;
        assert_eq!(players(PlayerRel::ActivePlayer, &state, P0), Some(vec![P1]));
        state.turn.active = P0;
        assert_eq!(players(PlayerRel::ActivePlayer, &state, P1), Some(vec![P0]));
        assert_eq!(players(PlayerRel::ControllerOfAttached, &state, P0), None);

        let host = land(&mut state, P1, &[]);
        let aura = creature(&mut state, P0, KeywordSet::EMPTY);
        assert_eq!(
            controller_of_attached(&state, aura),
            None,
            "attached to nothing"
        );
        state.object_mut(aura).expect("just made").attached_to = Some(host);
        assert_eq!(
            controller_of_attached(&state, aura),
            Some(P1),
            "the host's controller, not the Aura's (CR 303.4e)"
        );
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
            origin: crate::effects::EffectOrigin::Resolution,
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

    /// "Activate only during combat" holds in each step of the combat phase
    /// (CR 506.1), whoever's turn it is, and in no other phase.
    #[test]
    fn during_combat_is_the_combat_phase_of_any_turn() {
        use crate::turn::{Phase, Step};
        let mut state = empty_state();
        let this = creature(&mut state, P0, KeywordSet::EMPTY);
        let during = |state: &GameState| condition_holds(state, P0, this, Condition::DuringCombat);
        for (phase, step, holds) in [
            (Phase::FirstMain, Step::Main, false),
            (Phase::Combat, Step::CombatBegin, true),
            (Phase::Combat, Step::DeclareBlockers, true),
            (Phase::Combat, Step::CombatEnd, true),
            (Phase::SecondMain, Step::Main, false),
            (Phase::Ending, Step::End, false),
        ] {
            state.turn.phase = phase;
            state.turn.step = step;
            assert_eq!(during(&state), holds, "{step:?}");
        }
        state.turn.phase = Phase::Combat;
        state.turn.step = Step::DeclareAttackers;
        state.turn.active = P1;
        assert!(during(&state), "the opponent's combat too");
    }

    /// "For each creature that died this turn" reads the turn's tally, which
    /// the move to a graveyard writes and the turn's end clears.
    #[test]
    fn the_turns_deaths_are_the_tally_the_moves_wrote() {
        let mut state = empty_state();
        let this = creature(&mut state, P0, KeywordSet::EMPTY);
        let died = Amount::CreaturesDiedThisTurn;
        assert_eq!(amount(&died, &state, P0, this, None), 0);
        for _ in 0..2 {
            let dying = creature(&mut state, P1, KeywordSet::EMPTY);
            state
                .move_object(
                    dying,
                    ZoneLocation::Graveyard(P1),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .expect("it moves");
        }
        assert_eq!(amount(&died, &state, P0, this, None), 2, "either player's");
    }

    /// "Toughness less than Stone Giant's power": compared with the
    /// source's projected power, strictly, and a source with no power
    /// bounds nothing in.
    #[test]
    fn a_comparison_with_the_sources_power_is_strict_and_reads_the_source() {
        let mut state = empty_state();
        let giant = creature(&mut state, P0, KeywordSet::EMPTY);
        let small = creature(&mut state, P0, KeywordSet::EMPTY);
        let land = land(&mut state, P0, &[]);
        {
            let b = state.object_mut(giant).expect("seated").base_mut();
            b.power = Some(3);
            b.toughness = Some(4);
        }
        {
            let b = state.object_mut(small).expect("seated").base_mut();
            b.power = Some(3);
            b.toughness = Some(2);
        }
        state.invalidate_projections();
        let ask = |state: &GameState, f: &Filter, id: ObjectId, source: ObjectId| {
            matches(f, state, state.object(id).expect("still here"), P0, source)
        };
        assert!(ask(
            &state,
            &Filter::ToughnessLessThanSourcePower,
            small,
            giant
        ));
        assert!(
            !ask(&state, &Filter::PowerLessThanSourcePower, small, giant),
            "3 is not less than 3"
        );
        assert!(
            !ask(&state, &Filter::ToughnessLessThanSourcePower, giant, giant),
            "the giant's own 4 is not less than its 3"
        );
        assert!(
            !ask(&state, &Filter::ToughnessLessThanSourcePower, small, land),
            "a source with no power bounds nothing in"
        );
        state.object_mut(giant).expect("seated").base_mut().power = Some(2);
        state.invalidate_projections();
        assert!(
            !ask(&state, &Filter::ToughnessLessThanSourcePower, small, giant),
            "the source's power as it is now"
        );
    }

    /// "If no creatures are on the battlefield" (Pestilence) counts both
    /// sides of the table, where "you control" counts one.
    #[test]
    fn a_battlefield_count_is_everybodys() {
        let mut state = empty_state();
        let theirs = creature(&mut state, P1, KeywordSet::EMPTY);

        let none = Condition::BattlefieldCountAtMost(&ANY_CREATURE, 0);
        let some = Condition::BattlefieldCount(&ANY_CREATURE, 1);
        assert!(!condition_holds(&state, P0, theirs, none), "theirs counts");
        assert!(condition_holds(&state, P0, theirs, some));
        assert!(
            condition_holds(
                &state,
                P0,
                theirs,
                Condition::ControlCountAtMost(&ANY_CREATURE, 0)
            ),
            "the control: P0 controls none of it"
        );
        let bare = empty_state();
        assert!(condition_holds(&bare, P0, theirs, none));
        assert!(!condition_holds(&bare, P0, theirs, some));
    }

    /// Blaze of Glory's "creature defending player controls": during the
    /// combat phase, a creature an opponent of the active player controls
    /// (CR 506.2, 802.2), whoever `you` is; outside it, nobody's. A
    /// teammate of the active player defends nothing.
    #[test]
    fn a_defending_players_creatures_are_the_active_players_opponents_in_combat() {
        static DEFENDING: Filter = Filter::ControlledByDefendingPlayer;
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::EMPTY);
        let theirs = creature(&mut state, P1, KeywordSet::EMPTY);
        let asks = |state: &GameState, id| {
            state
                .object(id)
                .is_some_and(|o| matches(&DEFENDING, state, o, P1, id))
        };
        state.turn.active = P0;
        state.turn.phase = crate::turn::Phase::FirstMain;
        assert!(!asks(&state, theirs), "no defending player outside combat");
        state.turn.phase = crate::turn::Phase::Combat;
        assert!(asks(&state, theirs), "whoever `you` is");
        assert!(!asks(&state, mine), "the attacking player defends nothing");
        state.turn.active = P1;
        assert!(asks(&state, mine) && !asks(&state, theirs));
        for player in &mut state.players {
            player.team = Some(1);
        }
        assert!(!asks(&state, mine), "a teammate is not attacked");
    }

    /// Karma's "Swamps they control" is the active player's, and Spell
    /// Blast's "mana value X" is the X its own source announced.
    #[test]
    fn the_active_players_permanents_and_a_mana_value_of_exactly_x() {
        static ACTIVES: Filter = Filter::ControlledByActivePlayer;
        static EXACTLY_X: Filter = Filter::CmcExactlyX;
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::EMPTY);
        let theirs = creature(&mut state, P1, KeywordSet::EMPTY);
        state.turn.active = P1;
        let asks = |state: &GameState, id, filter: &Filter| {
            state
                .object(id)
                .is_some_and(|o| matches(filter, state, o, P0, id))
        };
        assert!(asks(&state, theirs, &ACTIVES), "whoever `you` is");
        assert!(!asks(&state, mine, &ACTIVES));
        state.turn.active = P0;
        assert!(asks(&state, mine, &ACTIVES));

        let spell = creature(&mut state, P0, KeywordSet::EMPTY);
        let value = state
            .object(mine)
            .map_or(0, |o| o.characteristics().mana_value());
        let check = |state: &GameState| {
            state
                .object(mine)
                .is_some_and(|o| matches(&EXACTLY_X, state, o, P0, spell))
        };
        state.object_mut(spell).expect("here").x_value = value;
        assert!(check(&state));
        state.object_mut(spell).expect("here").x_value = value + 1;
        assert!(!check(&state), "at most X would have said yes");
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

    /// A leveler's `{LEVEL N1-N2}` band (CR 711.2a) is closed at both ends:
    /// Hexdrinker's "LEVEL 3-7" holds at three and at seven and at neither
    /// two nor eight.
    #[test]
    fn a_level_band_holds_at_both_ends_and_not_beyond_them() {
        let mut state = empty_state();
        let source = creature(&mut state, P0, KeywordSet::EMPTY);
        let kind = baylee_cards_dsl::CounterKind::Level;
        let band = Condition::CountersOnSelfBetween(kind, 3, 7);
        let mut held = Vec::new();
        for _ in 0..9 {
            held.push(condition_holds(&state, P0, source, band));
            state
                .object_mut(source)
                .expect("just made it")
                .counters
                .add(kind, 1);
        }
        assert_eq!(
            held,
            [false, false, false, true, true, true, true, true, false],
            "levels 0 to 8"
        );
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

    /// "Target spell or permanent" is the spells on the stack and the
    /// permanents on the battlefield. An ability on the stack is neither
    /// (CR 113.9, CR 110.1), and offering one let every Venser trigger aim
    /// at the triggers stacked under it until the menu outgrew a `u8` (l29
    /// game 1930). The spell is asserted beside it, so a fix that dropped
    /// the stack half altogether is red too.
    #[test]
    fn a_spell_or_permanent_is_never_an_ability_on_the_stack() {
        static SPELL_OR_PERMANENT: TargetSpec = TargetSpec::StackOrBattlefield(&Filter::Any);
        let mut state = empty_state();
        let permanent = creature(&mut state, P1, KeywordSet::EMPTY);
        let name = state.names.intern("Test Spell");
        let spell = state.create_bare(P1, ObjectKind::Spell, name, ZoneLocation::Stack);
        let name = state.names.intern("Test Ability");
        let ability = state.create_bare(P1, ObjectKind::AbilityOnStack, name, ZoneLocation::Stack);

        let options = target_options(&SPELL_OR_PERMANENT, &state, P0, permanent);
        assert!(
            options.contains(&spell),
            "a spell on the stack: {options:?}"
        );
        assert!(
            options.contains(&permanent),
            "a permanent on the battlefield: {options:?}"
        );
        assert!(
            !options.contains(&ability),
            "an ability on the stack is no spell: {options:?}"
        );
        assert_eq!(options.len(), 2, "and nothing else: {options:?}");
    }
}
