//! What using an ability is worth on this board, net of what it costs.
//!
//! One measure for three questions that used to be answered three ways:
//! whether an activation is worth taking, which loyalty ability a deep
//! profile uses, and which target an activation names once it is taken. The
//! first was a whitelist of effects that are gains wherever they land, so an
//! ability that is good or bad according to its target — a Wasteland, a Maze
//! of Ith, a Recurring Nightmare, an equip — was one the agent never took;
//! the third ranked candidates by the sign of the effect list, so even an
//! ability taken for the right target would have pointed wherever that sign
//! sent it. Now both ask [`HeuristicAgent::effects_worth`] of the same
//! candidates, and the target named is the one the activation was valued
//! for.
//!
//! The currency is this crate's usual one: a card drawn is [`CARD`],
//! [`crate::tactics::material`] prices a permanent, a point of life is
//! [`life_price`]. What the table cannot read is `None`, never a guess: an
//! effect, amount, target shape or cost part without a row here leaves the
//! whole ability alone, so a mechanic added tomorrow is inert rather than
//! misplayed. An effect with a row but nothing to act on — no target, no
//! object — is worth `Some(0)`, which keeps "can this be read?" a question
//! about the table and not about the board.

use baylee_cards_dsl::{
    AbilityDef, Amount, Cost, CostPart, CounterKind, Duration, Effect, Filter, KeywordSet,
    Modifier, PlayerRel, TargetReq, TargetSpec, Trigger, ZoneRef, ZoneSel,
};
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_core::types::TypeSet;
use baylee_view::{ObjectStatus, Phase, PlayerView, PublicObject, StackItem, Step, TargetRef};

use crate::HeuristicAgent;
use crate::activate::printed_list;
use crate::combat::{self, Fighter};
use crate::tactics::material;

/// Match engine counter identities to their public representation.
fn counter_view(kind: CounterKind) -> baylee_view::CounterKind {
    use baylee_view::CounterKind as V;
    match kind {
        CounterKind::Plus { power, toughness } => V::Plus { power, toughness },
        CounterKind::Minus { power, toughness } => V::Minus { power, toughness },
        CounterKind::Loyalty => V::Loyalty,
        CounterKind::Lore => V::Lore,
        CounterKind::Time => V::Time,
        CounterKind::Charge => V::Charge,
        CounterKind::Poison => V::Poison,
        CounterKind::Energy => V::Energy,
        CounterKind::Rad => V::Rad,
        CounterKind::Lifelink => V::Lifelink,
        CounterKind::Level => V::Level,
        CounterKind::Custom(id) => V::Custom(u32::from(id)),
    }
}

/// A card drawn: the unit the rest is measured against.
const CARD: i64 = 400;
/// Drawing from an empty library (CR 704.5b), or making an opponent do it.
const DECKED: i64 = 100_000;
/// Damage, an attack or a draw that ends the game.
pub(crate) const LETHAL: i64 = 50_000;
/// A trigger that fires on a draw, for its controller: Sheoldred, the
/// Apocalypse's life on either side of the table, Orcish Bowmasters' ping.
/// A trigger works for whoever controls it, which is the whole reading.
const PUNISH: i64 = 150;
/// A loyalty counter, the price a planeswalker's ability is paid in.
const LOYALTY: i64 = 45;
/// The least an activation must be worth, net of its cost, to be taken.
///
/// Above zero so that a use worth nothing — a type change alone, a
/// symmetric draw — is not taken for the rounding, and small so that a
/// mana the seat would not otherwise spend buys what it can.
pub(crate) const THRESHOLD: i64 = 25;

/// A change to a creature: power, toughness and the keyword bits it gains.
type Pump = (i32, i32, u128);

/// What an ability's effects are pointed at.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Aim {
    /// An ability that targets nothing: "it" is its source (CR 113.7).
    Source,
    /// "Up to one" with none named: the target's half does nothing.
    Nothing,
    /// A permanent, a card in a graveyard or a spell.
    Object(ObjectId),
    /// A player.
    Player(PlayerId),
}

/// Where an ability comes from: the object `This` names, and that object
/// as the view shows it when it is on the table. A card in hand has an id
/// and no public object.
#[derive(Clone, Copy)]
pub(crate) struct Origin<'a> {
    pub id: ObjectId,
    pub object: Option<&'a PublicObject>,
}

impl<'a> Origin<'a> {
    pub(crate) fn of(view: &'a PlayerView, id: ObjectId) -> Self {
        Self {
            id,
            object: view.object(id),
        }
    }
}

/// What a point of `player`'s life is worth to them: little at twenty, a
/// great deal at five.
pub(crate) fn life_price(view: &PlayerView, player: PlayerId) -> i64 {
    match view.seat(player).map_or(20, |s| s.life) {
        ..=5 => 150,
        6..=10 => 60,
        _ => 30,
    }
}

/// `n` damage or life loss to `player`, as it is worth to that player.
fn hurt(view: &PlayerView, player: PlayerId, n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    if view.seat(player).is_some_and(|s| i64::from(s.life) <= n) {
        LETHAL
    } else {
        n * life_price(view, player)
    }
}

fn has(o: &PublicObject, k: KeywordSet) -> bool {
    o.keywords & k.bits() != 0
}

fn creature(o: &PublicObject) -> bool {
    o.types.contains(TypeSet::CREATURE)
}

/// Able to attack this turn, as the view says it.
fn can_attack(o: &PublicObject) -> bool {
    creature(o)
        && !o.status.contains(ObjectStatus::TAPPED)
        && !o.status.contains(ObjectStatus::PHASED_OUT)
        && (!o.summoning_sick || has(o, KeywordSet::HASTE))
        && !has(o, KeywordSet::DEFENDER)
        && o.power.unwrap_or(0) > 0
}

/// Whether `o` makes mana when tapped, so a tap of it is a mana spent.
fn makes_mana(o: &PublicObject) -> bool {
    o.granted_mana.is_some()
        || baylee_client_core::manaplan::basic_land_color(&o.subtypes).is_some()
        || printed_list(o).iter().any(|a| {
            matches!(
                a,
                AbilityDef::Activated {
                    mana_ability: true,
                    ..
                } | AbilityDef::ActivatedConditional {
                    mana_ability: true,
                    ..
                }
            )
        })
}

