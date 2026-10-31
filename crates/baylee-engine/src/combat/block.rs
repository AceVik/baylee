//! Declaring blockers: who may block what, the offer, the rules effects add (CR 509.1), and recording a block.

use super::damage::has_keyword;
use super::{historical_defending_player, landwalk_removed};
use crate::object::{GameObject, Status};
use crate::state::GameState;
use baylee_cards_dsl::KeywordSet as K;
use baylee_core::color::Color;
use baylee_core::generated::subtypes::land;
use baylee_core::ids::{ObjectId, PlayerId, SubtypeId};
use baylee_core::types::TypeSet;

/// Whether `b` could block anything at all for `defending`: the half of
/// [`can_block`] that asks only about the blocker.
fn ready_to_block(b: &GameObject, defending: PlayerId) -> bool {
    b.zone == crate::zone::Zone::Battlefield
        && b.controller == defending
        && b.characteristics().types.contains(TypeSet::CREATURE)
        && !b.characteristics().types.contains(TypeSet::BATTLE)
        && !b.status.contains(Status::TAPPED)
        && !b.status.contains(Status::PHASED_OUT)
}

/// The permanents `defending` could declare as blockers at all, in
/// battlefield order: every one [`can_block`] might answer `true` for,
/// whatever the attacker.
///
/// The declare-blockers offer asks [`can_block`] of every candidate against
/// every attacker, so its candidates are worked out once, here, and not by
/// walking the whole battlefield per attacker. Over the battlefield that
/// step cost permanents × attackers — 168,000 permanents against 33,600
/// attackers took 18 s in one self-play game (r001 game 431) for a defender
/// with a handful of creatures — where over these it costs the defender's
/// untapped creatures × attackers.
#[must_use]
pub fn ready_blockers(state: &GameState, defending: PlayerId) -> Vec<ObjectId> {
    state
        .battlefield_seen()
        .filter(|id| {
            state
                .object(*id)
                .is_some_and(|b| ready_to_block(b, defending))
        })
        .collect()
}

/// Each basic landwalk and the land type it names (CR 702.14c, 305.6).
pub(super) const LANDWALKS: [(K, SubtypeId); 5] = [
    (K::PLAINSWALK, land::PLAINS),
    (K::ISLANDWALK, land::ISLAND),
    (K::SWAMPWALK, land::SWAMP),
    (K::MOUNTAINWALK, land::MOUNTAIN),
    (K::FORESTWALK, land::FOREST),
];

/// Whether `player` controls a land with land type `subtype`, as the
/// battlefield is now: projected types, phased-out permanents not there
/// at all (CR 702.26b).
#[must_use]
pub fn controls_land_of_type(state: &GameState, player: PlayerId, subtype: SubtypeId) -> bool {
    state.battlefield_seen().any(|id| {
        state.object(id).is_some_and(|land| {
            let chars = land.characteristics();
            land.controller == player
                && chars.types.contains(TypeSet::LAND)
                && chars.subtypes.contains(subtype)
        })
    })
}

/// Landwalk is read against this defender's lands; an as-though permission
/// ignores only its named walks while deciding whether the creature can block.
fn basic_landwalk_prevents_blocking(
    state: &GameState,
    attacker: &GameObject,
    defending: PlayerId,
) -> bool {
    let keywords = attacker.characteristics().keywords;
    let mut walks = LANDWALKS.iter().fold(K::EMPTY, |all, (walk, land)| {
        if keywords.contains(*walk) && controls_land_of_type(state, defending, *land) {
            all.union(*walk)
        } else {
            all
        }
    });
    if walks.is_empty() {
        return false;
    }
    for effect in state.effects.iter() {
        if let baylee_cards_dsl::Modifier::BlockableWithoutLandwalk(ignored) = effect.modifier
            && crate::effects::applies_to(state, effect, attacker)
        {
            walks = walks.difference(ignored);
        }
    }
    !walks.is_empty()
}

