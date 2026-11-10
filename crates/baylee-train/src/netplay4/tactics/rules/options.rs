//! Spells and abilities a group takes off the menu at priority, because they
//! do nothing or help only the other side. Each check reads the effect's
//! structure against what the view shows.

use baylee_cards_dsl::{
    AbilityDef, Amount, Effect, Filter, Modifier, PlayerRel, TargetReq, TargetSpec,
};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::types::{SupertypeSet, TargetSide, TypeSet};
use baylee_engine::target_side::effects_side;
use baylee_view::{CardIdentity, PlayerView, PublicObject};

use super::super::{
    abilities_of_battlefield_object, is_ally, is_hostile_object_effects, is_opponent,
    lookup_card_identity, lookup_spell_cost,
};
use super::read::{
    Hitter, actor, blockers_of, damage_through, hitters_of, is_creature, is_tapped, life,
    opponents, toughness_left, worth,
};
use super::{RuleGroup, TacticRules4};
use crate::features4::{public_object_matches_basic_filter, untapped_mana_production};

/// What one ability or mode of a card does, and what it asks to target.
type Part = (&'static [Effect], Option<TargetReq>);

/// The modes or the single body of a spell.
fn spell_parts(abilities: &'static [AbilityDef]) -> Vec<Part> {
    let mut parts = Vec::new();
    for ability in abilities {
        match ability {
            AbilityDef::Spell {
                effects, targets, ..
            } => parts.push((*effects, *targets)),
            AbilityDef::ModalSpell { modes, .. } => {
                parts.extend(modes.iter().map(|mode| (mode.effects, mode.targets)));
            }
            _ => {}
        }
    }
    parts
}

/// The modes or the single body of the activated ability at `index`.
fn ability_parts(ability: &'static AbilityDef) -> Vec<Part> {
    match ability {
        AbilityDef::Activated {
            effects, targets, ..
        }
        | AbilityDef::ActivatedConditional {
            effects, targets, ..
        } => vec![(*effects, *targets)],
        AbilityDef::ModalActivated { modes, .. } => modes
            .iter()
            .map(|mode| (mode.effects, mode.targets))
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether an opponent has anything `spec` can name that also matches `gate`.
fn gate_can_open(view: &PlayerView, me: PlayerId, spec: Option<TargetSpec>, gate: &Filter) -> bool {
    let theirs = |object: &&PublicObject| {
        is_opponent(me, object.controller) && public_object_matches_basic_filter(me, object, gate)
    };
    match spec {
        Some(TargetSpec::Spell(filter)) => view
            .stack
            .iter()
            .filter(theirs)
            .any(|object| public_object_matches_basic_filter(me, object, filter)),
        Some(TargetSpec::Object(filter)) => view
            .battlefield
            .iter()
            .filter(theirs)
            .any(|object| public_object_matches_basic_filter(me, object, filter)),
        // Not a shape read here: it may well open.
        _ => true,
    }
}

/// How many `filter` matches among what `player` controls.
fn count_of(view: &PlayerView, me: PlayerId, player: PlayerId, filter: &Filter) -> usize {
    view.battlefield_of(player)
        .filter(|object| public_object_matches_basic_filter(me, object, filter))
        .count()
}

/// Whether `amount`, dealt to the players of `to`, comes to nothing.
fn amount_is_nothing(view: &PlayerView, me: PlayerId, amount: Amount, to: PlayerRel) -> bool {
    match amount {
        Amount::Fixed(0) => true,
        Amount::ControlledCount {
            who: PlayerRel::Recipient,
            filter,
        } if matches!(to, PlayerRel::EachOpponent | PlayerRel::Opponent) => {
            opponents(view, me).all(|player| count_of(view, me, player, filter) == 0)
        }
        Amount::ControlledCount {
            who: PlayerRel::You,
            filter,
        } => count_of(view, me, me, filter) == 0,
        _ => false,
    }
}

/// Whether one effect would change nothing as the board stands.
fn effect_is_void(
    view: &PlayerView,
    me: PlayerId,
    effect: &Effect,
    targets: Option<TargetReq>,
) -> bool {
    match effect {
        Effect::IfTargetMatches { filter, .. } => {
            !gate_can_open(view, me, targets.map(|req| req.spec), filter)
        }
        Effect::DealDamage {
            amount,
            target: TargetSpec::Player(to),
        } => amount_is_nothing(view, me, *amount, *to),
        _ => false,
    }
}

/// Whether an effect that hurts what it names must name something of the seat's:
/// nothing of theirs answers to its target.
fn hurts_only_me(view: &PlayerView, me: PlayerId, part: &Part) -> bool {
    let Some(req) = part.1 else { return false };
    let TargetSpec::Object(filter) = req.spec else {
        return false;
    };
    req.min >= 1
        && effects_side(part.0, 1) == Some(TargetSide::Harmful)
        && !view.battlefield.iter().any(|object| {
            is_opponent(me, object.controller)
                && public_object_matches_basic_filter(me, object, filter)
        })
}

/// Whether everything of theirs this kill could name is already named by a kill
/// of the seat's on the stack.
fn all_doomed(view: &PlayerView, me: PlayerId, part: &Part) -> bool {
    let doomed = super::targets::doomed(view, me);
    if doomed.is_empty() || !is_hostile_object_effects(part.0) {
        return false;
    }
    let Some(TargetSpec::Object(filter)) = part.1.map(|req| req.spec) else {
        return false;
    };
    !view.battlefield.iter().any(|object| {
        is_opponent(me, object.controller)
            && !doomed.contains(&object.id)
            && public_object_matches_basic_filter(me, object, filter)
    })
}

fn part_is_void(view: &PlayerView, me: PlayerId, part: &Part) -> bool {
    !part.0.is_empty()
        && (part
            .0
            .iter()
            .all(|effect| effect_is_void(view, me, effect, part.1))
            || all_doomed(view, me, part))
}

/// What a mass effect sweeps, for the ones that hit both sides alike, and the
/// damage it deals, if that is how it sweeps.
fn swept(effect: &Effect, x: i32) -> Option<(&'static Filter, Option<i32>)> {
    match effect {
        Effect::DestroyAll { filter, .. }
        | Effect::DestroyAllThenForEachDestroyed { filter, .. }
        | Effect::ExileAll { filter, .. }
        | Effect::SacrificeAll { filter }
        | Effect::ReturnAllToHand {
            filter,
            opponents_only: false,
        } => Some((*filter, None)),
        Effect::DealDamageEach { amount, filter } => Some((*filter, damage_of(*amount, x))),
        _ => None,
    }
}

/// A damage amount this reading can count, with X at `x`.
fn damage_of(amount: Amount, x: i32) -> Option<i32> {
    match amount {
        Amount::Fixed(damage) => i32::try_from(damage).ok(),
        Amount::X => Some(x),
        _ => None,
    }
}

/// What the swept filter takes from each side, in worth: `(mine, theirs)`. A
/// sweep of damage takes what that much damage kills.
fn sweep_toll(view: &PlayerView, me: PlayerId, filter: &Filter, damage: Option<i32>) -> (i32, i32) {
    let mut toll = (0, 0);
    for object in &view.battlefield {
        let survives = damage.is_some_and(|amount| toughness_left(object) > amount);
        if !survives && public_object_matches_basic_filter(me, object, filter) {
            if is_ally(me, object.controller) {
                toll.0 += worth(object);
            } else {
                toll.1 += worth(object);
            }
        }
    }
    toll
}

/// Whether the swing the other side has now would kill `me` or an allied seat.
fn facing_lethal(view: &PlayerView, me: PlayerId) -> bool {
    view.seats
        .iter()
        .filter(|s| is_ally(me, s.player) && s.life > 0)
        .any(|ally_seat| {
            let ally = ally_seat.player;
            let blockers = blockers_of(view, ally, &[]);
            opponents(view, me)
                .map(|player| damage_through(view, ally, hitters_of(view, player), &blockers))
                .sum::<i32>()
                >= life(view, ally)
        })
}

/// A sweep that costs me more than it costs them, or hits nothing of theirs,
/// unless it is what stands between me and dying.
fn sweep_backfires(view: &PlayerView, me: PlayerId, part: &Part, x: i32) -> bool {
    let Some((filter, damage)) = part.0.iter().find_map(|effect| swept(effect, x)) else {
        return false;
    };
    let (mine, theirs) = sweep_toll(view, me, filter, damage);
    // Lands alone are a plan the toll cannot read: Armageddon wins from ahead on
    // the board at a cost in lands.
    let lands_only = view
        .battlefield
        .iter()
        .filter(|object| public_object_matches_basic_filter(me, object, filter))
        .all(|object| object.types.contains(TypeSet::LAND));
    theirs == 0 || (mine > theirs && !lands_only && !facing_lethal(view, me))
}

/// Burn with nothing of theirs to kill and no game to close: their life is more
/// than this and two swings of the seat's creatures. Held, it answers the creature
/// that comes; thrown, it is one card for a few points of a healthy life total.
fn burn_is_idle(view: &PlayerView, me: PlayerId, part: &Part, x: i32) -> bool {
    let [
        Effect::DealDamage {
            amount,
            target: TargetSpec::AnyTarget,
        },
    ] = part.0
    else {
        return false;
    };
    let Some(damage) = damage_of(*amount, x) else {
        return false;
    };
    let reaches = view.battlefield.iter().any(|object| {
        is_opponent(me, object.controller)
            && ((is_creature(object) && toughness_left(object) <= damage)
                || object.types.contains(TypeSet::PLANESWALKER))
    });
    let swing: i32 = hitters_of(view, me).iter().map(|hitter| hitter.power).sum();
    !reaches && opponents(view, me).all(|player| life(view, player) > damage + 2 * swing)
}

/// What X can be for the spell `id`, on all the mana the seat can reach.
fn reachable_x(view: &PlayerView, me: PlayerId, id: ObjectId) -> i32 {
    let pool = view
        .seat(me)
        .map_or(0, |seat| u32::try_from(seat.mana_pool.total()).unwrap_or(0));
    let mana = pool.saturating_add(untapped_mana_production(view, me));
    let base = lookup_spell_cost(view, id)
        .map_or(0, |(cost, tax)| cost.with_x(0).cmc().saturating_add(tax));
    i32::try_from(mana.saturating_sub(base)).unwrap_or(0)
}

/// A lord printed without "you control" lifts their team too. Reads the swing
/// they would have with it on the table against the swing they have now: a
/// card the mistake analysis names is Goblin King into eight Goblins.
fn lifts_their_team(view: &PlayerView, me: PlayerId, abilities: &'static [AbilityDef]) -> bool {
    let (mut power, mut keywords, mut filter) = (0_i32, 0_u128, None);
    for ability in abilities {
        if let AbilityDef::Static(statics) = ability {
            match statics.modifier {
                Modifier::ModifyPT(lift, _) => {
                    power += i32::from(lift);
                    filter = Some(&statics.filter);
                }
                Modifier::AddKeyword(words) => {
                    keywords |= words.bits();
                    filter = Some(&statics.filter);
                }
                _ => {}
            }
        }
    }
    let Some(filter) = filter else { return false };
    if matches!(filter, Filter::This) {
        return false;
    }
    let lifted = |object: &PublicObject| {
        is_creature(object) && public_object_matches_basic_filter(me, object, filter)
    };
    let mine = view
        .battlefield
        .iter()
        .filter(|object| is_ally(me, object.controller) && lifted(object))
        .count();
    let blockers = blockers_of(view, me, &[]);
    let (mut before, mut after, mut theirs) = (0, 0, 0);
    for player in opponents(view, me) {
        let hitters = hitters_of(view, player);
        theirs += hitters
            .iter()
            .filter(|hitter| lifted(hitter.object))
            .count();
        let raised: Vec<Hitter<'_>> = hitters
            .iter()
            .map(|hitter| {
                if lifted(hitter.object) {
                    Hitter {
                        power: hitter.power + power,
                        keywords: hitter.keywords | keywords,
                        ..*hitter
                    }
                } else {
                    *hitter
                }
            })
            .collect();
        before += damage_through(view, me, hitters, &blockers);
        after += damage_through(view, me, raised, &blockers);
    }
    let gift = after - before;
    theirs > mine && (gift >= 4 || (after >= life(view, me) && before < life(view, me)))
}

/// A legend the seat already controls: one of the two goes at once (CR 704.5j).
fn is_second_legend(view: &PlayerView, me: PlayerId, identity: CardIdentity) -> bool {
    let legendary = baylee_cards::by_index(identity.index)
        .and_then(|card| card.faces.get(usize::from(identity.face)))
        .is_some_and(|face| face.supertypes.contains(SupertypeSet::LEGENDARY));
    legendary
        && view
            .battlefield_of(me)
            .any(|object| object.card.is_some_and(|card| card.index == identity.index))
}

/// An ability that only makes its own land a creature until end of turn does
/// nothing on a land already one, or one too tapped to attack or block.
fn animates_for_nothing(object: &PublicObject, effects: &[Effect]) -> bool {
    let animates = effects.iter().any(|effect| {
        matches!(effect, Effect::CreateContinuousEffect { filter: Filter::This, modifier: Modifier::AddType(types), .. }
            if types.contains(TypeSet::CREATURE))
    });
    animates && (is_creature(object) || is_tapped(object))
}

impl TacticRules4 {
    /// The group that takes the spell `id` off the menu, if one does.
    pub(super) fn strike_spell(&self, view: &PlayerView, id: ObjectId) -> Option<RuleGroup> {
        let me = actor(view);
        let identity = lookup_card_identity(view, id)?;
        let card = baylee_cards::by_index(identity.index)?;
        let abilities = card.abilities_for_face(usize::from(identity.face));
        let parts = spell_parts(abilities);
        if self.on(RuleGroup::Void)
            && ((!parts.is_empty() && parts.iter().all(|part| part_is_void(view, me, part)))
                || is_second_legend(view, me, identity))
        {
            return Some(RuleGroup::Void);
        }
        if self.on(RuleGroup::Side)
            && !parts.is_empty()
            && parts.iter().all(|part| hurts_only_me(view, me, part))
        {
            return Some(RuleGroup::Side);
        }
        let x = reachable_x(view, me, id);
        if self.on(RuleGroup::Mass)
            && !parts.is_empty()
            && parts.iter().all(|part| sweep_backfires(view, me, part, x))
        {
            return Some(RuleGroup::Mass);
        }
        if self.on(RuleGroup::Burn)
            && !parts.is_empty()
            && parts.iter().all(|part| burn_is_idle(view, me, part, x))
        {
            return Some(RuleGroup::Burn);
        }
        if self.on(RuleGroup::Gift) && lifts_their_team(view, me, abilities) {
            return Some(RuleGroup::Gift);
        }
        None
    }

    /// The group that takes this activation off the menu, if one does.
    pub(super) fn strike_ability(
        &self,
        view: &PlayerView,
        source: ObjectId,
        index: u32,
    ) -> Option<RuleGroup> {
        let me = actor(view);
        let object = view.object(source)?;
        let ability = abilities_of_battlefield_object(object)?.get(index as usize)?;
        let parts = ability_parts(ability);
        if parts.is_empty() {
            return None;
        }
        if self.on(RuleGroup::Void)
            && (parts.iter().all(|part| part_is_void(view, me, part))
                || parts
                    .iter()
                    .all(|part| animates_for_nothing(object, part.0)))
        {
            return Some(RuleGroup::Void);
        }
        if self.on(RuleGroup::Side) && parts.iter().all(|part| hurts_only_me(view, me, part)) {
            return Some(RuleGroup::Side);
        }
        if self.on(RuleGroup::Mass) && parts.iter().all(|part| sweep_backfires(view, me, part, 0)) {
            return Some(RuleGroup::Mass);
        }
        if self.on(RuleGroup::Ping)
            && super::ping::shot(view, source, index)
                .is_some_and(|damage| super::ping::shot_is_wasted(view, source, damage))
        {
            return Some(RuleGroup::Ping);
        }
        None
    }
}