impl HeuristicAgent {
    /// Net worth of activating `def` from `origin` now, at its best target;
    /// `None` where any part of it cannot be read.
    ///
    /// A loyalty ability's price is loyalty, and a walker that pays its last
    /// counter is a card spent (CR 704.5i); both are the numbers the deep
    /// profiles used before this measure existed.
    pub(crate) fn activation_worth(
        &self,
        view: &PlayerView,
        origin: Origin<'_>,
        def: &AbilityDef,
    ) -> Option<i64> {
        match def {
            AbilityDef::Activated {
                cost,
                effects,
                targets,
                second_targets,
                mana_ability: false,
                ..
            }
            | AbilityDef::ActivatedConditional {
                cost,
                effects,
                targets,
                second_targets,
                mana_ability: false,
                ..
            } => {
                if second_targets.is_some() {
                    return None;
                }
                let (gain, _) = self.aimed_worth(view, origin, effects, targets.as_ref(), 0)?;
                Some(gain - self.cost_worth(view, origin, cost)?)
            }
            AbilityDef::Loyalty {
                cost,
                effects,
                targets,
                second_targets,
            } => {
                // A walker's text this table cannot read is still a walker's
                // text: its plus is loyalty gained, and ticking it is what
                // keeps the card alive. So an unread ability is weighed the
                // way the deep profiles weighed every loyalty ability before
                // this table existed (`effect_value`), not left alone — left
                // alone, Teferi, Time Raveler's +1 and Karn, the Great
                // Creator's +1 were never used and both walkers spent
                // themselves on their minus.
                let gain = if second_targets.is_some() {
                    None
                } else {
                    self.aimed_worth(view, origin, effects, targets.as_ref(), 0)
                }
                .map_or_else(|| self.effect_value(view, effects), |(gain, _)| gain);
                let left = origin.object.map_or(0, |o| {
                    o.counters
                        .iter()
                        .filter(|c| c.kind == baylee_view::CounterKind::Loyalty)
                        .map(|c| i64::from(c.count))
                        .sum::<i64>()
                });
                let mut net = gain + i64::from(*cost) * LOYALTY;
                if *cost < 0 && left <= i64::from(cost.unsigned_abs()) {
                    net -= 300;
                }
                // Loyalty is also the walker's life (CR 306.8): a minus that
                // brings it into reach of what can attack it next turn risks
                // the card, and a plus that lifts it out of reach saves it.
                // Only the change counts — doing nothing leaves it where it
                // stands — so a walker in reach either way still spends
                // itself on what it does best before it goes.
                if let Some(walker) = origin.object {
                    let reach = self.reach(view, walker);
                    let risk = material(walker) / 2;
                    let exposed = |loyalty: i64| reach > 0 && loyalty <= reach;
                    let after = left + i64::from(*cost);
                    net += risk * (i64::from(exposed(left)) - i64::from(exposed(after)));
                }
                Some(net)
            }
            _ => None,
        }
    }

    /// The damage that can reach `walker` in its controller's opponents'
    /// next combat: the power of every creature of theirs that can attack.
    /// Blockers are not subtracted — a blocker spent on a walker is not
    /// free, and the question is whether the walker is in reach at all.
    fn reach(&self, view: &PlayerView, walker: &PublicObject) -> i64 {
        view.battlefield
            .iter()
            .filter(|c| {
                self.hostile(c.controller, walker.controller)
                    && creature(c)
                    && !has(c, KeywordSet::DEFENDER)
            })
            .map(|c| i64::from(c.power.unwrap_or(0).max(0)))
            .sum()
    }

    /// The best of what `targets` may name for `effects`, and its worth.
    ///
    /// The candidates are this seat's own reading of the target words, so
    /// they can include one the engine will not offer (protection, say); the
    /// target question is answered from the engine's offer by the same
    /// measure, so a miss here costs a slightly wrong estimate and never a
    /// target that disagrees with it.
    pub(crate) fn aimed_worth(
        &self,
        view: &PlayerView,
        origin: Origin<'_>,
        effects: &[Effect],
        targets: Option<&TargetReq>,
        x: u32,
    ) -> Option<(i64, Aim)> {
        let Some(req) = targets else {
            return Some((
                self.effects_worth(view, origin, effects, Aim::Source, x)?,
                Aim::Source,
            ));
        };
        let mut aims = self.aims(view, origin, &req.spec)?;
        if req.min == 0 {
            aims.push(Aim::Nothing);
        }
        aims.into_iter()
            .filter_map(|aim| Some((self.effects_worth(view, origin, effects, aim, x)?, aim)))
            .fold(None, |best: Option<(i64, Aim)>, next| match best {
                Some(b) if b.0 >= next.0 => Some(b),
                _ => Some(next),
            })
    }

    /// Everything `spec` could name on this board, in view order.
    fn aims(&self, view: &PlayerView, origin: Origin<'_>, spec: &TargetSpec) -> Option<Vec<Aim>> {
        let me = view.seat;
        let live = || {
            view.seats
                .iter()
                .filter(|s| !s.has_lost())
                .map(|s| s.player)
        };
        // Shroud stops everyone and hexproof stops opponents (CR 702.18a,
        // 702.11b); ward is a price, and a price is still a legal target.
        let open = |o: &PublicObject| {
            !(has(o, KeywordSet::SHROUD)
                || (self.hostile(o.controller, me) && has(o, KeywordSet::HEXPROOF)))
        };
        let matching =
            |objects: &mut dyn Iterator<Item = &PublicObject>, filter: &Filter, zone: ZoneRef| {
                objects
                    .filter(|o| {
                        self.filter_matches(filter, view, o, zone, Some(origin.id)) == Some(true)
                    })
                    .map(|o| Aim::Object(o.id))
                    .collect::<Vec<_>>()
            };
        Some(match spec {
            TargetSpec::Object(filter) => matching(
                &mut view.battlefield.iter().filter(|o| open(o)),
                filter,
                ZoneRef::Battlefield,
            ),
            TargetSpec::ThisObject => vec![Aim::Object(origin.id)],
            TargetSpec::CardInGraveyard(filter, rel) => {
                let owners = self.seats(*rel, view)?;
                matching(
                    &mut view
                        .graveyards
                        .iter()
                        .flatten()
                        .filter(|o| owners.contains(&o.owner)),
                    filter,
                    ZoneRef::Graveyard,
                )
            }
            TargetSpec::Spell(filter) => matching(
                &mut view
                    .stack
                    .iter()
                    .filter(|o| !matches!(o.stack_item, Some(StackItem::Ability { .. }))),
                filter,
                ZoneRef::Stack,
            ),
            TargetSpec::Player(rel) => self
                .seats(*rel, view)?
                .into_iter()
                .map(Aim::Player)
                .collect(),
            TargetSpec::AnyPlayer => live().map(Aim::Player).collect(),
            TargetSpec::AnyOpponent => live()
                .filter(|p| self.hostile(*p, me))
                .map(Aim::Player)
                .collect(),
            TargetSpec::AnyTarget => view
                .battlefield
                .iter()
                .filter(|o| {
                    open(o)
                        && o.types
                            .intersects(TypeSet::CREATURE.union(TypeSet::PLANESWALKER))
                })
                .map(|o| Aim::Object(o.id))
                .chain(live().map(Aim::Player))
                .collect(),
            _ => return None,
        })
    }

    /// The worth of a list of effects run in order at `aim`.
    pub(crate) fn effects_worth(
        &self,
        view: &PlayerView,
        origin: Origin<'_>,
        effects: &[Effect],
        aim: Aim,
        x: u32,
    ) -> Option<i64> {
        effects.iter().try_fold(0i64, |sum, effect| {
            Some(sum.saturating_add(self.effect_worth(view, origin, effect, aim, x)?))
        })
    }