/// All landwalk restrictions, including chosen subtypes and as-though permissions.
fn landwalk_prevents_blocking(state: &GameState, a: &GameObject, defending: PlayerId) -> bool {
    let attacker = a.id;
    let ignore_landwalk = state.effects.iter().any(|effect| {
        matches!(
            effect.modifier,
            baylee_cards_dsl::Modifier::BlockableWithoutAnyLandwalk
        ) && crate::effects::applies_to(state, effect, a)
    });
    // Landwalk (CR 702.14c): unblockable while the *defending* player
    // controls a land of that type — that player's lands and nobody
    // else's, read as they are now (a land an effect made an Island is
    // one).
    if !ignore_landwalk && basic_landwalk_prevents_blocking(state, a, defending) {
        return true;
    }
    // The rest of the family (desertwalk, CR 702.14c): a land the effect's
    // filter describes, among the defending player's.
    !ignore_landwalk
        && state.effects.iter().any(|fx| {
            if let baylee_cards_dsl::Modifier::LandwalkOfSubtype { subtype, lands } = fx.modifier {
                return crate::effects::applies_to(state, fx, a)
                    && !landwalk_removed(state, a, fx.timestamp)
                    && state.battlefield_seen().any(|id| {
                        state.object(id).is_some_and(|land| {
                            land.controller == defending
                                && land.characteristics().types.contains(TypeSet::LAND)
                                && land.characteristics().subtypes.contains(subtype)
                                && crate::eval::matches_with_context(
                                    lands,
                                    state,
                                    land,
                                    fx.controller,
                                    crate::text_changes::RuleContext {
                                        source: fx.source.unwrap_or(attacker),
                                        text: state.effect_text(fx),
                                        resolved_name: None,
                                        reference: None,
                                    },
                                )
                        })
                    });
            }
            let baylee_cards_dsl::Modifier::LandwalkMatching(filter) = fx.modifier else {
                return false;
            };
            crate::effects::applies_to(state, fx, a)
                && !landwalk_removed(state, a, fx.timestamp)
                && state.battlefield_seen().any(|id| {
                    state.object(id).is_some_and(|land| {
                        land.controller == defending
                            && land.characteristics().types.contains(TypeSet::LAND)
                            && crate::eval::matches(
                                filter,
                                state,
                                land,
                                fx.controller,
                                fx.source.unwrap_or(attacker),
                            )
                    })
                })
        })
}

