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
        Filter::ControlledByOpponent => state.is_opponent(obj.controller, you),
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
            .filter(|p| state.is_opponent(p.id, you) && !p.has_lost)
            .map(|p| p.id)
            .collect(),
        PlayerRel::EachPlayer => state
            .players
            .iter()
            .filter(|p| !p.has_lost)
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
            if p.has_lost {
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
}