    /// The seat's reading of an amount; `None` where the view cannot say.
    fn count(
        &self,
        view: &PlayerView,
        origin: Origin<'_>,
        amount: Amount,
        aim: Aim,
        x: u32,
    ) -> Option<i64> {
        if let Amount::Negated(inner) = amount {
            return self.count(view, origin, *inner, aim, x).map(|n| -n);
        }
        let target = match aim {
            Aim::Object(id) => view.object(id),
            _ => None,
        };
        let magnitude = match amount {
            Amount::Fixed(n) | Amount::NegXFixed(n) => i64::from(n),
            Amount::X | Amount::NegX => i64::from(x),
            Amount::DoubleX => 2 * i64::from(x),
            Amount::Plus { base, offset } => {
                self.count(view, origin, *base, aim, x)? + i64::from(offset)
            }
            Amount::SaturatingSub { base, subtract } => {
                (self.count(view, origin, *base, aim, x)? - i64::from(subtract)).max(0)
            }
            Amount::CountOf { filter, zone } => self.count_of(view, origin, filter, zone)?,
            Amount::DistinctColorsAmong(filter) => i64::from(
                view.battlefield
                    .iter()
                    .filter(|o| {
                        self.filter_matches(filter, view, o, ZoneRef::Battlefield, Some(origin.id))
                            == Some(true)
                    })
                    .fold(baylee_core::color::ColorSet::EMPTY, |all, o| {
                        all.union(o.colors)
                    })
                    .len(),
            ),
            Amount::SourcePower => i64::from(origin.object?.power?),
            Amount::TargetPower => i64::from(target?.power?),
            Amount::TargetCmc => i64::from(target?.mana_value),
            _ => return None,
        };
        Some(if amount.is_negative() {
            -magnitude
        } else {
            magnitude
        })
    }

    /// How many objects in `zone` match `filter`; `None` when one of them
    /// cannot be read, since then the count is not known.
    fn count_of(
        &self,
        view: &PlayerView,
        origin: Origin<'_>,
        filter: &Filter,
        zone: ZoneSel,
    ) -> Option<i64> {
        let me = usize::from(view.seat.get());
        let (objects, zone): (Vec<&PublicObject>, ZoneRef) = match zone {
            ZoneSel::Battlefield => (view.battlefield.iter().collect(), ZoneRef::Battlefield),
            ZoneSel::GraveyardYou => (
                view.graveyards.get(me).into_iter().flatten().collect(),
                ZoneRef::Graveyard,
            ),
            ZoneSel::GraveyardAll => (
                view.graveyards.iter().flatten().collect(),
                ZoneRef::Graveyard,
            ),
            ZoneSel::HandActivePlayer if matches!(filter, Filter::Any) => {
                return view.seat(view.active).map(|s| i64::from(s.hand_count));
            }
            ZoneSel::HandYou if matches!(filter, Filter::Any) => {
                return i64::try_from(view.hand.len()).ok();
            }
            ZoneSel::LibraryYou if matches!(filter, Filter::Any) => {
                return view.seat(view.seat).map(|s| i64::from(s.library_count));
            }
            ZoneSel::HandYou | ZoneSel::HandActivePlayer | ZoneSel::LibraryYou => return None,
        };
        objects.into_iter().try_fold(0i64, |n, o| {
            Some(n + i64::from(self.filter_matches(filter, view, o, zone, Some(origin.id))?))
        })
    }

    /// The players `rel` names from this seat, with the aim filling in
    /// "target player" and "that permanent's controller".
    fn players(&self, view: &PlayerView, rel: PlayerRel, aim: Aim) -> Option<Vec<PlayerId>> {
        match (rel, aim) {
            (PlayerRel::Chosen, Aim::Player(p)) => Some(vec![p]),
            (PlayerRel::ControllerOfTarget, Aim::Object(id)) => {
                view.object(id).map(|o| vec![o.controller])
            }
            (PlayerRel::Chosen | PlayerRel::ControllerOfTarget, _) => Some(Vec::new()),
            _ => self.seats(rel, view),
        }
    }

    /// `n` cards drawn by `player`, to this seat.
    ///
    /// The library bounds it both ways: this seat never draws the last card
    /// it has (the old whitelist's rule, kept), and an opponent made to draw
    /// more than they have loses (CR 704.5b). A draw trigger on the table
    /// works for whoever controls it.
    fn draw_worth(&self, view: &PlayerView, player: PlayerId, n: i64) -> i64 {
        let Some(seat) = view.seat(player) else {
            return 0;
        };
        if n <= 0 {
            return 0;
        }
        let library = i64::from(seat.library_count);
        let triggers = self.draw_triggers(view, player) * n;
        if player == view.seat {
            if library <= n {
                -DECKED
            } else {
                n * CARD + triggers
            }
        } else if self.hostile(player, view.seat) {
            if library < n {
                DECKED
            } else {
                -n * CARD + triggers
            }
        } else {
            n * CARD / 2 + triggers
        }
    }

    /// What one draw by `player` sets off on the table, to this seat: plus
    /// for each trigger this seat controls, minus for each an opponent does.
    fn draw_triggers(&self, view: &PlayerView, player: PlayerId) -> i64 {
        let mut sum = 0;
        for o in &view.battlefield {
            for ability in printed_list(o) {
                let AbilityDef::Triggered {
                    trigger: Trigger::Draws(rel) | Trigger::DrawsExceptFirst(rel),
                    ..
                } = ability
                else {
                    continue;
                };
                let fires = match rel {
                    PlayerRel::You => player == o.controller,
                    PlayerRel::Opponent | PlayerRel::EachOpponent => {
                        self.hostile(player, o.controller)
                    }
                    PlayerRel::EachPlayer => true,
                    _ => false,
                };
                if fires {
                    sum += if o.controller == view.seat {
                        PUNISH
                    } else if self.hostile(o.controller, view.seat) {
                        -PUNISH
                    } else {
                        0
                    };
                }
            }
        }
        sum
    }

    /// What a permanent is worth to whoever controls it.
    ///
    /// A land is not a body: it is a mana a turn, worth more the fewer its
    /// controller has, and more again when it does something besides —
    /// Wasteland's own reason to exist. Everything else is [`material`].
    pub(crate) fn permanent_worth(view: &PlayerView, o: &PublicObject) -> i64 {
        if o.types.contains(TypeSet::LAND) && !creature(o) {
            return land_worth(view, o, true);
        }
        material(o)
    }

    /// `o` leaving the battlefield for good, to this seat.
    fn removal(&self, view: &PlayerView, o: &PublicObject) -> i64 {
        let worth = Self::permanent_worth(view, o);
        if self.hostile(o.controller, view.seat) {
            worth
        } else {
            -worth
        }
    }

    /// Whether `o` is going anyway: an opponent's spell or ability on the
    /// stack points at it, or it is in a fight this combat that kills it.
    ///
    /// What a doomed permanent is worth is what it can still be spent on, so
    /// sacrificing it costs nothing and saving it is worth all of it —
    /// Sakura-Tribe Elder in front of an attacker, a Wizard under a removal
    /// spell with Riptide Laboratory untapped.
    fn doomed(&self, view: &PlayerView, o: &PublicObject) -> bool {
        let me = view.seat;
        let targeted = view.stack.iter().any(|s| {
            self.hostile(s.controller, me)
                && s.targets.iter().any(|target| match target {
                    TargetRef::Object(source) => {
                        source.object == o.id
                            && view
                                .target_object(*source)
                                .is_some_and(|target| target.is_current)
                    }
                    TargetRef::Player(_) => false,
                })
        });
        if targeted {
            return true;
        }
        if view.step != Step::DeclareBlockers {
            return false;
        }
        let Some(fo) = Fighter::from_object(o) else {
            return false;
        };
        let attacking = view.combat.attackers.iter().any(|a| a.creature == o.id);
        view.combat.blockers.iter().any(|b| {
            let (foe, dies) = if attacking && b.attacker == o.id {
                (b.blocker, true)
            } else if b.blocker == o.id {
                (b.attacker, false)
            } else {
                return false;
            };
            view.object(foe)
                .and_then(Fighter::from_object)
                .is_some_and(|ff| {
                    if dies {
                        combat::exchange(fo, ff).attacker_dies
                    } else {
                        combat::exchange(ff, fo).blocker_dies
                    }
                })
        })
    }