/// Whether `defending` may block `attacker` with `blocker` (keyword
/// restrictions included): only a creature attacking that player or one of
/// their planeswalkers (CR 509.1a, 802.4a).
#[must_use]
pub fn can_block(
    state: &GameState,
    defending: PlayerId,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    let (Some(b), Some(a)) = (state.object(blocker), state.object(attacker)) else {
        return false;
    };
    // CR 509.1a, and at a table of several defending players 802.4a: a
    // defending player blocks only "creatures attacking that player, a
    // planeswalker that player controls". A creature not declared as an
    // attacker is asked only about its keywords (the pair's own half).
    if state
        .combat
        .defender_of(attacker)
        .is_some_and(|_| historical_defending_player(state, attacker) != Some(defending))
    {
        return false;
    }
    if !ready_to_block(b, defending)
        || a.zone != crate::zone::Zone::Battlefield
        || a.status.contains(Status::PHASED_OUT)
        || !a.characteristics().types.contains(TypeSet::CREATURE)
    {
        return false;
    }
    let kw =
        |o: &GameObject, k: baylee_cards_dsl::KeywordSet| o.characteristics().keywords.contains(k);
    // Shadow restricts both sides of the pair. It does not replace flying
    // or any other evasion restriction (Dauthi Voidwalker's release notes).
    if kw(a, K::SHADOW) != kw(b, K::SHADOW) {
        return false;
    }
    // Flying can only be blocked by flying/reach (CR 702.9).
    if kw(a, K::FLYING) && !kw(b, K::FLYING) && !kw(b, K::REACH) {
        return false;
    }
    // Raging River's piles (a restriction, CR 509.1b): only a creature with
    // flying or one in the pile chosen for this attacker.
    if !state
        .combat
        .pile_limits_allow(attacker, blocker, kw(b, K::FLYING))
    {
        return false;
    }
    // Fear (CR 702.36b): only artifact creatures and/or black creatures.
    if kw(a, K::FEAR) {
        let blocker = b.characteristics();
        if !blocker.types.contains(TypeSet::ARTIFACT) && !blocker.colors.contains(Color::Black) {
            return false;
        }
    }
    // Intimidate (CR 702.13b): any shared color works; colorless is not
    // a color and therefore only an artifact creature can block it.
    if kw(a, K::INTIMIDATE) {
        let blocker = b.characteristics();
        if !blocker.types.contains(TypeSet::ARTIFACT)
            && !blocker.colors.intersects(a.characteristics().colors)
        {
            return false;
        }
    }
    if landwalk_prevents_blocking(state, a, defending) {
        return false;
    }
    // Menace is deliberately *not* asked here. CR 702.111b restricts the
    // whole declaration and CR 509.1b is where that is checked, so a
    // function that sees one pair cannot answer it — and asking it here
    // answered "no" every time, because both callers ask before anything is
    // recorded: `progress_step` while it builds the offer, and
    // `declare_blockers` in a per-pair loop that runs to completion before
    // the first `declare_block`. `state.combat.blockers_of(attacker)` was
    // therefore always empty, which made menace read as plain unblockable
    // (#156). The count lives in `Engine::declare_blockers`, and
    // [`menace_satisfiable`] is what keeps the offer from naming a pairing
    // that count must refuse.
    // Unblockable.
    if kw(a, K::UNBLOCKABLE) {
        return false;
    }
    // "Can't block" (CR 509.1b, the restrictions half): read on the
    // **blocker**, where the line above is read on the attacker. The rule
    // covers both in one sentence and they are still two questions — a
    // creature that can't block and a creature that can't be blocked are
    // different cards, and a reader that asked only the attacker answered
    // one of them.
    if kw(b, K::CANT_BLOCK) {
        return false;
    }
    // Protection (CR 702.16f): can't be blocked by matching creatures.
    if crate::eval::protected_from(state, attacker, blocker) {
        return false;
    }
    // "Can't be blocked by creatures with power 2 or less" (CR 509.1b):
    // the attacker's restriction, asked of this blocker.
    if state.effects.iter().any(|fx| {
        let (filter, affected, other, source) = match fx.modifier {
            // The Black Gate: a player, bound as its ability resolved.
            baylee_cards_dsl::Modifier::CantBeBlockedByPlayer(player) => {
                return b.controller == player && crate::effects::applies_to(state, fx, a);
            }
            baylee_cards_dsl::Modifier::CantBeBlockedBy(filter) => (filter, a, b, attacker),
            baylee_cards_dsl::Modifier::CantBlock(filter) => (filter, b, a, blocker),
            _ => return false,
        };
        crate::effects::applies_to(state, fx, affected)
            && crate::eval::matches(
                filter,
                state,
                other,
                fx.controller,
                fx.source.unwrap_or(source),
            )
    }) {
        return false;
    }
    true
}

/// Whether `defending` could field the two blockers CR 702.111b demands.
///
/// The half of menace that *is* answerable one attacker at a time. The
/// restriction itself is on the whole declaration, so [`can_block`] does not
/// try — but a defender who has only one creature that may legally block a
/// menace attacker has no legal declaration that blocks it at all, and an
/// offer naming that pairing would name a block `declare_blockers` must
/// refuse. The engine publishes only what a player may actually answer.
///
/// The count is over distinct blockers and never over blocks: CR 702.111b
/// asks for two or more *creatures*, so a single creature that may block an
/// additional creature still answers it once. `take(2)` walks `candidates`,
/// which holds each permanent once: [`ready_blockers`], or any list of
/// permanents that includes them (the whole battlefield answers the same,
/// only slower, since [`can_block`] asks the blocker's half again).
///
/// `true` for an attacker without menace, so callers may ask it of every
/// attacker without asking twice. The count asked for is the one
/// [`block_bound`] states, so the offer and the declaration read one number.
#[must_use]
pub fn menace_satisfiable(
    state: &GameState,
    defending: PlayerId,
    attacker: ObjectId,
    candidates: &[ObjectId],
) -> bool {
    if state.object(attacker).is_none() {
        return false;
    }
    let Some(bound) = block_bound(state, attacker) else {
        return true;
    };
    let need = usize::try_from(bound.min_blockers).unwrap_or(usize::MAX);
    candidates
        .iter()
        .copied()
        .filter(|blocker| can_block(state, defending, *blocker, attacker))
        .take(need)
        .count()
        == need
}

