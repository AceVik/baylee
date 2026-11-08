//! What the rule groups read off a seat's view: values of objects, and the
//! damage the other side can visibly swing for. Everything here takes the view
//! and the printed rules of what the view shows, nothing else.

use baylee_cards_dsl::{AbilityDef, Filter, KeywordSet, Trigger};
use baylee_core::generated::subtypes::land as land_subtypes;
use baylee_core::ids::{ObjectId, PlayerId, SubtypeId};
use baylee_core::types::TypeSet;
use baylee_view::{ObjectStatus, PlayerView, PublicObject};

use super::super::abilities_of_battlefield_object;

/// The seat deciding.
pub(in crate::netplay4) fn actor(view: &PlayerView) -> PlayerId {
    view.decision_player.unwrap_or(view.seat)
}

/// The players against `me` still in the game.
pub(in crate::netplay4) fn opponents(
    view: &PlayerView,
    me: PlayerId,
) -> impl Iterator<Item = PlayerId> + '_ {
    view.seats
        .iter()
        .filter(move |seat| seat.player != me && seat.life > 0)
        .map(|seat| seat.player)
}

pub(in crate::netplay4) fn life(view: &PlayerView, player: PlayerId) -> i32 {
    view.seat(player).map_or(20, |seat| seat.life)
}

pub(in crate::netplay4) const fn has(object: &PublicObject, keyword: KeywordSet) -> bool {
    object.keywords & keyword.bits() != 0
}

pub(in crate::netplay4) fn is_creature(object: &PublicObject) -> bool {
    object.types.contains(TypeSet::CREATURE) && !object.status.contains(ObjectStatus::PHASED_OUT)
}

pub(in crate::netplay4) fn is_tapped(object: &PublicObject) -> bool {
    object.status.contains(ObjectStatus::TAPPED)
}

fn power(object: &PublicObject) -> i32 {
    i32::from(object.power.unwrap_or(0)).max(0)
}

/// Toughness left this turn.
pub(in crate::netplay4) fn toughness_left(object: &PublicObject) -> i32 {
    (i32::from(object.toughness.unwrap_or(1)) - i32::from(object.damage)).max(1)
}

/// Whether a permanent does something beyond its body: an activated ability
/// other than mana, or a trigger. A body that matters more than its size.
fn has_engine(object: &PublicObject) -> bool {
    abilities_of_battlefield_object(object).is_some_and(|abilities| {
        abilities.iter().any(|ability| match ability {
            AbilityDef::Activated { mana_ability, .. } => !mana_ability,
            AbilityDef::ActivatedConditional { .. }
            | AbilityDef::ModalActivated { .. }
            | AbilityDef::Triggered { .. }
            | AbilityDef::ModalTriggered { .. } => true,
            _ => false,
        })
    })
}

/// A rough worth of a permanent to whoever controls it. It only has to order
/// a token under a real creature, a body under an engine, and to add up
/// across a board: mass effects compare sums of it.
pub(in crate::netplay4) fn worth(object: &PublicObject) -> i32 {
    let mana_value = i32::try_from(object.mana_value).unwrap_or(0);
    let token = object.token.is_some() || object.card.is_none();
    if object.types.contains(TypeSet::LAND) && !object.types.contains(TypeSet::CREATURE) {
        return 2;
    }
    let body = if object.types.contains(TypeSet::CREATURE) {
        2 * power(object) + i32::from(object.toughness.unwrap_or(0)).max(0) + 1
    } else {
        2 + mana_value
    };
    let mut value = body;
    if !token {
        value += mana_value;
    }
    if has_engine(object) {
        value += 3;
    }
    if has(object, KeywordSet::FLYING)
        || has(object, KeywordSet::LIFELINK)
        || has(object, KeywordSet::DEATHTOUCH)
        || has(object, KeywordSet::DOUBLE_STRIKE)
    {
        value += 2;
    }
    value
}

/// A creature as a swing reads it: its own object, with a bonus a rule supposes.
#[derive(Clone, Copy)]
pub(in crate::netplay4) struct Hitter<'a> {
    pub(super) object: &'a PublicObject,
    pub(super) power: i32,
    pub(super) keywords: u128,
}

impl<'a> Hitter<'a> {
    pub(in crate::netplay4) fn of(object: &'a PublicObject) -> Self {
        Self {
            object,
            power: power(object),
            keywords: object.keywords,
        }
    }

    const fn has(&self, keyword: KeywordSet) -> bool {
        self.keywords & keyword.bits() != 0
    }
}

const LAND_WALKS: [(KeywordSet, SubtypeId); 5] = [
    (KeywordSet::PLAINSWALK, land_subtypes::PLAINS),
    (KeywordSet::ISLANDWALK, land_subtypes::ISLAND),
    (KeywordSet::SWAMPWALK, land_subtypes::SWAMP),
    (KeywordSet::MOUNTAINWALK, land_subtypes::MOUNTAIN),
    (KeywordSet::FORESTWALK, land_subtypes::FOREST),
];

