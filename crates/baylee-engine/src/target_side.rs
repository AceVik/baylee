//! Which side of the table a target question's effect belongs on, read from
//! the rules of the effect and never from a card's name.
//!
//! One reading for everyone who needs it: the house's `meaning` table takes
//! its signs from [`effect_benefit`], and Encoder 10 reads the verdict through
//! `TargetingContext::side`. It says what an effect does to what it names and
//! nothing about whether naming that is a good idea.
//!
//! Deliberately partial, like the table it came from. An effect it cannot read
//! says nothing, and a question it has no side for reads as `None`:
//! a fight (two questions wanting opposite sides), a redirect or a copy (they
//! mean what the spell they turn means), a time or lore counter (the card under
//! it decides), and a hand attack that pays its victim back (Vendilion Clique
//! reads as a gift by its rider, and is played at oneself as well).
//!

use baylee_core::types::TargetSide;

use baylee_cards_dsl::{
    AbilityDef, Amount, CounterKind, Effect, Filter, KeywordSet, Modifier, PlayerRel,
};

/// A deliberately partial evaluator: an amount this side of the wire cannot
/// count answers zero rather than a guess.
///
/// The magnitude and the sign are read separately, and the sign is
/// [`Amount::is_negative`] rather than a list of the negative variants. This
/// was such a list — `NegX` and `NegXFixed` each with an arm of its own —
/// and it was the fourth copy of the same question in the workspace. The
/// next negative amount added to the DSL would have been read here as a
/// bonus, silently, in a heuristic nobody asserts a sign on.
#[must_use]
pub fn amount(n: Amount, x: u32) -> i32 {
    if let Amount::Negated(inner) = n {
        return -amount(*inner, x);
    }
    let magnitude = match n {
        Amount::Fixed(v) | Amount::NegXFixed(v) => i32::try_from(v).unwrap_or(i32::MAX),
        Amount::X | Amount::NegX | Amount::XPlusCommanderCasts | Amount::TimesKicked => {
            i32::try_from(x).unwrap_or(i32::MAX)
        }
        Amount::DoubleX => i32::try_from(x.saturating_mul(2)).unwrap_or(i32::MAX),
        _ => 0,
    };
    if n.is_negative() {
        -magnitude
    } else {
        magnitude
    }
}

/// Whether `effect` binds its own subjects or grants something for later:
/// its children do not act once on the current target, so they say nothing
/// about it.
#[must_use]
pub const fn acts_elsewhere(effect: &Effect) -> bool {
    matches!(
        effect,
        Effect::CreateDelayedObserver { .. }
            | Effect::DestroyAllThenForEachDestroyed { .. }
            | Effect::ForEachCreatureDamagedBySourceThatDiedThisTurn { .. }
            | Effect::ForEachTarget { .. }
            | Effect::WheneverCombatEventThisTurn { .. }
            | Effect::GrantTriggeredIfAbsent { .. }
    )
}

/// What `effect` itself does to what it names: `1` helps, `-1` hurts, `0` has
/// no sign of its own; `None` where this reading has no row, and what the
/// effect runs inside itself ([`Effect::branches`]) keeps its word.
#[must_use]
pub fn effect_benefit(effect: &Effect, x: u32) -> Option<i32> {
    match effect {
        Effect::AddCounter { kind, .. } => match kind {
            CounterKind::Minus { .. }
            | CounterKind::Poison
            | CounterKind::Rad
            | CounterKind::Stun
            | CounterKind::Finality => Some(-1),
            CounterKind::Plus { .. }
            | CounterKind::Loyalty
            | CounterKind::Lifelink
            | CounterKind::Flying
            | CounterKind::Indestructible
            | CounterKind::Hone
            | CounterKind::Shadow
            | CounterKind::Energy
            | CounterKind::Charge
            | CounterKind::Shield
            | CounterKind::Level => Some(1),
            // A lore or a time counter has no sign of its own, and a custom
            // one means whatever the card that prints it says.
            CounterKind::Time
            | CounterKind::Lore
            | CounterKind::Bore
            | CounterKind::Defense
            | CounterKind::Custom(_) => None,
        },
        Effect::PumpTarget {
            power,
            toughness,
            keywords,
            ..
        } => {
            let sign = amount(*power, x)
                .saturating_add(amount(*toughness, x))
                .signum();
            if sign == 0 && keywords.bits() & !KeywordSet::DEFENDER.bits() != 0 {
                Some(1)
            } else {
                Some(sign)
            }
        }
        Effect::CreateContinuousEffect { modifier, .. } => Some(match modifier {
            Modifier::ModifyPT(p, t) => (i32::from(*p) + i32::from(*t)).signum(),
            Modifier::AddKeyword(_) => 1,
            _ => 0,
        }),
        Effect::Destroy { .. }
        | Effect::DestroyTargetSlots { .. }
        | Effect::Exile { .. }
        | Effect::ReturnToHand { .. }
        | Effect::PutTargetOnBottomOfLibrary
        | Effect::ChangeController { .. }
        | Effect::CounterTargetSpell
        | Effect::CounterTargetSpellToExile
        | Effect::CounterTargetAbility
        | Effect::CounterTargetSpellOrAbility
        | Effect::DealDamage { .. }
        | Effect::DealDamageEvenly { .. }
        | Effect::DealDamageWithCappedLifeGain { .. }
        | Effect::DealDamageWithLimitedLifeGain { .. }
        | Effect::DealDamageDivided { .. }
        | Effect::TapTarget => Some(-1),
        Effect::Blink { .. }
        | Effect::ExileAndReturnAtEndStep
        | Effect::UntapTarget
        | Effect::GrantFlashback
        | Effect::GraveyardToHand { .. }
        | Effect::GraveyardToTop { .. }
        | Effect::GainLifeFor {
            who: PlayerRel::Chosen,
            ..
        }
        | Effect::DrawCardsFor {
            who: PlayerRel::Chosen,
            ..
        } => Some(1),
        _ => None,
    }
}