/// How many creatures may block `attacker`, where a rule bounds it: the
/// restriction on the whole declaration that CR 509.1b checks and no pair
/// can answer. Menace is two or more (CR 702.111b); an attacker nobody
/// blocks keeps it.
///
/// `None` for an attacker any number of creatures may block. The one
/// reader of the rule: the offer states what this returns
/// (`Pending::ChooseBlockers::bounds`), and `Engine::declare_blockers`
/// holds a declaration to it.
#[must_use]
pub fn block_bound(state: &GameState, attacker: ObjectId) -> Option<crate::choice::AttackerBound> {
    let a = state.object(attacker)?;
    a.characteristics()
        .keywords
        .contains(K::MENACE)
        .then_some(crate::choice::AttackerBound {
            attacker,
            min_blockers: 2,
            max_blockers: u32::MAX,
        })
}

/// The declare-blockers offer for `defending` (CR 509.1a): each creature
/// that may block, and the attackers it may block.
///
/// CR 702.111b restricts the declaration and not the pair, so [`can_block`]
/// cannot answer it — but an attacker this defender could never field two
/// legal blockers against is one no legal declaration blocks, and offering
/// that pairing would name a block `declare_blockers` has to refuse (#156).
/// Asked once per attacker rather than once per pair, because the answer is
/// the same for every blocker.
///
/// Both walks go over the defender's ready creatures, not the battlefield:
/// the pairs they skip are ones [`can_block`] refuses on the blocker's half
/// alone, and walking the battlefield per attacker made this step cost
/// permanents × attackers ([`ready_blockers`]).
///
/// The one universe the block requirements are measured in: a pair outside
/// it breaks a restriction, and "the maximum possible number of
/// requirements that could be obeyed without disobeying any restrictions"
/// (CR 509.1c) is a maximum over declarations made of these pairs.
#[must_use]
pub fn block_options(
    state: &GameState,
    defending: PlayerId,
    mana_reach: u64,
) -> Vec<crate::choice::BlockOption> {
    let rules = BlockRules::new(state).reaching(mana_reach);
    let candidates: Vec<ObjectId> = ready_blockers(state, defending)
        .into_iter()
        .filter(|blocker| rules.can_pay_mana(*blocker))
        .collect();
    let blockable: Vec<ObjectId> = state
        .combat
        .attackers()
        .iter()
        .map(|a| a.creature)
        .filter(|a| menace_satisfiable(state, defending, *a, &candidates))
        .collect();
    candidates
        .iter()
        .copied()
        .filter_map(|blocker| {
            let attackers: Vec<ObjectId> = blockable
                .iter()
                .copied()
                .filter(|a| can_block(state, defending, blocker, *a))
                .collect();
            (!attackers.is_empty()).then_some(crate::choice::BlockOption { blocker, attackers })
        })
        .collect()
}