    /// What `o` of this seat's costs to give up: nothing when it is
    /// [`doomed`](Self::doomed), all of it otherwise.
    fn spent(&self, view: &PlayerView, o: &PublicObject, worth: i64) -> i64 {
        if self.doomed(view, o) { 0 } else { worth }
    }

    /// What sacrificing `o` gives up, the number a sacrifice cost is priced
    /// with and paid by.
    pub(crate) fn given_up(&self, view: &PlayerView, o: &PublicObject) -> i64 {
        self.spent(view, o, Self::permanent_worth(view, o))
    }

    /// What `o` entering the battlefield sets off for its controller, by
    /// the vocabulary [`crate::tactics::meaning`] reads: its own ETB
    /// triggers, never counted below zero.
    fn etb_worth(&self, view: &PlayerView, o: &PublicObject) -> i64 {
        printed_list(o)
            .iter()
            .map(|a| match a {
                AbilityDef::Triggered {
                    trigger: Trigger::EntersBattlefield(Filter::This),
                    effects,
                    ..
                } => self.effect_value(view, effects).max(0),
                _ => 0,
            })
            .sum()
    }

    /// `o` returned to its owner's hand.
    ///
    /// An opponent's creature sent back is the price of casting it again and
    /// the damage it would have dealt this seat before it could — at five
    /// life facing a 6/6, that is the game, and it is capped at the
    /// creature because it comes back.
    fn bounce(&self, view: &PlayerView, o: &PublicObject) -> i64 {
        let tempo = i64::from(o.mana_value) * 60 + 50;
        if self.hostile(o.controller, view.seat) {
            // A token does not come back (CR 111.7).
            if o.token.is_some() {
                return Self::permanent_worth(view, o);
            }
            let pressure = if creature(o) {
                hurt(view, view.seat, i64::from(o.power.unwrap_or(0).max(0))).min(material(o))
            } else {
                0
            };
            tempo + pressure
        } else if self.doomed(view, o) {
            Self::permanent_worth(view, o) - tempo
        } else {
            -tempo
        }
    }

    /// `o` exiled and returned (CR 400.7): a new object under its owner's
    /// control with its ETBs to run again and its counters gone.
    fn blink(&self, view: &PlayerView, o: &PublicObject) -> i64 {
        let me = view.seat;
        let hostile = self.hostile(o.controller, me);
        if hostile && o.token.is_some() {
            return Self::permanent_worth(view, o);
        }
        let worth = Self::permanent_worth(view, o);
        let counters: i64 = o
            .counters
            .iter()
            .map(|c| match c.kind {
                baylee_view::CounterKind::Plus { .. } => 110 * i64::from(c.count),
                baylee_view::CounterKind::Loyalty => LOYALTY * i64::from(c.count),
                _ => 0,
            })
            .sum();
        // Back under its owner's control: a permanent of this seat's that an
        // opponent was holding comes home, and one of theirs goes home.
        let homecoming = if o.owner == me && o.controller != me {
            2 * worth
        } else if o.controller == me && o.owner != me {
            -2 * worth
        } else {
            0
        };
        let own = !hostile && o.controller == me;
        let saved = if own && self.doomed(view, o) {
            worth
        } else {
            0
        };
        let fresh = self.etb_worth(view, o) - counters;
        homecoming + saved + if hostile { -fresh } else { fresh }
    }

    /// What an equipment is worth on `c`: its statics on the equipped
    /// creature, and its triggers — a token copy of `c` each combat is `c`
    /// once more.
    fn equipped(&self, view: &PlayerView, equipment: &PublicObject, c: &PublicObject) -> i64 {
        printed_list(equipment)
            .iter()
            .map(|a| match a {
                AbilityDef::Static(s) if matches!(s.filter, Filter::AttachedToBySource) => {
                    match s.modifier {
                        Modifier::ModifyPT(p, t) => i64::from(p) * 110 + i64::from(t) * 30,
                        Modifier::AddKeyword(k) => 60 * i64::from(k.bits().count_ones()),
                        _ => 40,
                    }
                }
                AbilityDef::Triggered { effects, .. } => {
                    if effects
                        .iter()
                        .any(|e| matches!(e, Effect::CreateTokenCopyOfEquipped { .. }))
                    {
                        material(c)
                    } else {
                        self.effect_value(view, effects).max(0) / 2
                    }
                }
                _ => 0,
            })
            .sum()
    }

    /// Moving the equipment `origin` onto `c`: what it is worth there, less
    /// what it was worth where it hung, and a little more on a creature that
    /// can swing with it this turn.
    fn attach(&self, view: &PlayerView, origin: Origin<'_>, c: &PublicObject) -> i64 {
        let Some(equipment) = origin.object else {
            return 0;
        };
        if equipment.attached_to == Some(c.id) || c.controller != view.seat || !creature(c) {
            return 0;
        }
        let holder = equipment
            .attached_to
            .and_then(|id| view.object(id))
            .filter(|w| w.controller == view.seat && creature(w));
        let early =
            view.active == view.seat && matches!(view.phase, Phase::Beginning | Phase::FirstMain);
        // Worth where it goes less worth where it hangs, the swing
        // included on both sides: a move back is then this move with its
        // sign turned, and the two can never both be taken. Counting the
        // swing only where it goes made every move between two creatures
        // that could attack worth the swing, and Shuko's free equip went
        // back and forth for ever (self-play mac-d001).
        let worn = |w: &PublicObject| {
            self.equipped(view, equipment, w) + if early && can_attack(w) { 50 } else { 0 }
        };
        worn(c) - holder.map_or(0, worn)
    }

