//! Target and choice options: the players, objects and stack items a spec may point at.

mod players;

pub use players::{
    first_target_players, target_player_options, target_player_options_from, targeted_players,
};

use super::protection::{protected_from_object, untargetable_by_source_object};
use super::{
    defending_player_of, live_context, matches_for_x, matches_with_context, players,
    protected_from, target_domain, target_pair, untargetable_by, untargetable_by_source,
};
use crate::object::GameObject;
use crate::state::GameState;
use crate::text_changes::RuleContext;
use crate::zone::ZoneLocation;
use baylee_cards_dsl::{Filter, PlayerRel, TargetSpec};
use baylee_core::ids::{ObjectId, PlayerId};

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
    context: RuleContext,
) -> Vec<ObjectId> {
    let rel = if rel == PlayerRel::Chosen {
        PlayerRel::EachPlayer
    } else {
        rel
    };
    let seats = if rel == PlayerRel::DefendingPlayer {
        defending_player_of(state, context.source, you)
            .into_iter()
            .collect()
    } else {
        let Some(seats) = players(rel, state, you) else {
            return Vec::new();
        };
        seats
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
                        .is_some_and(|o| matches_with_context(filter, state, o, you, context))
                })
                .copied(),
        );
    }
    out
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

fn named_graveyard_options(
    filter: &'static Filter,
    player: PlayerId,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
) -> Vec<ObjectId> {
    if state.has_left(player) {
        return Vec::new();
    }
    state
        .zones
        .list(ZoneLocation::Graveyard(player))
        .iter()
        .copied()
        .filter(|id| {
            state
                .object(*id)
                .is_some_and(|object| matches_with_context(filter, state, object, you, context))
        })
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
    context: RuleContext,
) -> Vec<ObjectId> {
    graveyard_options(filter, rel, state, you, context)
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
    context: RuleContext,
) -> Vec<ObjectId> {
    match *spec {
        // The object half of "target opponent or [filter]"; the opponents
        // come from `target_player_options`, offered beside it.
        TargetSpec::OpponentOrObject(filter) | TargetSpec::PlayerOrObject(filter) => {
            choice_options_with_context(&TargetSpec::Object(filter), state, you, context)
        }
        TargetSpec::ObjectControlledBy(filter, player) => {
            let mut all =
                choice_options_with_context(&TargetSpec::Object(filter), state, you, context);
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
    context: RuleContext,
) -> Vec<ObjectId> {
    let mut all = choice_options_with_context(&TargetSpec::Object(filter), state, you, context);
    all.retain(|id| {
        state
            .object(*id)
            .is_some_and(|o| state.is_opponent(o.controller, you))
    });
    all
}

/// Target options before announcing X (`None`) or with the announced value.
/// The widened enumeration still enforces protection and targeting rules.
pub(crate) fn target_options_with_x(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    x: Option<u32>,
) -> Vec<ObjectId> {
    let Some((widened, filter)) = crate::casting::x_bounded(*spec) else {
        return target_options_with_context(spec, state, you, context);
    };
    let mut options = target_options_with_context(&widened, state, you, context);
    options.retain(|id| {
        state
            .object(*id)
            .is_some_and(|obj| matches_for_x(filter, state, obj, you, context, x))
    });
    options
}

/// Enumerate one instance using the complete selected targeting domain.
pub(crate) fn target_options_in_domain(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    x: Option<u32>,
    wall_only: bool,
) -> Vec<ObjectId> {
    let mut options = target_options_with_x(spec, state, you, context, x);
    target_domain::retain(state, &mut options, wall_only);
    options
}

/// Legal target options for a [`TargetSpec`] (empty = cannot be chosen).
#[must_use]
pub fn target_options(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    this: ObjectId,
) -> Vec<ObjectId> {
    target_options_with_context(spec, state, you, live_context(state, this))
}

/// Legal target options using frozen ability text or a grant's original words.
#[must_use]
pub fn target_options_with_context(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
) -> Vec<ObjectId> {
    let this = context.source;
    // Protection (CR 702.16b) keeps out matching sources, and so does a
    // printed "can't be the target of" sentence; hexproof and shroud
    // (CR 702.11b/702.18b) keep out whole classes of chooser.
    let mut options = choice_options_with_context(spec, state, you, context)
        .into_iter()
        .filter(|id| {
            !protected_from(state, *id, this)
                && !untargetable_by_source(state, *id, this)
                && !untargetable_by(state, *id, you)
        })
        .collect();
    target_pair::retain_partners(state, spec, context, &mut options);
    options
}

/// Targeting with the characteristics of the chosen face and casting form.
pub(crate) fn spell_target_options_for(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    source: ObjectId,
    face: Option<(&baylee_cards_dsl::CardDef, usize)>,
    form: Option<crate::casting::SpellForm>,
    wall_only: bool,
) -> Vec<ObjectId> {
    spell_target_options_relative(spec, state, you, source, face, form, wall_only, None)
}

/// [`spell_target_options_for`] for a second instance of "target" whose
/// requirement names the first (`Filter::Reference` and its family, CR
/// 601.2c chooses them in order): `reference` is the first target.
#[allow(clippy::too_many_arguments)] // the face, the form and the domain are each the caller's
pub(crate) fn spell_target_options_relative(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    source: ObjectId,
    face: Option<(&baylee_cards_dsl::CardDef, usize)>,
    form: Option<crate::casting::SpellForm>,
    wall_only: bool,
    reference: Option<ObjectId>,
) -> Vec<ObjectId> {
    let Some(original) = state.object(source) else {
        return Vec::new();
    };
    let mut spell = original.clone();
    if let Some((def, face)) = face {
        let Some(printed) = def.faces.get(face) else {
            return Vec::new();
        };
        let name = state.names.find(printed.name).unwrap_or(original.base.name);
        spell.base =
            std::sync::Arc::new(crate::object::Characteristics::from_face(def, face, name));
        spell.cache.clear();
        spell.face_index = u8::try_from(face).unwrap_or(0);
    }
    if let Some(form) = form {
        spell = form.project(&spell);
    }
    spell.zone = crate::zone::Zone::Stack;
    spell.kind = crate::object::ObjectKind::Spell;
    spell.controller = you;
    spell.cache.clear();
    let projected = crate::layers::recompute(state, &spell);
    spell.base = std::sync::Arc::new(projected.characteristics);
    spell.controller = projected.controller;
    let mut context = live_context(state, source);
    context.reference = reference;
    let mut options = choice_options_with_context(spec, state, you, context)
        .into_iter()
        .filter(|id| {
            !protected_from_object(state, *id, &spell)
                && !untargetable_by_source_object(state, *id, &spell)
                && aura_target_permitted(state, &spell, *id)
                && !untargetable_by(state, *id, you)
        })
        .collect();
    target_domain::retain(state, &mut options, wall_only);
    target_pair::retain_partners(state, spec, context, &mut options);
    options
}

fn aura_target_permitted(state: &GameState, spell: &GameObject, host: ObjectId) -> bool {
    spell.kind != crate::object::ObjectKind::Spell
        || !spell
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::enchantment::AURA)
        || crate::permanent_guards::permits_new_enchantment(state, host, spell.id)
}

/// The objects `spec` describes, for a choice that does **not** target.
///
/// "Unless that object or player is identified by the word 'target' in the
/// text of that spell or ability, or the rule for that keyword ability,
/// it's not a target" (CR 115.10a). Protection, hexproof and shroud are
/// all about being targeted, so they keep nothing off this list: a Clone
/// may enter as a copy of a creature with shroud, since entering as a copy
/// is how it enters (CR 707.5) and not an ability that targets. The same
/// enumeration as [`target_options`], without its last filter.
#[must_use]
pub fn choice_options(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    this: ObjectId,
) -> Vec<ObjectId> {
    choice_options_with_context(spec, state, you, live_context(state, this))
}

/// Spells on the stack and permanents on the battlefield matching `filter`.
/// Abilities on the stack belong to neither zone-specific object category.
fn stack_or_battlefield_options(
    filter: &Filter,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
) -> Vec<ObjectId> {
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
                .is_some_and(|o| matches_with_context(filter, state, o, you, context))
        })
        .copied()
        .collect();
    out.sort();
    out.dedup();
    out
}