/// The rules about the declaration of blockers that effects add to what
/// [`can_block`] asks of one pair: how many attackers a creature may block
/// (CR 509.1a gives it one; "can block an additional creature each
/// combat" raises that), and the requirements (CR 509.1c: "all creatures
/// able to block enchanted creature do so", "it blocks each attacking
/// creature if able").
///
/// Collected once per declaration, for the reason [`AttackRules`] gives.
///
/// # Counting requirements
///
/// A requirement here is always about one pair, a blocker and an attacker:
/// a lure asks each creature able to block its creature to block it, and
/// "blocks each attacking creature" asks its creature to block each
/// attacker. [`BlockRules::demands`] is how many requirements ask for one
/// pair — two lures on one attacker ask twice, and blocking it obeys both —
/// and the requirements a declaration obeys are the sum over its pairs.
///
/// # The maximum
///
/// CR 509.1c refuses a declaration obeying fewer requirements than the
/// most any declaration could obey without breaking a restriction. The
/// restrictions the engine knows are the pairs [`block_options`] offers,
/// each blocker's limit, and menace's two blockers or none (CR 702.111b).
/// Without menace the blockers do not touch one another, and each obeys
/// most by blocking the attackers most requirements ask of it, up to its
/// limit: [`BlockRules::obeying`] does exactly that, so its count is the
/// maximum. A menace attacker a requirement names ties blockers together —
/// blocking it needs a second blocker, whose own requirements may then go
/// unobeyed — and there the declaration `obeying` builds is one good
/// declaration and not necessarily the best: its count can fall short of
/// the maximum, and a declaration between the two is accepted. It never
/// refuses a legal declaration, since `obeying`'s own is always one.
pub struct BlockRules<'a> {
    state: &'a GameState,
    additional: Vec<&'a crate::effects::ContinuousEffect>,
    any_number: Vec<&'a crate::effects::ContinuousEffect>,
    lures: Vec<&'a crate::effects::ContinuousEffect>,
    each: Vec<&'a crate::effects::ContinuousEffect>,
    unless_pay_mana: Vec<&'a crate::effects::ContinuousEffect>,
    max_blockers: Option<usize>,
    /// Mana the defending player could still make in the payment window.
    mana_reach: u64,
}

impl<'a> BlockRules<'a> {
    /// The rules in force now.
    #[must_use]
    pub fn new(state: &'a GameState) -> Self {
        use baylee_cards_dsl::Modifier;
        let mut rules = Self {
            state,
            additional: Vec::new(),
            any_number: Vec::new(),
            lures: Vec::new(),
            each: Vec::new(),
            unless_pay_mana: Vec::new(),
            max_blockers: None,
            mana_reach: 0,
        };
        for fx in state.effects.iter() {
            match fx.modifier {
                Modifier::CantBlockUnlessPayManaPerCounter(_) => rules.unless_pay_mana.push(fx),
                Modifier::CanBlockAdditional(_) => rules.additional.push(fx),
                Modifier::CanBlockAnyNumber => rules.any_number.push(fx),
                Modifier::MustBeBlockedByAllAble | Modifier::MustBeBlockedByAllAbleMatching(_) => {
                    rules.lures.push(fx);
                }
                Modifier::BlocksEachAttackerIfAble => rules.each.push(fx),
                Modifier::MaxBlockersPerCombat(n)
                    if crate::effects::source_is_present(state, fx) =>
                {
                    let cap = usize::from(n);
                    rules.max_blockers = Some(rules.max_blockers.map_or(cap, |cur| cur.min(cap)));
                }
                _ => {}
            }
        }
        rules
    }

    /// Whether any block costs mana, so finding the defender's mana reach is
    /// worth it. Each blocker is offered against the whole reach; the total
    /// cost of several blockers is still checked once, at declaration.
    #[must_use]
    pub fn has_mana_tax(&self) -> bool {
        !self.unless_pay_mana.is_empty()
    }

    /// The same rules with up to `mana` more mana available in the payment
    /// window (CR 509.1d–f).
    #[must_use]
    pub const fn reaching(mut self, mana: u64) -> Self {
        self.mana_reach = mana;
        self
    }

    /// Whether `blocker`'s own tax can be paid from its controller's pool or
    /// the sources the payment window offers.
    #[must_use]
    fn can_pay_mana(&self, blocker: ObjectId) -> bool {
        let tax = self.mana_tax(blocker);
        if tax == 0 {
            return true;
        }
        let Some(object) = self.state.object(blocker) else {
            return false;
        };
        let pool = &self.state.players[usize::from(object.controller.get())].mana_pool;
        crate::mana_pay::can_pay(
            pool,
            &baylee_core::mana::ManaCost::ZERO.with_more_generic(tax),
        ) || pool.total().saturating_add(self.mana_reach) >= u64::from(tax)
    }

