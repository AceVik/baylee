//! Effect-based choices, shared by targeting, planeswalkers and deck analysis.
//! Unknown effects retain the general policy; new cards need no name table.

use baylee_cards_dsl::{AbilityDef, Amount, CounterKind, Effect, Modifier};
use baylee_core::ids::{ObjectId, PlayerId, SubtypeId};
use baylee_core::types::TypeSet;
use baylee_engine::choice::PlayerAction;
use baylee_engine::engine::DecisionContext;
use baylee_view::{CounterKind as ViewCounter, PlayerView, PublicObject};

use crate::HeuristicAgent;

#[derive(Clone, Copy, Default)]
pub(crate) struct Meaning {
    pub benefit: i32,
    pub damage: u32,
    pub removal: bool,
    pub destroy: bool,
    pub counter: bool,
    pub draw: u32,
    commander_draws: u32,
    pub value: i64,
    /// A counter kind whose sign is not its own: see [`meaning`]'s
    /// `AddCounter` arm. Scored per candidate object by
    /// [`HeuristicAgent::clock_score`] instead of by `benefit`.
    pub clock: Option<CounterKind>,
}

impl Meaning {
    pub(crate) fn draws(self, view: &PlayerView) -> u32 {
        let casts = view
            .seat(view.seat)
            .map_or(0, |s| s.commanders.iter().map(|c| c.casts).sum::<u32>());
        self.draw
            .saturating_add(self.commander_draws.saturating_mul(casts))
    }
}

fn amount(n: Amount, x: u32) -> i32 {
    match n {
        Amount::Fixed(n) => i32::try_from(n).unwrap_or(i32::MAX),
        Amount::X | Amount::XPlusCommanderCasts => i32::try_from(x).unwrap_or(i32::MAX),
        Amount::DoubleX => i32::try_from(x.saturating_mul(2)).unwrap_or(i32::MAX),
        Amount::NegX => -i32::try_from(x).unwrap_or(i32::MAX),
        Amount::NegXFixed(n) => -i32::try_from(n).unwrap_or(i32::MAX),
        _ => 0,
    }
}

/// A deliberately partial evaluation vocabulary. Unsupported effects are not
/// mistaken for removal; the coverage ledger records what remains to model.
#[allow(clippy::too_many_lines)] // the partial effect vocabulary stays in one auditable table
pub(crate) fn meaning(effects: &[Effect], x: u32) -> Meaning {
    let mut result = Meaning::default();
    for effect in effects {
        let mut m = Meaning::default();
        // What an effect runs inside itself is [`Effect::branches`]' to say,
        // not this table's. It used to name `Sequence` and `MayDo` and stop
        // there, so removal printed inside a kicker clause or behind "unless
        // you pay" read as no meaning at all — and an effect with no meaning
        // is one `targets` declines to express a preference about.
        let (then, otherwise) = effect.branches();
        if !then.is_empty() || !otherwise.is_empty() {
            m = meaning(then, x);
            m.absorb(meaning(otherwise, x));
        }
        match effect {
            Effect::AddCounter { kind, .. } => match kind {
                CounterKind::Minus { .. } | CounterKind::Poison | CounterKind::Rad => {
                    m.benefit = -1;
                }
                CounterKind::Plus { .. }
                | CounterKind::Loyalty
                | CounterKind::Lifelink
                | CounterKind::Energy
                | CounterKind::Charge
                | CounterKind::Level => m.benefit = 1,
                // These two have no sign of their own. A lore counter
                // advances whatever Saga it lands on and a time counter
                // delays whatever is counting down, so both are good for one
                // seat and bad for another depending on the card underneath —
                // which `meaning` cannot see, because it is handed an effect
                // list and no object. Saying 0 here is what made the agent
                // decline to express a preference at all: `targets` bailed on
                // a zero benefit and the fallback aimed at an opponent, which
                // is right for almost every other spell and hands an opponent
                // their next chapter.
                CounterKind::Time | CounterKind::Lore => m.clock = Some(*kind),
                // A custom counter is named by the card that prints it and
                // means whatever that card says; there is no rule to read.
                CounterKind::Custom(_) => {}
            },
            Effect::PumpTarget {
                power,
                toughness,
                keywords,
                ..
            } => {
                m.benefit = amount(*power, x)
                    .saturating_add(amount(*toughness, x))
                    .signum();
                if m.benefit == 0
                    && keywords.bits() & !baylee_cards_dsl::KeywordSet::DEFENDER.bits() != 0
                {
                    m.benefit = 1;
                }
            }
            Effect::CreateContinuousEffect { modifier, .. } => {
                m.benefit = match modifier {
                    Modifier::ModifyPT(p, t) => (i32::from(*p) + i32::from(*t)).signum(),
                    Modifier::AddKeyword(_) => 1,
                    _ => 0,
                }
            }
            Effect::Destroy { .. } => {
                m.removal = true;
                m.destroy = true;
                m.benefit = -1;
            }
            Effect::Exile { .. }
            | Effect::ReturnToHand { .. }
            | Effect::PutTargetOnBottomOfLibrary
            | Effect::ChangeController { .. } => {
                m.removal = true;
                m.benefit = -1;
            }
            Effect::CounterTargetSpell
            | Effect::CounterTargetSpellToExile
            | Effect::CounterTargetAbility
            | Effect::CounterTargetSpellOrAbility => {
                m.counter = true;
                m.benefit = -1;
            }
            Effect::DealDamage { amount: n, .. } => {
                m.damage = u32::try_from(amount(*n, x)).unwrap_or(0);
                m.benefit = -1;
            }
            Effect::Blink { .. }
            | Effect::ExileAndReturnAtEndStep
            | Effect::UntapTarget
            | Effect::GrantFlashback
            | Effect::GraveyardToHand { .. }
            | Effect::GraveyardToTop { .. }
            | Effect::GainLifeFor {
                who: baylee_cards_dsl::PlayerRel::Chosen,
                ..
            } => m.benefit = 1,
            Effect::TapTarget => m.benefit = -1,
            Effect::DrawCards { amount: n } | Effect::DrawCardsFor { amount: n, .. } => {
                m.draw = u32::try_from(amount(*n, x)).unwrap_or(0);
                m.commander_draws = u32::from(matches!(n, Amount::XPlusCommanderCasts));
                m.value = i64::from(m.draw) * 400;
                if matches!(
                    effect,
                    Effect::DrawCardsFor {
                        who: baylee_cards_dsl::PlayerRel::Chosen,
                        ..
                    }
                ) {
                    m.benefit = 1;
                }
            }
            Effect::PutFromHandOnTop { count } => m.value = -i64::from(*count) * 250,
            Effect::Scry { .. } | Effect::ScryFor { .. } | Effect::Surveil { .. } => m.value = 140,
            Effect::CreateToken { .. } | Effect::CreateTokenN { .. } | Effect::Amass { .. } => {
                m.value = 500;
            }
            Effect::TakeExtraTurn | Effect::ExileLibraryAndShuffleHand { .. } => m.value = 10_000,
            Effect::SearchLibrary { .. } | Effect::LookAtTopPick { .. } => m.value = 350,
            Effect::GainLife { .. } => m.value = 80,
            _ => {}
        }
        result.absorb(m);
    }
    result
}