/// [`choice_options`] with frozen ability text or a grant's original words.
#[must_use]
pub fn choice_options_with_context(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
) -> Vec<ObjectId> {
    let this = context.source;
    match spec {
        TargetSpec::Object(filter) => state
            .battlefield_view()
            .iter()
            .filter(|id| {
                state
                    .object(**id)
                    .is_some_and(|o| matches_with_context(filter, state, o, you, context))
            })
            .copied()
            .collect(),
        TargetSpec::ObjectOfEachOpponent(filter) => opponents_objects(filter, state, you, context),
        TargetSpec::OpponentOrObject(_)
        | TargetSpec::PlayerOrObject(_)
        | TargetSpec::ObjectControlledBy(..)
        | TargetSpec::ObjectOfFirstTargetsPlayer(_)
        | TargetSpec::ObjectOfEventPlayer(_) => objects_of_a_player(spec, state, you, context),
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
                        && matches_with_context(filter, state, o, you, context)
                })
            })
            .copied()
            .collect(),
        TargetSpec::CardInGraveyard(filter, rel) => {
            graveyard_options(filter, *rel, state, you, context)
        }
        TargetSpec::ObjectOrCardInAnyGraveyard(filter) => {
            let mut out = graveyard_options(filter, PlayerRel::EachPlayer, state, you, context);
            out.extend(state.battlefield_view().iter().copied().filter(|id| {
                state
                    .object(*id)
                    .is_some_and(|o| matches_with_context(filter, state, o, you, context))
            }));
            out.sort();
            out.dedup();
            out
        }
        TargetSpec::CardInFirstTargetsGraveyard(_) | TargetSpec::CardInGraveyardBelowEvent(..) => {
            Vec::new()
        }
        TargetSpec::CardInPlayersGraveyard(filter, player) => {
            named_graveyard_options(filter, *player, state, you, context)
        }
        TargetSpec::CardInGraveyardBelowValue(filter, rel, limit) => {
            graveyard_options_below(filter, *rel, *limit, state, you, context)
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
            stack_or_battlefield_options(filter, state, you, context)
        }
        TargetSpec::ThisObject => vec![this],
        TargetSpec::AbilityOnStack(filter) => state
            .zones
            .list(ZoneLocation::Stack)
            .iter()
            .filter(|id| {
                state.object(**id).is_some_and(|o| {
                    o.kind == crate::object::ObjectKind::AbilityOnStack
                        && matches_with_context(filter, state, o, you, context)
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
                    ) && matches_with_context(filter, state, o, you, context)
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
        | TargetSpec::AnyOpponent
        | TargetSpec::PlayerWhoAttackedThisTurn
        | TargetSpec::OpponentWithMoreThanActivePlayer(_)
        | TargetSpec::OpponentWithMoreThanPlayer(..) => vec![],
    }
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
    stack_target_options_relative(state, obj, spec, None)
}

/// [`stack_target_options`] for a second instance of "target" whose
/// requirement names the first: `reference` is the first target.
#[must_use]
pub fn stack_target_options_relative(
    state: &GameState,
    obj: &GameObject,
    spec: &TargetSpec,
    reference: Option<baylee_core::ids::DamageSourceRef>,
) -> (Vec<ObjectId>, Vec<PlayerId>) {
    stack_target_options_impl(state, obj, spec, reference, false)
}

/// Candidate replacement targets, before joint restrictions are checked against
/// the complete prospective target list by the retarget continuation.
pub(crate) fn stack_retarget_options(
    state: &GameState,
    obj: &GameObject,
    spec: &TargetSpec,
) -> (Vec<ObjectId>, Vec<PlayerId>) {
    stack_target_options_impl(state, obj, spec, None, true)
}

fn stack_target_options_impl(
    state: &GameState,
    obj: &GameObject,
    spec: &TargetSpec,
    reference: Option<baylee_core::ids::DamageSourceRef>,
    retargeting: bool,
) -> (Vec<ObjectId>, Vec<PlayerId>) {
    let you = obj.controller;
    let this = if obj.kind == crate::object::ObjectKind::AbilityOnStack {
        obj.ability.map_or(obj.id, |loc| loc.source)
    } else {
        obj.id
    };
    let context = RuleContext {
        source: this,
        text: live_context(state, obj.id).text,
        resolved_name: None,
        reference: reference
            .map(|reference| reference.object)
            .or(obj.event_object),
    };
    let mut objects = target_pair::stack_options(state, obj, spec, context, reference, retargeting)
        .unwrap_or_else(|| target_options_with_x(spec, state, you, context, Some(obj.x_value)));
    target_domain::retain(state, &mut objects, target_domain::stack_walls(obj));
    objects.retain(|id| *id != obj.id && aura_target_permitted(state, obj, *id));
    (
        objects,
        target_player_options_from(state, spec, you, context),
    )
}