    /// Prevention on a creature in combat: damage `o` deals (`from`) or is
    /// dealt, weighed by the fights it is in — or, attacking unblocked, by
    /// the player it would hit. Out of combat it prevents nothing.
    fn prevent(&self, view: &PlayerView, o: &PublicObject, from: bool) -> i64 {
        let me = view.seat;
        let attack = view.combat.attackers.iter().find(|a| a.creature == o.id);
        let foes: Vec<&PublicObject> = view
            .combat
            .blockers
            .iter()
            .filter_map(|b| {
                if attack.is_some() && b.attacker == o.id {
                    view.object(b.blocker)
                } else if b.blocker == o.id {
                    view.object(b.attacker)
                } else {
                    None
                }
            })
            .collect();
        let Some(fo) = Fighter::from_object(o) else {
            return 0;
        };
        let hostile = self.hostile(o.controller, me);
        if foes.is_empty() {
            // Blocked stays blocked when the blockers are gone (CR 509.1h),
            // and then deals its damage to nothing.
            let Some(attack) = attack.filter(|a| !a.blocked) else {
                return 0;
            };
            if !from {
                return 0;
            }
            let victim = match attack.defending {
                Defender::Player(p) => Some(p),
                Defender::Planeswalker(w) => view.object(w).map(|w| w.controller),
            };
            let Some(victim) = victim else {
                return 0;
            };
            let dealt = hurt(view, victim, i64::from(fo.power.max(0)));
            return match (hostile, self.hostile(victim, me)) {
                (true, false) => dealt,
                (false, true) => -dealt,
                _ => 0,
            };
        }
        let mut value = 0;
        for foe in foes {
            let Some(ff) = Fighter::from_object(foe) else {
                continue;
            };
            let fight = if attack.is_some() {
                combat::exchange(fo, ff)
            } else {
                let e = combat::exchange(ff, fo);
                combat::Exchange {
                    attacker_dies: e.blocker_dies,
                    blocker_dies: e.attacker_dies,
                }
            };
            // `attacker_dies` is now "o dies", `blocker_dies` "foe dies".
            if from && fight.blocker_dies {
                value -= self.removal(view, foe);
            }
            if !from && fight.attacker_dies {
                value -= self.removal(view, o);
            }
        }
        value
    }

    /// What this seat's attack this turn gains from `change` — extra power,
    /// toughness and keywords per creature — as the attack planner measures
    /// an attack: each defender blocks what it can ([`combat::through_to`]).
    /// Nothing outside this seat's turn before attackers are declared.
    fn attack_gain(
        &self,
        view: &PlayerView,
        change: &dyn Fn(&PublicObject) -> Option<Pump>,
    ) -> i64 {
        let me = view.seat;
        let early = matches!(view.phase, Phase::Beginning | Phase::FirstMain)
            || view.phase == Phase::Combat && view.step == Step::CombatBegin;
        if view.active != me || !early {
            return 0;
        }
        let mut before = Vec::new();
        let mut after = Vec::new();
        let mut lost = 0;
        for o in view.battlefield_of(me) {
            let Some(mut f) = Fighter::from_object(o) else {
                continue;
            };
            let delta = change(o);
            if can_attack(o) {
                before.push(f);
            }
            if let Some((p, t, k)) = delta {
                f.power += p;
                f.toughness += t;
                f.keywords |= k;
                // A change that leaves no toughness is the creature, not
                // an attack (CR 704.5f, and 704.5g for the damage it has
                // already taken). Flowstone Hellion's `+1/-1`, taken one
                // resolution at a time, walked a 3/3 into a 6/0.
                let lethal = i32::from(o.toughness.unwrap_or(0)) + t <= 0
                    || f.toughness <= 0 && !has(o, KeywordSet::INDESTRUCTIBLE);
                if t < 0 && lethal {
                    lost += self.removal(view, o);
                    continue;
                }
            }
            if f.power > 0 && can_attack(o) {
                after.push(f);
            }
        }
        view.seats
            .iter()
            .filter(|s| !s.has_lost() && self.hostile(s.player, me))
            .map(|s| {
                let a = i64::from(combat::through_to(view, &before, s.player));
                let b = i64::from(combat::through_to(view, &after, s.player));
                hurt(view, s.player, b) - hurt(view, s.player, a)
            })
            .max()
            .unwrap_or(0)
            + lost
    }

    /// Power, toughness and keywords given to `objects` for `duration`.
    ///
    /// For good, it is a bigger body. Until the turn ends, it is what it
    /// adds to this turn's attack and nothing else: a pump in the second
    /// main does nothing, and one in combat is not modelled yet.
    fn pump(
        &self,
        view: &PlayerView,
        objects: &[&PublicObject],
        (power, toughness, keywords): Pump,
        duration: Duration,
    ) -> i64 {
        if matches!(
            duration,
            Duration::Indefinitely
                | Duration::WhileSourceOnBattlefield
                | Duration::WhileYouControlSource
        ) {
            let each = i64::from(power) * 110
                + i64::from(toughness) * 30
                + 60 * i64::from(keywords.count_ones());
            return objects
                .iter()
                .filter(|o| creature(o))
                .map(|o| {
                    if self.hostile(o.controller, view.seat) {
                        -each
                    } else {
                        each
                    }
                })
                .sum();
        }
        let ids: Vec<ObjectId> = objects.iter().map(|o| o.id).collect();
        self.attack_gain(view, &|o| {
            ids.contains(&o.id).then_some((power, toughness, keywords))
        })
    }

    /// Counters put on `o`, to this seat.
    pub(crate) fn counters(
        &self,
        view: &PlayerView,
        o: &PublicObject,
        kind: CounterKind,
        n: i64,
    ) -> Option<i64> {
        let sign = if self.hostile(o.controller, view.seat) {
            -1
        } else {
            1
        };
        Some(match kind {
            CounterKind::Plus { power, toughness } if creature(o) => {
                sign * n * (i64::from(power) * 110 + i64::from(toughness) * 30)
            }
            CounterKind::Minus { toughness, .. } if creature(o) => {
                let left = i64::from(o.toughness.unwrap_or(0)) - i64::from(o.damage);
                if n * i64::from(toughness) >= left {
                    -self.removal(view, o)
                } else {
                    -sign * n * 100
                }
            }
            CounterKind::Plus { .. } | CounterKind::Minus { .. } => 0,
            CounterKind::Loyalty => sign * n * LOYALTY,
            CounterKind::Charge
            | CounterKind::Level
            | CounterKind::Energy
            | CounterKind::Lifelink => sign * n * 40,
            CounterKind::Poison | CounterKind::Rad => -sign * n * 50,
            // A clock's sign is the card's under it (`tactics::clock_score`),
            // and a custom counter means what its card says.
            CounterKind::Time | CounterKind::Lore | CounterKind::Custom(_) => return None,
        })
    }

    /// Damage to `o`: what finishing it is worth, and an eighth of that
    /// when it does not finish it.
    fn damage_object(&self, view: &PlayerView, o: &PublicObject, n: i64) -> i64 {
        let finishes =
            n >= crate::tactics::damage_to_finish(o) && !has(o, KeywordSet::INDESTRUCTIBLE);
        let removal = self.removal(view, o);
        if finishes { removal } else { removal / 8 }
    }