impl Meaning {
    /// Fold another reading into this one.
    ///
    /// One method rather than a block per caller, because there are two of
    /// them now: the effect list's accumulator, and the two branches of an
    /// effect that carries both. A merge written twice is a field that gets
    /// added to one of them.
    fn absorb(&mut self, other: Meaning) {
        self.benefit += other.benefit;
        self.damage = self.damage.saturating_add(other.damage);
        self.draw = self.draw.saturating_add(other.draw);
        self.commander_draws = self.commander_draws.saturating_add(other.commander_draws);
        self.value += other.value;
        self.removal |= other.removal;
        self.destroy |= other.destroy;
        self.counter |= other.counter;
        // The first clock wins. A spell that both advances a Saga and delays
        // a suspended card prints two sentences and would need two target
        // choices; one effect list answering one prompt has one.
        self.clock = self.clock.or(other.clock);
    }
}

pub(crate) fn material(o: &PublicObject) -> i64 {
    200 + i64::from(o.mana_value) * 70
        + i64::from(o.power.unwrap_or(0).max(0)) * 110
        + i64::from(o.toughness.unwrap_or(0).max(0)) * 30
        + if o.commander { 200 } else { 0 }
        + if o.types.contains(TypeSet::PLANESWALKER) {
            500
        } else {
            0
        }
}

impl HeuristicAgent {
    pub(crate) fn subtype(&self, view: &PlayerView, options: &[SubtypeId]) -> SubtypeId {
        options
            .iter()
            .copied()
            .max_by_key(|subtype| {
                let hand = view
                    .hand
                    .iter()
                    .filter_map(|c| crate::policy::face(c.card))
                    .filter(|f| f.types.contains(TypeSet::CREATURE) && f.subtypes.contains(subtype))
                    .count();
                let board = view
                    .battlefield_of(view.seat)
                    .filter(|o| {
                        o.types.contains(TypeSet::CREATURE) && o.subtypes.contains(*subtype)
                    })
                    .count();
                let commanders = view
                    .command
                    .iter()
                    .flatten()
                    .filter(|o| {
                        o.controller == view.seat
                            && o.types.contains(TypeSet::CREATURE)
                            && o.subtypes.contains(*subtype)
                    })
                    .count();
                (
                    hand * 3
                        + board * 2
                        + commanders * 4
                        + usize::from(self.strategy.tribe == Some(*subtype)) * 2,
                    std::cmp::Reverse(*subtype),
                )
            })
            .unwrap_or(options[0])
    }