/// Whether `defender` holds a land that `hitter` walks through (CR 702.14c).
pub(in crate::netplay4) fn walks_past(
    view: &PlayerView,
    defender: PlayerId,
    hitter: &Hitter<'_>,
) -> bool {
    LAND_WALKS.iter().any(|(walk, land)| {
        hitter.has(*walk)
            && view
                .battlefield_of(defender)
                .any(|object| object.subtypes.contains(*land))
    })
}

/// Whether `blocker` may block `hitter` as far as evasion goes (CR 509.1b).
pub(in crate::netplay4) fn may_block(hitter: &Hitter<'_>, blocker: &PublicObject) -> bool {
    use baylee_core::color::Color;
    if has(blocker, KeywordSet::CANT_BLOCK) {
        return false;
    }
    if hitter.has(KeywordSet::UNBLOCKABLE) {
        return false;
    }
    if hitter.has(KeywordSet::FLYING)
        && !has(blocker, KeywordSet::FLYING)
        && !has(blocker, KeywordSet::REACH)
    {
        return false;
    }
    if hitter.has(KeywordSet::SHADOW) != has(blocker, KeywordSet::SHADOW) {
        return false;
    }
    if hitter.has(KeywordSet::HORSEMANSHIP) && !has(blocker, KeywordSet::HORSEMANSHIP) {
        return false;
    }
    if hitter.has(KeywordSet::FEAR)
        && !blocker.types.contains(TypeSet::ARTIFACT)
        && !blocker.colors.contains(Color::Black)
    {
        return false;
    }
    if hitter.has(KeywordSet::INTIMIDATE)
        && !blocker.types.contains(TypeSet::ARTIFACT)
        && !hitter.object.colors.intersects(blocker.colors)
    {
        return false;
    }
    !(hitter.has(KeywordSet::SKULK) && power(blocker) > hitter.power)
}

/// What gets through if `defender` blocks as well as it can with `blockers`:
/// the biggest hitters first, each taking the blockers it needs, trample
/// carrying over. The visible swing, with no trick on either side.
pub(in crate::netplay4) fn damage_through(
    view: &PlayerView,
    defender: PlayerId,
    mut hitters: Vec<Hitter<'_>>,
    blockers: &[&PublicObject],
) -> i32 {
    hitters.sort_by_key(|hitter| std::cmp::Reverse(hitter.power));
    let mut used = vec![false; blockers.len()];
    let mut through = 0;
    for hitter in &hitters {
        let needed = if hitter.has(KeywordSet::MENACE) { 2 } else { 1 };
        let able: Vec<usize> = (0..blockers.len())
            .filter(|&index| {
                !used[index]
                    && !walks_past(view, defender, hitter)
                    && may_block(hitter, blockers[index])
            })
            .collect();
        if able.len() < needed {
            through += hitter.power;
            continue;
        }
        // The smallest that will do: the big ones stay for the next hitter.
        let mut chosen = able;
        chosen.sort_by_key(|&index| toughness_left(blockers[index]));
        let taken = &chosen[..needed];
        for &index in taken {
            used[index] = true;
        }
        if hitter.has(KeywordSet::TRAMPLE) {
            let soaked: i32 = taken
                .iter()
                .map(|&index| toughness_left(blockers[index]))
                .sum();
            through += (hitter.power - soaked).max(0);
        }
    }
    through
}

/// What `attacker`'s creatures can bring next turn: all of them untap.
pub(in crate::netplay4) fn hitters_of(view: &PlayerView, attacker: PlayerId) -> Vec<Hitter<'_>> {
    view.battlefield_of(attacker)
        .filter(|object| {
            is_creature(object) && !has(object, KeywordSet::DEFENDER) && power(object) > 0
        })
        .map(Hitter::of)
        .collect()
}

/// The creatures `player` could block with on the other side's coming turn if
/// those in `leaving` were tapped: nothing tapped now untaps before it.
pub(in crate::netplay4) fn blockers_of<'a>(
    view: &'a PlayerView,
    player: PlayerId,
    leaving: &[ObjectId],
) -> Vec<&'a PublicObject> {
    view.battlefield_of(player)
        .filter(|object| is_creature(object) && !leaving.contains(&object.id) && !is_tapped(object))
        .collect()
}

/// Whether `object` prints a trigger on hitting a player with damage: what a
/// block on it stops and a damage count does not see.
pub(super) fn hits_for_value(object: &PublicObject) -> bool {
    abilities_of_battlefield_object(object).is_some_and(|abilities| {
        abilities.iter().any(|ability| {
            let AbilityDef::Triggered { trigger, .. } = ability else {
                return false;
            };
            match trigger {
                Trigger::DealsCombatDamageToPlayer(source)
                | Trigger::DealsCombatDamageToOpponent(source)
                | Trigger::DealsDamageToOpponent(source)
                | Trigger::DealsDamageToPlayer { source, .. } => matches!(*source, Filter::This),
                _ => false,
            }
        })
    })
}