    /// One effect's worth at `aim`.
    #[allow(clippy::too_many_lines)] // the effect vocabulary stays in one auditable table
    fn effect_worth(
        &self,
        view: &PlayerView,
        origin: Origin<'_>,
        effect: &Effect,
        aim: Aim,
        x: u32,
    ) -> Option<i64> {
        let me = view.seat;
        let (then, otherwise) = effect.branches();
        if !then.is_empty() || !otherwise.is_empty() {
            let then = self.effects_worth(view, origin, then, aim, x)?;
            let otherwise = self.effects_worth(view, origin, otherwise, aim, x)?;
            return Some(match effect {
                Effect::Sequence(_) => then,
                // This seat's own "may": taken when it helps.
                Effect::MayDo { .. } | Effect::MayDoOnceEachTurn { .. } => then.max(0),
                // A condition, or somebody else's choice: either may happen.
                _ => i64::midpoint(then, otherwise),
            });
        }
        // What "it" is: the target when there is one, the source when the
        // ability targets nothing (the engine's `this_to_affect`), and
        // nothing at all for an "up to one" left empty.
        let subject = match aim {
            Aim::Object(id) => view.object(id),
            Aim::Source => origin.object,
            Aim::Nothing | Aim::Player(_) => None,
        };
        let count = |amount: Amount| self.count(view, origin, amount, aim, x);
        let on_board = |filter: &Filter| -> Option<Vec<&PublicObject>> {
            view.battlefield.iter().try_fold(Vec::new(), |mut all, o| {
                if self.filter_matches(filter, view, o, ZoneRef::Battlefield, Some(origin.id))? {
                    all.push(o);
                }
                Some(all)
            })
        };
        let hostile_subject = subject.is_some_and(|o| self.hostile(o.controller, me));
        Some(match effect {
            Effect::DrawCards { amount } => self.draw_worth(view, me, count(*amount)?),
            Effect::DrawCardsFor { amount, who } => {
                let n = count(*amount)?;
                self.players(view, *who, aim)?
                    .into_iter()
                    .map(|p| self.draw_worth(view, p, n))
                    .sum()
            }
            Effect::PutFromHandOnTop { count } => -i64::from(*count) * 250,
            Effect::GainLife { amount } => count(*amount)? * life_price(view, me),
            Effect::GainLifeFor { amount, who } => {
                let n = count(*amount)?;
                self.players(view, *who, aim)?
                    .into_iter()
                    .map(|p| {
                        let gain = n * life_price(view, p);
                        if self.hostile(p, me) { -gain } else { gain }
                    })
                    .sum()
            }
            Effect::LoseLife { amount, target } => {
                let n = count(*amount)?;
                self.players(view, *target, aim)?
                    .into_iter()
                    .map(|p| {
                        let loss = hurt(view, p, n);
                        if self.hostile(p, me) { loss } else { -loss }
                    })
                    .sum()
            }
            // Keeping or binning what is looked at needs a seat that reads
            // cards; one that does not keeps everything, which changes
            // nothing.
            Effect::Scry { amount }
            | Effect::Surveil { amount }
            | Effect::ScryFor { amount, .. } => {
                if self.profile.mulligan_skill >= 2 {
                    count(*amount)? * 40
                } else {
                    0
                }
            }
            Effect::ReorderTopLibrary { count } => {
                if self.profile.mulligan_skill >= 2 {
                    i64::from(*count) * 20
                } else {
                    0
                }
            }
            Effect::CreateToken { token } => token_worth(token),
            Effect::CreateTokenN { token, amount } => count(*amount)? * token_worth(token),
            Effect::Amass { amount, .. } => 150 + i64::from(*amount) * 110,
            Effect::SearchLibrary { .. }
            | Effect::SearchLibraryUpTo { .. }
            | Effect::WishToHand { .. }
            | Effect::LookAtTopPick { .. }
            | Effect::RevealTopAndSort { .. }
            | Effect::LookAtTopMayPut { .. } => 350,
            Effect::TakeExtraTurn => 10_000,
            Effect::ExileLibraryAndShuffleHand { player } => {
                if self
                    .players(view, *player, aim)?
                    .iter()
                    .any(|p| self.hostile(*p, me))
                {
                    10_000
                } else {
                    -10_000
                }
            }
            Effect::CreateEmblem { .. } => 1_000,
            Effect::Destroy { .. } => subject.map_or(0, |o| {
                if has(o, KeywordSet::INDESTRUCTIBLE) {
                    0
                } else {
                    self.removal(view, o)
                }
            }),
            Effect::DestroyOthersNamedLike { .. } => subject.map_or(0, |target| {
                view.battlefield
                    .iter()
                    .filter(|o| {
                        o.id != target.id
                            && o.name == target.name
                            && !has(o, KeywordSet::INDESTRUCTIBLE)
                    })
                    .map(|o| self.removal(view, o))
                    .sum()
            }),
            Effect::Exile { .. } => subject.map_or(0, |o| {
                if view.battlefield.iter().any(|b| b.id == o.id) {
                    self.removal(view, o)
                } else if self.hostile(o.owner, me) {
                    30
                } else {
                    -30
                }
            }),
            Effect::PutTargetOnBottomOfLibrary => subject.map_or(0, |o| self.removal(view, o)),
            Effect::ReturnToHand { .. } => subject.map_or(0, |o| self.bounce(view, o)),
            Effect::Blink { .. } | Effect::ExileAndReturnAtEndStep => {
                subject.map_or(0, |o| self.blink(view, o))
            }
            // Back from a graveyard under this seat's control: the body and
            // what it does on the way in.
            Effect::GraveyardToBattlefield { owner_control, .. } => subject.map_or(0, |o| {
                let worth = Self::permanent_worth(view, o) + self.etb_worth(view, o);
                if *owner_control && self.hostile(o.owner, me) {
                    -worth
                } else {
                    worth
                }
            }),
            Effect::GraveyardToHand { .. } => {
                subject.map_or(0, |o| if self.hostile(o.owner, me) { -350 } else { 350 })
            }
            Effect::AttachSelf { .. } => subject.map_or(0, |c| self.attach(view, origin, c)),
            Effect::UntapTarget => subject.map_or(0, |o| {
                let tapped = o.status.contains(ObjectStatus::TAPPED);
                match (hostile_subject, tapped) {
                    (_, false) => 0,
                    (true, true) => -20,
                    (false, true) if makes_mana(o) => 40,
                    (false, true) => 30,
                }
            }),
            Effect::TapTarget => subject.map_or(0, |o| {
                if o.status.contains(ObjectStatus::TAPPED) || !creature(o) {
                    0
                } else if hostile_subject {
                    30 + i64::from(o.power.unwrap_or(0).max(0)) * 30
                } else {
                    -(30 + i64::from(o.power.unwrap_or(0).max(0)) * 30)
                }
            }),
            Effect::CreateContinuousEffect {
                filter,
                modifier,
                duration,
                ..
            } => {
                let objects: Vec<&PublicObject> = if matches!(filter, Filter::This) {
                    subject.into_iter().collect()
                } else {
                    on_board(filter)?
                };
                match modifier {
                    Modifier::ModifyPT(p, t) => {
                        self.pump(view, &objects, (i32::from(*p), i32::from(*t), 0), *duration)
                    }
                    Modifier::AddKeyword(k) => {
                        self.pump(view, &objects, (0, 0, k.bits()), *duration)
                    }
                    Modifier::PreventDamageToIt => {
                        objects.iter().map(|o| self.prevent(view, o, false)).sum()
                    }
                    Modifier::PreventDamageFromIt => {
                        objects.iter().map(|o| self.prevent(view, o, true)).sum()
                    }
                    // A type, a colour or a subtype changes what other
                    // cards say about the object and nothing by itself.
                    Modifier::AddType(_)
                    | Modifier::RemoveType(_)
                    | Modifier::AddSubtype(_)
                    | Modifier::AllCreatureTypes
                    | Modifier::AddColor(_)
                    | Modifier::SetColor(_) => 0,
                    _ => return None,
                }
            }
            Effect::PumpTarget {
                power,
                toughness,
                keywords,
                duration,
            } => {
                let objects: Vec<&PublicObject> = subject.into_iter().collect();
                let p = i32::try_from(count(*power)?).unwrap_or(0);
                let t = i32::try_from(count(*toughness)?).unwrap_or(0);
                self.pump(view, &objects, (p, t, keywords.bits()), *duration)
            }
            Effect::PumpFilter {
                filter,
                controlled_by,
                power,
                toughness,
                keywords,
                duration,
            } => {
                let owners = match controlled_by {
                    Some(rel) => Some(self.players(view, *rel, aim)?),
                    None => None,
                };
                let objects: Vec<&PublicObject> = on_board(filter)?
                    .into_iter()
                    .filter(|o| {
                        owners
                            .as_ref()
                            .is_none_or(|owners| owners.contains(&o.controller))
                    })
                    .collect();
                let p = i32::try_from(count(*power)?).unwrap_or(0);
                let t = i32::try_from(count(*toughness)?).unwrap_or(0);
                self.pump(view, &objects, (p, t, keywords.bits()), *duration)
            }
            Effect::AddCounter { kind, amount } => match subject {
                Some(o) => self.counters(view, o, *kind, count(*amount)?)?,
                None => 0,
            },
            Effect::AddCountersUpTo {
                kind,
                amount,
                maximum,
            } => match origin.object {
                Some(o) => {
                    let existing: u32 = o
                        .counters
                        .iter()
                        .filter(|entry| entry.kind == counter_view(*kind))
                        .map(|entry| u32::from(entry.count))
                        .sum();
                    let room = u32::from(*maximum).saturating_sub(existing);
                    self.counters(view, o, *kind, count(*amount)?.min(i64::from(room)))?
                        .max(0)
                }
                None => 0,
            },
            Effect::AddCounterFilter {
                filter,
                kind,
                amount,
            } => {
                let n = count(*amount)?;
                on_board(filter)?
                    .into_iter()
                    .map(|o| self.counters(view, o, *kind, n))
                    .sum::<Option<i64>>()?
            }
            // At least the damage exchange is known; prevention and replacement
            // effects can reduce the incidental life gain, so do not rely on it.
            Effect::DealDamage { amount, .. }
            | Effect::DealDamageEvenly { amount, .. }
            | Effect::DealDamageWithCappedLifeGain { amount } => {
                let n = count(*amount)?;
                match aim {
                    Aim::Player(p) => {
                        let d = hurt(view, p, n);
                        if self.hostile(p, me) { d } else { -d }
                    }
                    _ => subject.map_or(0, |o| self.damage_object(view, o, n)),
                }
            }
            // Every creature goes home to its owner (CR 108.3).
            Effect::AllCreaturesToOwner => view
                .battlefield
                .iter()
                .filter(|o| creature(o) && o.owner != o.controller)
                .map(|o| {
                    let worth = 2 * Self::permanent_worth(view, o);
                    if o.owner == me {
                        worth
                    } else if o.controller == me {
                        -worth
                    } else {
                        0
                    }
                })
                .sum(),
            Effect::CounterTargetSpell
            | Effect::CounterTargetSpellToExile
            | Effect::CounterTargetAbility
            | Effect::CounterTargetSpellOrAbility => subject.map_or(0, |o| {
                if !self.hostile(o.controller, me) {
                    return -(200 + i64::from(o.mana_value) * 100);
                }
                if !crate::tactics::counterable(o) || crate::tactics::only_replaces_itself(o) {
                    return 0;
                }
                200 + i64::from(o.mana_value) * 100 + self.aimed_at_this_seat(view, o)
            }),
            Effect::SacrificeSelf | Effect::ExileSource => {
                -origin.object.map_or(0, |o| Self::permanent_worth(view, o))
            }
            // Back on top: the card comes again next draw, for its price.
            Effect::PutSourceOnTopOfLibrary => {
                -(origin.object.map_or(0, |o| i64::from(o.mana_value)) * 60 + 60)
            }
            Effect::Mill { amount, target } => {
                let n = count(*amount)?;
                self.players(view, *target, aim)?
                    .into_iter()
                    .map(|p| if self.hostile(p, me) { n * 10 } else { -n * 10 })
                    .sum()
            }
            Effect::AddMana { .. } => 0,
            _ => return None,
        })
    }