    /// What targeting this permanent costs on top of the spell, because it
    /// has ward (CR 702.21).
    ///
    /// Ward triggers on a spell an **opponent** controls, so it is only ever
    /// asked of a candidate on the other side of the table, and it is asked
    /// here rather than left to the tax question because by then the spell is
    /// already on the stack: a target chosen into a ward the seat cannot pay
    /// is the whole card, and choosing the other creature costs nothing.
    ///
    /// The two answers are a toll and a refusal. A payable ward is priced —
    /// `mana` against the material scale, so a bigger threat is still worth
    /// the tax and an equal one is not — while an unpayable ward sinks the
    /// candidate below everything on offer without sending it past the band
    /// that holds a wrong-side target, because aiming removal at the agent's
    /// own creature to dodge a ward is not an improvement.
    ///
    /// Both halves of the price are read together: CR 601.2 chooses targets
    /// before mana is paid, so the seat has to cover the spell **and** the
    /// tax out of what is untapped now, and `DecisionContext` carries the
    /// spell's own cost at exactly this moment.
    fn ward_priced(
        view: &PlayerView,
        object: &PublicObject,
        context: &DecisionContext<'_>,
        score: i64,
    ) -> i64 {
        let ward = object
            .card
            .and_then(|c| baylee_cards::by_index(c.index))
            .and_then(|def| {
                def.abilities
                    .iter()
                    .chain(def.faces.iter().flat_map(|face| face.abilities.iter()))
                    .find_map(|ability| match ability {
                        AbilityDef::Ward { mana } => Some(*mana),
                        _ => None,
                    })
            });
        let Some(mana) = ward else {
            return score;
        };
        let whole = context
            .cost
            .unwrap_or(baylee_core::mana::ManaCost::ZERO)
            .with_more_generic(u32::from(mana));
        if crate::policy::can_pay(view, &whole) {
            score - i64::from(mana) * 60
        } else {
            // A floor rather than a penalty: every untaxed candidate
            // outranks this one, and it still outranks the -10_000 band a
            // wrong-side target sits in.
            -5_000
        }
    }

    /// What a counter with no sign of its own is worth on one candidate.
    ///
    /// The sign comes off the card underneath. A lore counter advances the
    /// Saga it lands on, which is what that Saga's controller wants; a time
    /// counter is one more upkeep before a suspended card casts itself for
    /// nothing, which is what its owner does not. Positive means "aim here",
    /// and the scale is the one the ranking beside it uses: the count is
    /// taken while the score is above zero, so a negative score is a
    /// candidate the agent names only because the spell requires a target.
    ///
    /// Two shapes are refused rather than guessed, and both are scored 0.
    /// A lore counter that would reach the Saga's final chapter is that
    /// chapter *and* the Saga's death in one — the controller gets the last
    /// ability and loses the permanent, and which of those is worth more is
    /// not a judgment this table can make. And vanishing, the other user of
    /// time counters, runs the opposite way from suspend: a counter added
    /// there buys the permanent another turn. No card in this pool prints
    /// vanishing, so the case is left unscored instead of modelled blind.
    fn clock_score(&self, view: &PlayerView, object: &PublicObject, kind: CounterKind) -> i64 {
        let friendly = !self.hostile(object.controller, view.seat);
        let def = object.card.and_then(|c| baylee_cards::by_index(c.index));
        let mut abilities = def.into_iter().flat_map(|d| {
            d.abilities
                .iter()
                .chain(d.faces.iter().flat_map(|face| face.abilities.iter()))
        });
        let on_it = |want: ViewCounter| {
            object
                .counters
                .iter()
                .find(|c| c.kind == want)
                .map_or(0, |c| c.count)
        };
        match kind {
            CounterKind::Lore => {
                let last = abilities
                    .filter_map(|a| match a {
                        AbilityDef::SagaChapter { chapter, .. } => Some(u16::from(*chapter)),
                        _ => None,
                    })
                    .max();
                match last {
                    // Not a Saga at all: a lore counter on it is a sentence
                    // no rule in this pool finishes.
                    None => 0,
                    Some(last) if on_it(ViewCounter::Lore).saturating_add(1) >= last => 0,
                    Some(_) if friendly => 800,
                    Some(_) => -800,
                }
            }
            CounterKind::Time => {
                let suspended = abilities.any(|a| matches!(a, AbilityDef::Suspend { .. }));
                let left = on_it(ViewCounter::Time);
                if !suspended || left == 0 {
                    return 0;
                }
                if friendly {
                    // Delaying my own free spell is never the play.
                    -800
                } else {
                    // Every candidate here is hostile, so what ranks them is
                    // how soon the card would otherwise cast itself: one
                    // counter left is one upkeep away, four is four.
                    1000 - 100 * i64::from(left.min(9))
                }
            }
            _ => 0,
        }
    }

