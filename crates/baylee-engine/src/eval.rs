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

/// Hexproof (CR 702.11b) and shroud (CR 702.18b): does `object` refuse to
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
    keywords.contains(baylee_cards_dsl::KeywordSet::HEXPROOF) && obj.controller != you
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
    // Protection (CR 702.16c) keeps out matching sources; hexproof and
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
    use baylee_core::ids::CardIndex;
    use baylee_core::preset::{FormatId, GamePreset, HouseRules, SeatController, SeatSpec};
    use baylee_core::types::TypeSet;

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

    /// Who `chooser` may point a creature-targeting spell at.
    fn targets(state: &GameState, chooser: PlayerId, source: ObjectId) -> Vec<ObjectId> {
        target_options(&TargetSpec::Object(&ANY_CREATURE), state, chooser, source)
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

    /// Shroud (CR 702.18b) stops everyone, controller included — that is
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
}
