//! Proliferate is optional, but all counter kinds on a chosen recipient increase.
use crate::HeuristicAgent;
use baylee_cards_dsl::CounterKind;
use baylee_core::ids::PlayerId;
use baylee_view::{PlayerView, PublicObject};

fn engine_kind(kind: baylee_view::CounterKind) -> Option<CounterKind> {
    use baylee_view::CounterKind as V;
    Some(match kind {
        V::Plus { power, toughness } => CounterKind::Plus { power, toughness },
        V::Minus { power, toughness } => CounterKind::Minus { power, toughness },
        V::Loyalty => CounterKind::Loyalty,
        V::Lore => CounterKind::Lore,
        V::Time => CounterKind::Time,
        V::Charge => CounterKind::Charge,
        V::Poison => CounterKind::Poison,
        V::Energy => CounterKind::Energy,
        V::Rad => CounterKind::Rad,
        V::Lifelink => CounterKind::Lifelink,
        V::Level => CounterKind::Level,
        V::Flying => CounterKind::Flying,
        V::Shadow => CounterKind::Shadow,
        V::Indestructible => CounterKind::Indestructible,
        V::Hone => CounterKind::Hone,
        V::Stun => CounterKind::Stun,
        V::Finality => CounterKind::Finality,
        V::Custom(id) => CounterKind::Custom(u16::try_from(id).ok()?),
    })
}

impl HeuristicAgent {
    pub(crate) fn proliferate_object(&self, view: &PlayerView, object: &PublicObject) -> i64 {
        object
            .counters
            .iter()
            .filter(|c| c.count > 0)
            .map(|counter| self.counters(view, object, engine_kind(counter.kind)?, 1))
            .sum::<Option<i64>>()
            .unwrap_or(0)
    }

    pub(crate) fn proliferate_player(&self, view: &PlayerView, player: PlayerId) -> i64 {
        let Some(seat) = view.seat(player) else {
            return 0;
        };
        let poison = if seat.poison >= 9 {
            crate::worth::LETHAL
        } else {
            200
        };
        let value = 40 * i64::from(seat.energy > 0)
            - poison * i64::from(seat.poison > 0)
            - 50 * i64::from(seat.rad > 0);
        if self.hostile(player, view.seat) {
            -value
        } else {
            value
        }
    }
}