    /// What paying `cost` from `origin` gives up now; `None` for a cost this
    /// seat cannot price or cannot pay.
    ///
    /// Exhaustive with no wildcard, as [`crate::activate`]'s `consumes` is:
    /// a new cost part is a compile error here, not a free price.
    pub(crate) fn cost_worth(
        &self,
        view: &PlayerView,
        origin: Origin<'_>,
        cost: &Cost,
    ) -> Option<i64> {
        let me = view.seat;
        let mut total = i64::from(cost.mana.cmc()) * Self::mana_price(view);
        let own = |filter: &Filter, keep: &dyn Fn(&PublicObject) -> bool| -> Vec<&PublicObject> {
            view.battlefield_of(me)
                .filter(|o| {
                    keep(o)
                        && self.filter_matches(
                            filter,
                            view,
                            o,
                            ZoneRef::Battlefield,
                            Some(origin.id),
                        ) == Some(true)
                })
                .collect()
        };
        let card_value = Self::card_value(view);
        for part in cost.parts {
            total += match part {
                CostPart::TapSelf => match origin.object {
                    Some(o) if creature(o) => self.creature_tap_price(view, o)?,
                    Some(o) if makes_mana(o) => Self::mana_price(view),
                    _ => 0,
                },
                CostPart::UntapSelf | CostPart::PayLifeX => 0,
                CostPart::SacrificeSelf => {
                    let o = origin.object?;
                    // What is spent is the permanent as a source of mana or
                    // a body; its other uses are the one being made.
                    let worth = if o.types.contains(TypeSet::LAND) && !creature(o) {
                        land_worth(view, o, false)
                    } else {
                        Self::permanent_worth(view, o)
                    };
                    self.spent(view, o, worth)
                }
                CostPart::Sacrifice(filter) => own(filter, &|_| true)
                    .into_iter()
                    .map(|o| self.given_up(view, o))
                    .min()?,
                CostPart::PayLife(n) => {
                    if !crate::activate::life_ok(view, cost) {
                        return None;
                    }
                    i64::from(*n) * life_price(view, me)
                }
                CostPart::Discard(filter) | CostPart::ExileFromHand(filter) => view
                    .hand
                    .iter()
                    .filter(|h| h.id != origin.id && hand_matches(filter, h) == Some(true))
                    .map(|h| card_value(&h.id).max(0) + 50)
                    .min()?,
                CostPart::DiscardSelf => card_value(&origin.id).max(0),
                CostPart::ExileSelf => origin
                    .object
                    .filter(|o| view.battlefield.iter().any(|b| b.id == o.id))
                    .map_or(50, |o| Self::permanent_worth(view, o)),
                CostPart::ReturnSelfToHand => origin
                    .object
                    .map_or(50, |o| i64::from(o.mana_value) * 60 + 50),
                CostPart::RemoveCounterSelf { .. } | CostPart::RemoveCounterSelfX { .. } => 40,
                CostPart::PutCounterSelf { .. } | CostPart::ExileFromGraveyard(_) => 20,
                CostPart::TapOther(filter) => own(filter, &|o| {
                    o.id != origin.id && !o.status.contains(ObjectStatus::TAPPED)
                })
                .into_iter()
                .filter_map(|o| {
                    if creature(o) {
                        self.creature_tap_price(view, o)
                    } else if makes_mana(o) {
                        Some(Self::mana_price(view))
                    } else {
                        Some(0)
                    }
                })
                .min()?,
                CostPart::Crew(_) => 80,
                CostPart::ReturnToHand(filter) => own(filter, &|_| true)
                    .into_iter()
                    .map(|o| i64::from(o.mana_value) * 60 + 50)
                    .min()?,
            };
        }
        Some(total)
    }

