//! Effect-based choices, shared by targeting, planeswalkers and deck analysis.
//! Unknown effects retain the general policy; new cards need no name table.

use baylee_cards_dsl::{Amount, CounterKind, Effect, Modifier};
use baylee_core::ids::{ObjectId, PlayerId, SubtypeId};
use baylee_core::types::TypeSet;
use baylee_engine::choice::PlayerAction;
use baylee_engine::engine::DecisionContext;
use baylee_view::{PlayerView, PublicObject};

use crate::HeuristicAgent;

#[derive(Clone, Copy, Default)]
pub(crate) struct Meaning {
    pub benefit: i32,
    pub damage: u32,
    pub removal: bool,
    pub destroy: bool,
    pub counter: bool,
    pub draw: u32,
    pub value: i64,
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
pub(crate) fn meaning(effects: &[Effect], x: u32) -> Meaning {
    let mut result = Meaning::default();
    for effect in effects {
        let mut m = Meaning::default();
        match effect {
            Effect::Sequence(inner) | Effect::MayDo { effects: inner } => m = meaning(inner, x),
            Effect::AddCounter { kind, .. } => {
                m.benefit = match kind {
                    CounterKind::Minus { .. } | CounterKind::Poison | CounterKind::Rad => -1,
                    CounterKind::Plus { .. }
                    | CounterKind::Loyalty
                    | CounterKind::Lifelink
                    | CounterKind::Energy
                    | CounterKind::Charge
                    | CounterKind::Level => 1,
                    CounterKind::Time | CounterKind::Lore | CounterKind::Custom(_) => 0,
                }
            }
            Effect::PumpTarget {
                power, toughness, ..
            } => {
                m.benefit = amount(*power, x)
                    .saturating_add(amount(*toughness, x))
                    .signum();
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
            | Effect::GraveyardToTop { .. } => m.benefit = 1,
            Effect::TapTarget => m.benefit = -1,
            Effect::DrawCards { amount: n } => {
                m.draw = u32::try_from(amount(*n, x)).unwrap_or(0);
                m.value = i64::from(m.draw) * 400;
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
        result.benefit += m.benefit;
        result.damage = result.damage.saturating_add(m.damage);
        result.draw = result.draw.saturating_add(m.draw);
        result.value += m.value;
        result.removal |= m.removal;
        result.destroy |= m.destroy;
        result.counter |= m.counter;
    }
    result
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
    pub(crate) fn subtype(view: &PlayerView, options: &[SubtypeId]) -> SubtypeId {
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
                    hand * 3 + board * 2 + commanders * 4,
                    std::cmp::Reverse(*subtype),
                )
            })
            .unwrap_or(options[0])
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
        if m.benefit == 0 && m.damage == 0 {
            return None;
        }
        let beneficial = m.benefit > 0;
        let mut ranked: Vec<(i64, Option<ObjectId>, Option<PlayerId>)> = objects
            .iter()
            .map(|id| {
                let value = view.object(*id).map_or(0, |o| {
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
                    if friendly == beneficial {
                        score
                    } else {
                        -score - 10_000
                    }
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