/// Whether `effect` hurts what it names whatever a rider gives back: Swords
/// to Plowshares exiles, and its life for the controller changes nothing.
fn hurts(effect: &Effect, x: u32) -> bool {
    match effect {
        Effect::Destroy { .. }
        | Effect::DestroyTargetSlots { .. }
        | Effect::Exile { .. }
        | Effect::ReturnToHand { .. }
        | Effect::PutTargetOnBottomOfLibrary
        | Effect::ChangeController { .. }
        | Effect::CounterTargetSpell
        | Effect::CounterTargetSpellToExile
        | Effect::CounterTargetAbility
        | Effect::CounterTargetSpellOrAbility => true,
        Effect::DealDamage { amount: n, .. }
        | Effect::DealDamageEvenly { amount: n, .. }
        | Effect::DealDamageWithCappedLifeGain { amount: n }
        | Effect::DealDamageWithLimitedLifeGain { amount: n, .. } => amount(*n, x) > 0,
        Effect::DealDamageDivided { amount: n } => n.total(x) > 0,
        _ => false,
    }
}

/// An effect list's signs together.
#[derive(Clone, Copy, Default)]
struct Reading {
    benefit: i32,
    hurts: bool,
    clock: bool,
}

impl Reading {
    fn absorb(&mut self, other: Self) {
        self.benefit += other.benefit;
        self.hurts |= other.hurts;
        self.clock |= other.clock;
    }
}

fn read(effects: &[Effect], x: u32) -> Reading {
    let mut result = Reading::default();
    for effect in effects {
        if acts_elsewhere(effect) {
            continue;
        }
        let mut reading = Reading::default();
        let (then, otherwise) = effect.branches();
        if !then.is_empty() || !otherwise.is_empty() {
            reading = read(then, x);
            reading.absorb(read(otherwise, x));
        }
        if let Some(benefit) = effect_benefit(effect, x) {
            reading.benefit = benefit;
        }
        reading.hurts |= hurts(effect, x);
        reading.clock |= matches!(
            effect,
            Effect::AddCounter {
                kind: CounterKind::Time | CounterKind::Lore,
                ..
            }
        );
        result.absorb(reading);
    }
    result
}

/// Whether `effects` turn or copy a spell, behind a "you may" too.
fn turns_a_spell(effects: &[Effect]) -> bool {
    effects.iter().any(|effect| {
        matches!(
            effect,
            Effect::ChangeTarget { .. }
                | Effect::ChooseNewTargets
                | Effect::CopyTargetSpell { .. }
                | Effect::CopyThisSpell { .. }
        ) || {
            let (then, otherwise) = effect.branches();
            turns_a_spell(then) || turns_a_spell(otherwise)
        }
    })
}

/// The side the selected `effects` of a target question want, X read as `x`.
#[must_use]
pub fn effects_side(effects: &[Effect], x: u32) -> Option<TargetSide> {
    if turns_a_spell(effects)
        || effects.iter().any(|effect| {
            matches!(
                effect,
                Effect::Fight { .. }
                    | Effect::DamageEqualToPower { .. }
                    | Effect::BottomCardFromHand { .. }
            )
        })
    {
        return None;
    }
    let reading = read(effects, x);
    if reading.clock {
        None
    } else if reading.hurts || reading.benefit < 0 {
        Some(TargetSide::Harmful)
    } else if reading.benefit > 0 {
        Some(TargetSide::Helpful)
    } else {
        None
    }
}

/// Whether a static ability's filter is the creature its Aura is attached to.
#[must_use]
pub fn enchants(filter: &Filter) -> bool {
    match filter {
        Filter::AttachedToBySource => true,
        Filter::And(parts) => parts.iter().any(enchants),
        _ => false,
    }
}

/// The side an Aura printing `abilities` wants its creature on, by what its
/// statics do to that creature. `None` for a static with no row here (a
/// control Aura, a mana Aura's trigger) and for one that nets out at nothing.
#[must_use]
pub fn aura_side(abilities: &[AbilityDef]) -> Option<TargetSide> {
    let mut sign = 0_i32;
    for ability in abilities {
        let AbilityDef::Static(s) = ability else {
            continue;
        };
        if !enchants(&s.filter) {
            continue;
        }
        sign += match s.modifier {
            Modifier::ModifyPT(power, toughness) => {
                (i32::from(power) + i32::from(toughness)).signum()
            }
            Modifier::AddKeyword(keywords) if keywords.contains(KeywordSet::DEFENDER) => -1,
            Modifier::AddKeyword(_) => 1,
            Modifier::CantAttack | Modifier::CantBlock(_) => -1,
            _ => return None,
        };
    }
    match sign.signum() {
        1 => Some(TargetSide::Helpful),
        -1 => Some(TargetSide::Harmful),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