    #[allow(clippy::too_many_arguments)] // the engine's target offer, plus its explanation
    pub(crate) fn targets(
        &self,
        view: &PlayerView,
        objects: &[ObjectId],
        players: &[PlayerId],
        min: u8,
        max: u8,
        context: &DecisionContext<'_>,
    ) -> Option<PlayerAction> {
        let m = meaning(context.effects, context.x);
        if m.benefit == 0 && m.damage == 0 && m.clock.is_none() {
            return None;
        }
        let beneficial = m.benefit > 0;
        let mut ranked: Vec<(i64, Option<ObjectId>, Option<PlayerId>)> = objects
            .iter()
            .map(|id| {
                let value = view.object(*id).map_or(0, |o| {
                    if let Some(kind) = m.clock {
                        return self.clock_score(view, o, kind);
                    }
                    let friendly = !self.hostile(o.controller, view.seat);
                    let mut score = material(o);
                    if m.destroy
                        && o.keywords & baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE.bits() != 0
                    {
                        score = 0;
                    }
                    if m.damage > 0
                        && i64::from(m.damage)
                            < i64::from(o.toughness.unwrap_or(0)) - i64::from(o.damage)
                    {
                        score /= 8;
                    }
                    if friendly != beneficial {
                        return -score - 10_000;
                    }
                    if friendly {
                        // Ward is an opponent's toll (CR 702.21) and never
                        // this seat's own.
                        return score;
                    }
                    Self::ward_priced(view, o, context, score)
                });
                (value, Some(*id), None)
            })
            .collect();
        ranked.extend(players.iter().map(|p| {
            let friendly = !self.hostile(*p, view.seat);
            let value = if friendly != beneficial {
                -20_000
            } else if m.damage > 0
                && view
                    .seat(*p)
                    .is_some_and(|s| i64::from(s.life) <= i64::from(m.damage))
            {
                1_000_000
            } else {
                100 + i64::from(m.damage) * 30
            };
            (value, None, Some(*p))
        }));
        ranked.sort_by_key(|&(value, object, player)| (std::cmp::Reverse(value), object, player));
        let count = ranked
            .iter()
            .take_while(|(score, _, _)| *score > 0)
            .count()
            .max(usize::from(min))
            .min(usize::from(max));
        let mut selected = Vec::new();
        let mut seats = Vec::new();
        for (_, object, player) in ranked.into_iter().take(count) {
            if let Some(o) = object {
                selected.push(o);
            }
            if let Some(p) = player {
                seats.push(p);
            }
        }
        Some(PlayerAction::ChooseTargets {
            objects: selected,
            players: seats,
        })
    }

    pub(crate) fn player_target(
        &self,
        view: &PlayerView,
        options: &[PlayerId],
        context: &DecisionContext<'_>,
    ) -> PlayerId {
        let m = meaning(context.effects, context.x);
        options
            .iter()
            .copied()
            .rev()
            .max_by_key(|&p| {
                let enemy = self.hostile(p, view.seat);
                if m.draws(view) > 0
                    && enemy
                    && view
                        .seat(p)
                        .is_some_and(|s| m.draws(view) > s.library_count)
                {
                    return 1_000_000;
                }
                if m.benefit > 0 {
                    if p == view.seat {
                        200
                    } else if enemy {
                        -100
                    } else {
                        100
                    }
                } else if enemy {
                    100
                } else {
                    0
                }
            })
            .unwrap_or(options[0])
    }

    pub(crate) fn effect_value(&self, view: &PlayerView, effects: &[Effect]) -> i64 {
        let m = meaning(effects, 1);
        if m.draw > 0
            && view
                .seat(view.seat)
                .is_some_and(|s| s.library_count <= m.draw)
        {
            return -100_000;
        }
        let target = if m.removal || m.damage > 0 {
            view.battlefield
                .iter()
                .filter(|o| {
                    self.hostile(o.controller, view.seat) && !o.types.contains(TypeSet::LAND)
                })
                .filter(|o| {
                    !m.destroy
                        || o.keywords & baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE.bits() == 0
                })
                .map(material)
                .max()
                .unwrap_or(-200)
        } else if m.benefit > 0 {
            200
        } else {
            0
        };
        m.value + target
    }
}
