//! Evaluation of DSL data: filters, amounts, target options.
//!
//! All evaluation is pure read access to [`GameState`]; `you` is the
//! ability/spell controller, `this` its source object.

use crate::object::{GameObject, Status};
use crate::state::GameState;
use crate::zone::ZoneLocation;
use baylee_cards_dsl::{Amount, Filter, PlayerRel, TargetSpec, ZoneSel};
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
        Filter::IsToken => obj.card.is_none(),
        Filter::ControlledByYou => obj.controller == you,
        Filter::ControlledByOpponent => obj.controller != you,
        Filter::OwnedByYou => obj.owner == you,
        Filter::Tapped => obj.status.contains(Status::TAPPED),
        Filter::Untapped => !obj.status.contains(Status::TAPPED),
        Filter::Attacking => state
            .combat
            .attackers
            .iter()
            .any(|info| info.creature == obj.id),
        Filter::MatchesChosenTypeOfSource => state
            .object(this)
            .and_then(|src| src.chosen_subtype)
            .is_some_and(|s| chars.subtypes.contains(s)),
        Filter::AttachedToBySource => state
            .object(this)
            .and_then(|src| src.attached_to)
            .is_some_and(|attached| attached == obj.id),
        Filter::SharesSubtypeWithCommander => {
            // Eight `AND`s per commander, not one probe per subtype id.
            let obj_subs = chars.subtypes;
            state
                .zones
                .list(ZoneLocation::Command(you))
                .iter()
                .filter_map(|id| state.object(*id))
                .any(|commander| obj_subs.intersects(commander.characteristics().subtypes))
        }
        Filter::HasKeyword(k) => chars.keywords.contains(*k),
        Filter::CmcAtMost(n) => chars.mana_cost.cmc() <= *n,
        Filter::CmcAtLeast(n) => chars.mana_cost.cmc() >= *n,
        Filter::ToughnessAtMost(n) => chars.toughness.is_some_and(|t| t <= *n),
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

/// Resolves a relative player reference to concrete players.
#[must_use]
pub fn players(rel: PlayerRel, state: &GameState, you: PlayerId) -> Vec<PlayerId> {
    match rel {
        PlayerRel::You => vec![you],
        PlayerRel::Opponent | PlayerRel::EachOpponent => state
            .players
            .iter()
            .filter(|p| p.id != you && !p.has_lost)
            .map(|p| p.id)
            .collect(),
        PlayerRel::EachPlayer => state
            .players
            .iter()
            .filter(|p| !p.has_lost)
            .map(|p| p.id)
            .collect(),
        PlayerRel::ControllerOfTarget | PlayerRel::Chosen => {
            vec![] // resolved in resolve.rs (needs the target/player context)
        }
    }
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
        let applies = match &fx.filter {
            crate::effects::EffectFilter::ObjectIs(id) => *id == object,
            crate::effects::EffectFilter::Dsl(filter) => matches(
                filter,
                state,
                obj,
                fx.controller,
                fx.source.unwrap_or(object),
            ),
        };
        applies && matches(f, state, src, fx.controller, fx.source.unwrap_or(source))
    })
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
            let mut out = Vec::new();
            for player in players(*rel, state, you) {
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
        // EventObject is implicit (no player choice); player targeting
        // resolves via ChoosePlayer in the casting wizard.
        TargetSpec::EventObject | TargetSpec::Player(_) | TargetSpec::AnyPlayer => vec![],
    };
    // Protection (CR 702.16c): a protected object can't be targeted by a
    // matching source.
    options
        .into_iter()
        .filter(|id| !protected_from(state, *id, this))
        .collect()
}