    /// Maximum number of creatures that may block this combat, if capped.
    #[must_use]
    pub const fn max_blockers(&self) -> Option<usize> {
        self.max_blockers
    }

    /// Whether any block requirement is in force. Without one, every
    /// declaration obeys the most that can be obeyed, which is none.
    #[must_use]
    pub fn has_requirements(&self) -> bool {
        !self.lures.is_empty() || !self.each.is_empty()
    }

    /// Total generic mana cost `blocker`'s controller must pay to declare it
    /// blocking (CR 509.1d–f).
    #[must_use]
    pub fn mana_tax(&self, blocker: ObjectId) -> u32 {
        let Some(obj) = self.state.object(blocker) else {
            return 0;
        };
        self.unless_pay_mana
            .iter()
            .filter_map(|fx| {
                let baylee_cards_dsl::Modifier::CantBlockUnlessPayManaPerCounter(kind) =
                    fx.modifier
                else {
                    return None;
                };
                crate::effects::applies_to(self.state, fx, obj)
                    .then_some(u32::from(obj.counters.get(kind)))
            })
            .fold(0_u32, u32::saturating_add)
    }

    /// How many attackers `blocker` may block (CR 509.1a): one, one more
    /// for each "additional creature", and `None` for any number.
    #[must_use]
    pub fn capacity(&self, blocker: ObjectId) -> Option<usize> {
        let state = self.state;
        let Some(obj) = state.object(blocker) else {
            return Some(1);
        };
        if self
            .any_number
            .iter()
            .any(|fx| crate::effects::applies_to(state, fx, obj))
        {
            return None;
        }
        let more: usize = self
            .additional
            .iter()
            .filter(|fx| crate::effects::applies_to(state, fx, obj))
            .map(|fx| match fx.modifier {
                baylee_cards_dsl::Modifier::CanBlockAdditional(n) => usize::from(n),
                _ => 0,
            })
            .sum();
        Some(1 + more)
    }

    /// How many requirements ask `blocker` to block `attacker`: each lure
    /// on the attacker and each "blocks each attacking creature" on the
    /// blocker. Asked only of a pair [`block_options`] offers, which is
    /// what "able to block" means.
    #[must_use]
    pub fn demands(&self, blocker: ObjectId, attacker: ObjectId) -> usize {
        let state = self.state;
        let on = |effects: &[&crate::effects::ContinuousEffect], id: ObjectId| {
            state.object(id).map_or(0, |obj| {
                effects
                    .iter()
                    .filter(|fx| crate::effects::applies_to(state, fx, obj))
                    // Marble Priest's lure binds only the blockers its
                    // filter names (Walls), read with the effect's
                    // controller as "you".
                    .filter(|fx| match fx.modifier {
                        baylee_cards_dsl::Modifier::MustBeBlockedByAllAbleMatching(filter) => {
                            state.object(blocker).is_some_and(|b| {
                                crate::eval::matches(
                                    filter,
                                    state,
                                    b,
                                    fx.controller,
                                    fx.source.unwrap_or(attacker),
                                )
                            })
                        }
                        _ => true,
                    })
                    .count()
            })
        };
        on(&self.lures, attacker) + on(&self.each, blocker)
    }

    /// How many requirements `declared` obeys. Each pair counts once, so a
    /// declaration naming a pair twice is refused before it is counted.
    #[must_use]
    pub fn obeyed(&self, declared: &[(ObjectId, ObjectId)]) -> usize {
        if !self.has_requirements() {
            return 0;
        }
        declared.iter().map(|(b, a)| self.demands(*b, *a)).sum()
    }