    /// A mana this seat spends now, by what else it could have done.
    ///
    /// The priority ladder asks after spells, so what is left is mana no
    /// spell here wants: worth keeping up while an instant is held, worth
    /// little on an opponent's turn, and worth nothing in an end step,
    /// where it is about to empty unused.
    pub(crate) fn mana_price(view: &PlayerView) -> i64 {
        if matches!(view.step, Step::End | Step::Cleanup) {
            return 5;
        }
        let answers = view.hand.iter().any(|c| {
            c.mana_value > 0
                && (c.types.contains(TypeSet::INSTANT)
                    || baylee_cards::by_index(c.card.index).is_some_and(|d| {
                        d.keywords_for_face(usize::from(c.card.face))
                            .contains(KeywordSet::FLASH)
                    }))
        });
        if answers {
            120
        } else if view.active != view.seat {
            10
        } else {
            40
        }
    }

    /// What tapping this seat's creature `o` gives up now; `None` before
    /// its attack, where the answer used to be "wait until after combat" and
    /// still is.
    ///
    /// After this seat's combat it stays tapped through the next opponent's
    /// turn, so the price is a blocker; on an opponent's turn it is a
    /// blocker until blocks are declared, and nothing after.
    fn creature_tap_price(&self, view: &PlayerView, o: &PublicObject) -> Option<i64> {
        let mine = view.active == view.seat;
        let before_combat = matches!(view.phase, Phase::Beginning | Phase::FirstMain)
            || view.phase == Phase::Combat && view.step == Step::CombatBegin;
        if mine && before_combat {
            return None;
        }
        let blocks_ahead = mine
            || before_combat
            || view.phase == Phase::Combat && view.step == Step::DeclareAttackers;
        let threat = view.battlefield.iter().any(|c| {
            self.hostile(c.controller, view.seat) && creature(c) && c.power.unwrap_or(0) > 0
        });
        Some(if blocks_ahead && threat {
            60 + i64::from(o.power.unwrap_or(0).max(0)) * 30
                + i64::from(o.toughness.unwrap_or(0).max(0)) * 20
        } else {
            0
        })
    }

    /// The answer to an activated or loyalty ability's target question, by
    /// the measure that chose to activate it. `None` for every other
    /// question, and for one whose candidates this measure cannot read.
    pub(crate) fn ability_targets(
        &self,
        view: &PlayerView,
        offer: crate::tactics::Offer<'_>,
        context: &baylee_engine::engine::DecisionContext<'_>,
    ) -> Option<baylee_engine::choice::PlayerAction> {
        let printed = context.printed?;
        let def = baylee_cards::by_index(printed.card())?
            .abilities_for_face(usize::from(printed.face()))
            .get(usize::try_from(context.ability_index?).ok()?)?;
        if context.second_instance
            || !matches!(
                def,
                AbilityDef::Activated { .. }
                    | AbilityDef::ActivatedConditional { .. }
                    | AbilityDef::Loyalty { .. }
            )
        {
            return None;
        }
        let origin = Origin::of(view, context.source?);
        let mut scored: Vec<(i64, Aim)> = offer
            .objects
            .iter()
            .map(|id| Aim::Object(*id))
            .chain(offer.players.iter().map(|p| Aim::Player(*p)))
            .filter_map(|aim| {
                Some((
                    self.effects_worth(view, origin, context.effects, aim, context.x)?,
                    aim,
                ))
            })
            .collect();
        if scored.is_empty() {
            return None;
        }
        scored.sort_by_key(|&(worth, aim)| (std::cmp::Reverse(worth), aim_key(aim)));
        let wanted = scored
            .iter()
            .take_while(|(worth, _)| *worth > 0)
            .count()
            .clamp(
                usize::try_from(offer.min).unwrap_or(usize::MAX),
                usize::try_from(offer.max).unwrap_or(usize::MAX),
            );
        if wanted > scored.len() {
            return None;
        }
        let mut objects = Vec::new();
        let mut players = Vec::new();
        for (_, aim) in scored.into_iter().take(wanted) {
            match aim {
                Aim::Object(id) => objects.push(id),
                Aim::Player(p) => players.push(p),
                Aim::Source | Aim::Nothing => {}
            }
        }
        Some(baylee_engine::choice::PlayerAction::ChooseTargets { objects, players })
    }
}

/// A stable order for ties: objects before players, each by handle.
fn aim_key(aim: Aim) -> (u8, u64) {
    match aim {
        Aim::Object(id) => (0, u64::from(id.slot())),
        Aim::Player(p) => (1, u64::from(p.get())),
        Aim::Source | Aim::Nothing => (2, 0),
    }
}

/// A land to whoever controls it: a mana a turn, worth more the fewer lands
/// they have, and — with `uses` — more again for doing anything besides.
fn land_worth(view: &PlayerView, o: &PublicObject, uses: bool) -> i64 {
    let lands = view
        .battlefield_of(o.controller)
        .filter(|l| l.types.contains(TypeSet::LAND))
        .count();
    let scarcity = match lands {
        0..=3 => 300,
        4 => 200,
        5 => 100,
        _ => 0,
    };
    let utility = uses
        && printed_list(o).iter().any(|a| match a {
            AbilityDef::Activated { mana_ability, .. }
            | AbilityDef::ActivatedConditional { mana_ability, .. } => !mana_ability,
            AbilityDef::Triggered { .. } | AbilityDef::Static(_) => true,
            _ => false,
        });
    150 + scarcity + if utility { 250 } else { 0 }
}

/// A token as a permanent: its body, or a small floor for one without.
fn token_worth(token: &baylee_cards_dsl::TokenDef) -> i64 {
    let body = i64::from(token.power.unwrap_or(0).max(0)) * 110
        + i64::from(token.toughness.unwrap_or(0).max(0)) * 30;
    150 + body.max(if token.abilities.is_empty() { 0 } else { 100 })
}

/// Whether a card in hand matches a cost's filter, for the filters a hand
/// card can answer from its types alone; `None` for anything else.
fn hand_matches(filter: &Filter, h: &baylee_view::HandObject) -> Option<bool> {
    Some(match filter {
        Filter::Any => true,
        Filter::HasType(t) => h.types.intersects(*t),
        Filter::LacksType(t) => !h.types.intersects(*t),
        Filter::And(parts) => {
            for f in *parts {
                if !hand_matches(f, h)? {
                    return Some(false);
                }
            }
            true
        }
        Filter::Or(parts) => {
            for f in *parts {
                if hand_matches(f, h)? {
                    return Some(true);
                }
            }
            false
        }
        Filter::Not(inner) => !hand_matches(inner, h)?,
        _ => return None,
    })
}