    /// One legal declaration out of `options` that obeys as many
    /// requirements as the engine holds a declaration to (the header's
    /// "The maximum"): empty when none is in force.
    ///
    /// Each blocker takes the attackers most requirements ask of it, up to
    /// its limit — attackers without menace first, since blocking one
    /// needs nobody else, and within each kind the most asked first. A
    /// menace attacker left with one blocker then gets a second from any
    /// creature that may block it and has room, or loses the one it has
    /// (CR 702.111b: two or none). Every pair is one `options` offers, and
    /// no blocker exceeds its limit, so `declare_blockers` accepts it.
    #[must_use]
    pub fn obeying(&self, options: &[crate::choice::BlockOption]) -> Vec<(ObjectId, ObjectId)> {
        if !self.has_requirements() {
            return Vec::new();
        }
        let menace = |attacker: ObjectId| has_keyword(self.state, attacker, K::MENACE);
        let room = |blocker: ObjectId| self.capacity(blocker).unwrap_or(usize::MAX);
        let mut distinct_blockers: std::collections::BTreeSet<ObjectId> = self
            .state
            .combat
            .blockers
            .iter()
            .map(|b| b.blocker)
            .collect();
        let can_add_blocker =
            |set: &std::collections::BTreeSet<ObjectId>, b: ObjectId| match self.max_blockers {
                Some(max) => set.contains(&b) || set.len() < max,
                None => true,
            };
        let mut chosen: Vec<(ObjectId, ObjectId)> = Vec::new();
        for option in options {
            if !can_add_blocker(&distinct_blockers, option.blocker) {
                continue;
            }
            let mut asked: Vec<(bool, std::cmp::Reverse<usize>, ObjectId)> = option
                .attackers
                .iter()
                .filter_map(|attacker| {
                    let demands = self.demands(option.blocker, *attacker);
                    (demands > 0).then_some((
                        menace(*attacker),
                        std::cmp::Reverse(demands),
                        *attacker,
                    ))
                })
                .collect();
            asked.sort_by_key(|(menace, demands, _)| (*menace, *demands));
            let before = chosen.len();
            chosen.extend(
                asked
                    .into_iter()
                    .take(room(option.blocker))
                    .map(|(_, _, attacker)| (option.blocker, attacker)),
            );
            if chosen.len() > before {
                distinct_blockers.insert(option.blocker);
            }
        }
        let menacing: Vec<ObjectId> = chosen
            .iter()
            .map(|(_, attacker)| *attacker)
            .filter(|attacker| menace(*attacker))
            .collect();
        for attacker in menacing {
            let mut on_it = chosen.iter().filter(|(_, a)| *a == attacker);
            let (Some(&(lone, _)), None) = (on_it.next(), on_it.next()) else {
                continue;
            };
            let helper = options.iter().find(|o| {
                o.blocker != lone
                    && can_add_blocker(&distinct_blockers, o.blocker)
                    && o.attackers.contains(&attacker)
                    && chosen.iter().filter(|(b, _)| *b == o.blocker).count() < room(o.blocker)
            });
            match helper {
                Some(helper) => {
                    distinct_blockers.insert(helper.blocker);
                    chosen.push((helper.blocker, attacker));
                }
                None => chosen.retain(|pair| *pair != (lone, attacker)),
            }
        }
        chosen
    }
}

impl GameState {
    pub(crate) fn blocked_only_by(&self, attacker: ObjectId, blocker: ObjectId) -> bool {
        self.source_identity(attacker)
            .zip(self.source_identity(blocker))
            .is_some_and(|(attacker, blocker)| self.combat.blocked_only_by(attacker, blocker))
    }

    /// Records a block pair and the first blocked transition separately.
    pub(crate) fn declare_block(&mut self, blocker: ObjectId, attacker: ObjectId) {
        let became_blocked = !self.combat.is_blocked(attacker);
        let (Some(blocker_ref), Some(attacker_ref)) = (
            self.source_identity(blocker),
            self.source_identity(attacker),
        ) else {
            return;
        };
        self.combat.declare_block(blocker_ref, attacker_ref);
        self.record_blocked_or_blocking(blocker_ref, attacker_ref);
        if let Some(controller) = self.object(attacker).map(|o| o.controller) {
            self.per_turn.blocks.push(crate::state::BlockRecord {
                blocker: blocker_ref,
                attacker: attacker_ref,
                attacker_controller: controller,
            });
        }
        self.journal.record(crate::event::GameEvent::BecameBlocker {
            object: blocker,
            attacker,
        });
        if became_blocked {
            self.journal
                .record(crate::event::GameEvent::AttackerBlocked { object: attacker });
        }
    }
}
